use hypra::diag::Diagnostic;
use hypra::{Generated, Source, check_file, compile_file, js_file, php_file};
use serde_json::{Value as Json, json};
use std::{env, fs, path::Path, path::PathBuf, process::ExitCode};

const USAGE: &str = "usage:
  hypra build <file.hypra> [--data data.json] [--out dir]   html, md, json and webmcp.json from data
  hypra build <file.hypra> --target php [--out dir]         a PHP page for the hypra/runtime package
  hypra build <file.hypra> --target js [--out dir]          a JavaScript module for the hypra npm package
  hypra check <file.hypra> [--data data.json] [--format json]
                                                            with data, also checks the data
  hypra fmt [--check] <file.hypra>...                       formats files; --check lists those that need it
  hypra fmt -                                               formats stdin to stdout, for editors
  hypra lsp                                                 a language server for editors, over stdin and stdout
  hypra --version";

#[derive(PartialEq)]
enum Target {
    Static,
    Php,
    Js,
}

fn main() -> ExitCode {
    // Pages nest components inside components; give the compiler more room than the default stack.
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(run)
        .expect("start the compiler thread")
        .join()
        .unwrap_or(ExitCode::FAILURE)
}

fn run() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some((cmd, rest)) = args.split_first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match cmd.as_str() {
        "--version" | "-V" => {
            println!("hypra {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        "fmt" => return format_files(rest),
        "lsp" if rest.is_empty() => return ExitCode::from(hypra::lsp::run() as u8),
        _ => {}
    }

    let Options { file, data_path, out_dir, json_format, target } = match parse_options(cmd.as_str(), rest) {
        Ok(options) => options,
        Err(code) => return code,
    };

    let data: Json = match &data_path {
        Some(p) => match fs::read_to_string(p)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        {
            Ok(d) => d,
            Err(e) => {
                eprintln!("can't load data from {p}: {e}");
                return ExitCode::from(2);
            }
        },
        None => json!({}),
    };

    let stem = Path::new(&file).file_stem().and_then(|s| s.to_str()).unwrap_or("page").to_string();
    let (diagnostics, sources, files): Built = match target {
        // without data there's nothing to render, so check everything else
        Target::Static if cmd == "check" && data_path.is_none() => {
            let result = check_file(&file);
            (result.diagnostics, result.sources, None)
        }
        Target::Static => {
            let result = compile_file(&file, &data);
            let files = result.output.map(|out| static_files(&stem, &out));
            (result.diagnostics, result.sources, files)
        }
        Target::Php => generated(php_file(&file), &stem, "php"),
        Target::Js => {
            // .mjs is a module in every project, with or without "type": "module"; .d.mts types it for TypeScript
            let (diagnostics, sources, mut files) = generated(js_file(&file), &stem, "mjs");
            let types =
                "import type { Page } from \"hypra/runtime\";\n\ndeclare const page: Page;\nexport default page;\n";
            files.iter_mut().for_each(|files| files.insert(1, (format!("{stem}.d.mts"), types.to_string())));
            (diagnostics, sources, files)
        }
    };

    report(&diagnostics, &sources, &file, json_format);

    if cmd == "build"
        && let Some(files) = files
        && let Err(e) = write_files(&out_dir, files)
    {
        eprintln!("can't write output: {e}");
        return ExitCode::from(2);
    }

    let errors = diagnostics.iter().filter(|d| d.is_error()).count();
    if !json_format {
        eprintln!("{}, {}", plural(errors, "error"), plural(diagnostics.len() - errors, "warning"));
    }
    if errors > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS }
}

/// What a build found, the files it read, and the files to write as (name, contents) when it found no error.
type Built = (Vec<Diagnostic>, Vec<Source>, Option<Vec<(String, String)>>);

/// A page compiled to code: the code, and the files it was built from, so a server knows when to rebuild it.
fn generated(result: Generated, stem: &str, ext: &str) -> Built {
    let paths: Vec<&str> = result.sources.iter().map(|s| s.path.as_str()).collect();
    let manifest = format!("{}\n", serde_json::to_string(&paths).expect("paths are strings"));
    let files =
        result.code.map(|code| vec![(format!("{stem}.{ext}"), code), (format!("{stem}.sources.json"), manifest)]);
    (result.diagnostics, result.sources, files)
}

/// What `build` and `check` were asked to do.
struct Options {
    file: String,
    data_path: Option<String>,
    out_dir: PathBuf,
    json_format: bool,
    target: Target,
}

/// Reads the options of `build` and `check`; on a mistake, says what's wrong and gives the exit code.
fn parse_options(cmd: &str, rest: &[String]) -> Result<Options, ExitCode> {
    let mut file: Option<String> = None;
    let mut data_path: Option<String> = None;
    let mut out_dir = PathBuf::from("dist");
    let mut json_format = false;
    let mut target = Target::Static;
    let mut it = rest.iter();
    while let Some(arg) = it.next() {
        let option = arg.as_str();
        if matches!(option, "--data" | "--out" | "--format" | "--target") {
            let Some(value) = it.next().filter(|v| !v.starts_with("--")) else {
                eprintln!("{option} needs a value\n{USAGE}");
                return Err(ExitCode::from(2));
            };
            let unknown = |expected: &str| {
                eprintln!("unknown {} `{value}`, expected {expected}", &option[2..]);
                ExitCode::from(2)
            };
            match (option, value.as_str()) {
                ("--data", _) => data_path = Some(value.clone()),
                ("--out", _) => out_dir = PathBuf::from(value),
                ("--format", "text") => json_format = false,
                ("--format", "json") => json_format = true,
                ("--format", _) => return Err(unknown("`text` or `json`")),
                ("--target", "static") => target = Target::Static,
                ("--target", "php") => target = Target::Php,
                ("--target", "js") => target = Target::Js,
                _ => return Err(unknown("`static`, `php` or `js`")),
            }
        } else if file.is_none() && !option.starts_with("--") {
            file = Some(arg.clone());
        } else {
            eprintln!("unknown argument `{option}`\n{USAGE}");
            return Err(ExitCode::from(2));
        }
    }
    let (Some(file), true) = (file, matches!(cmd, "build" | "check")) else {
        eprintln!("{USAGE}");
        return Err(ExitCode::from(2));
    };
    if target != Target::Static && data_path.is_some() {
        eprintln!("--data doesn't apply to --target php or js: the page receives its data when it renders");
        return Err(ExitCode::from(2));
    }
    Ok(Options { file, data_path, out_dir, json_format, target })
}

/// Formats files in place, or with `--check` lists the ones that aren't formatted and changes nothing.
fn format_files(args: &[String]) -> ExitCode {
    if args == ["-"] {
        let mut src = String::new();
        if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut src) {
            eprintln!("can't read stdin: {e}");
            return ExitCode::from(2);
        }
        return match hypra::fmt::format(&src) {
            Ok(formatted) => {
                print!("{formatted}");
                ExitCode::SUCCESS
            }
            Err(d) => {
                eprint!("{}", d.render("<stdin>", &src));
                ExitCode::FAILURE
            }
        };
    }
    let check = args.iter().any(|a| a == "--check");
    let files: Vec<&String> = args.iter().filter(|a| *a != "--check").collect();
    if files.is_empty() || files.iter().any(|f| f.starts_with("--")) {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let mut failed = false;
    for file in files {
        let src = match fs::read_to_string(file) {
            Ok(src) => src,
            Err(e) => {
                eprintln!("can't read {file}: {e}");
                return ExitCode::from(2);
            }
        };
        let formatted = match hypra::fmt::format(&src) {
            Ok(formatted) => formatted,
            Err(d) => {
                eprint!("{}", d.render(file, &src));
                failed = true;
                continue;
            }
        };
        if formatted == src {
            continue;
        }
        if check {
            println!("{file}");
            failed = true;
        } else if let Err(e) = fs::write(file, formatted) {
            eprintln!("can't write {file}: {e}");
            return ExitCode::from(2);
        }
    }
    if failed { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}

/// Prints diagnostics, each pointing into the file it came from (the page or one of its imports).
fn report(diagnostics: &[Diagnostic], sources: &[Source], entry: &str, json_format: bool) {
    let source_of =
        |d: &Diagnostic| sources.get(d.pos.file).map(|s| (s.path.as_str(), s.text.as_str())).unwrap_or((entry, ""));
    if json_format {
        let list: Vec<Json> = diagnostics.iter().map(|d| d.to_json(source_of(d).0)).collect();
        println!("{}", serde_json::to_string_pretty(&list).unwrap_or_default());
    } else {
        for d in diagnostics {
            let (path, text) = source_of(d);
            eprintln!("{}", d.render(path, text));
        }
    }
}

fn static_files(stem: &str, out: &hypra::interp::Output) -> Vec<(String, String)> {
    let pretty = |j: &Json| serde_json::to_string_pretty(j).unwrap_or_default() + "\n";
    vec![
        (format!("{stem}.html"), out.html.clone()),
        (format!("{stem}.md"), out.markdown.clone()),
        (format!("{stem}.json"), pretty(&out.json)),
        (format!("{stem}.webmcp.json"), pretty(&out.webmcp)),
    ]
}

fn write_files(dir: &Path, files: Vec<(String, String)>) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    for (name, content) in files {
        let path = dir.join(name);
        fs::write(&path, content)?;
        eprintln!("✔ {}", path.display());
    }
    Ok(())
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 { format!("1 {word}") } else { format!("{n} {word}s") }
}
