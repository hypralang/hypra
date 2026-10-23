use hypra::diag::Diagnostic;
use hypra::{Generated, Source, check, check_file, compile_file, js_file, lexer, parser, php_file};
use serde_json::{Value as Json, json};
use std::{env, fs, path::Path, path::PathBuf, process::ExitCode};

const USAGE: &str = "usage:
  hypra build <file.hypra> [--data data.json] [--out dir]   html, md, json and webmcp.json from data
  hypra build <path>... --target php [--out dir]            PHP pages for the hypra/runtime package
  hypra build <path>... --target js [--out dir]             JavaScript modules for the hypra npm package
  hypra check <path>... [--format json]                     checks files, or every .hypra file in a folder
  hypra check <file.hypra> --data data.json                 also checks the data
  hypra fmt [--check] <path>...                             formats files; --check lists those that need it
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

    let Options { paths, data_path, out_dir, json_format, target } = match parse_options(cmd.as_str(), rest) {
        Ok(options) => options,
        Err(code) => return code,
    };
    let inputs = match inputs(&paths, cmd == "build") {
        Ok(inputs) => inputs,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    // two pages with the same name from different folders would overwrite each other
    let mut names = std::collections::HashSet::new();
    if cmd == "build"
        && let Some(twice) = inputs
            .iter()
            .map(|i| i.out.join(Path::new(&i.file).file_stem().unwrap_or_default()))
            .find(|n| !names.insert(n.clone()))
    {
        eprintln!("two pages build to {}; build them into different --out folders", out_dir.join(twice).display());
        return ExitCode::from(2);
    }
    if data_path.is_some() && (inputs.len() != 1 || Path::new(&paths[0]).is_dir()) {
        eprintln!("--data is the data of one page, so give one file");
        return ExitCode::from(2);
    }

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

    let mut all: Vec<Json> = vec![];
    let (mut errors, mut warnings) = (0, 0);
    for Input { file, out } in &inputs {
        let stem = Path::new(file).file_stem().and_then(|s| s.to_str()).unwrap_or("page").to_string();
        let (diagnostics, sources, files): Built = match target {
            // without data there's nothing to render, so check everything else
            Target::Static if cmd == "check" && data_path.is_none() => {
                let result = check_file(file);
                (result.diagnostics, result.sources, None)
            }
            Target::Static => {
                let result = compile_file(file, &data);
                let files = result.output.map(|out| static_files(&stem, &out));
                (result.diagnostics, result.sources, files)
            }
            Target::Php => generated(php_file(file), &stem, "php"),
            Target::Js => {
                // .mjs is a module in every project, with or without "type": "module"; .d.mts types it for TypeScript
                let (diagnostics, sources, mut files) = generated(js_file(file), &stem, "mjs");
                let types =
                    "import type { Page } from \"hypra/runtime\";\n\ndeclare const page: Page;\nexport default page;\n";
                files.iter_mut().for_each(|files| files.insert(1, (format!("{stem}.d.mts"), types.to_string())));
                (diagnostics, sources, files)
            }
        };

        report(&diagnostics, &sources, file, json_format, &mut all);
        let found = diagnostics.iter().filter(|d| d.is_error()).count();
        errors += found;
        warnings += diagnostics.len() - found;

        if cmd == "build"
            && let Some(files) = files
        {
            let dir = out_dir.join(out);
            if let Err(e) = write_files(&dir, files) {
                eprintln!("can't write output: {e}");
                return ExitCode::from(2);
            }
        }
    }

    if json_format {
        println!("{}", serde_json::to_string_pretty(&all).unwrap_or_default());
    } else {
        let files = if inputs.len() > 1 { format!("{}, ", plural(inputs.len(), "file")) } else { String::new() };
        eprintln!("{files}{}, {}", plural(errors, "error"), plural(warnings, "warning"));
    }
    if errors > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS }
}

/// A file to check or build, and the folder under `--out` its files go to.
struct Input {
    file: String,
    out: PathBuf,
}

/// The files the paths name: a file as it is, a folder as every `.hypra` file in it and its subfolders, which
/// keep their place under `--out`. A build leaves out files that hold only components and layouts, since the
/// pages that import them build them, so a folder of them builds nothing.
fn inputs(paths: &[String], build: bool) -> Result<Vec<Input>, String> {
    let mut inputs = vec![];
    for path in paths {
        let root = Path::new(path);
        if !root.is_dir() {
            inputs.push(Input { file: path.clone(), out: PathBuf::new() });
            continue;
        }
        for file in folder(path)? {
            if build && fs::read_to_string(&file).is_ok_and(|src| is_library(&src)) {
                continue;
            }
            let out = file.parent().and_then(|p| p.strip_prefix(root).ok()).unwrap_or(Path::new("")).to_path_buf();
            inputs.push(Input { file: file.to_string_lossy().into_owned(), out });
        }
    }
    if inputs.is_empty() {
        return Err(format!(
            "no pages to build in {}: files with only components and layouts are built by the pages that use them",
            paths.join(", ")
        ));
    }
    Ok(inputs)
}

/// The `.hypra` files in a folder and its subfolders, in order; a folder without any is a mistake.
fn folder(path: &str) -> Result<Vec<PathBuf>, String> {
    let mut found = vec![];
    hypra_files(Path::new(path), &mut found).map_err(|e| format!("can't read {path}: {e}"))?;
    if found.is_empty() {
        return Err(format!("{path} has no .hypra files"));
    }
    found.sort();
    Ok(found)
}

/// Every `.hypra` file under `dir`, leaving out hidden folders and installed packages.
fn hypra_files(dir: &Path, found: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if !name.starts_with('.') && !matches!(name, "node_modules" | "vendor") {
                hypra_files(&path, found)?;
            }
        } else if path.extension().is_some_and(|e| e == "hypra") {
            found.push(path);
        }
    }
    Ok(())
}

/// Whether a file holds only components and layouts.
fn is_library(src: &str) -> bool {
    lexer::lex(src, 0).and_then(parser::parse).is_ok_and(|items| check::is_library(&items))
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
    paths: Vec<String>,
    data_path: Option<String>,
    out_dir: PathBuf,
    json_format: bool,
    target: Target,
}

/// Reads the options of `build` and `check`; on a mistake, says what's wrong and gives the exit code.
fn parse_options(cmd: &str, rest: &[String]) -> Result<Options, ExitCode> {
    let mut paths: Vec<String> = vec![];
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
        } else if !option.starts_with("--") {
            paths.push(arg.clone());
        } else {
            eprintln!("unknown argument `{option}`\n{USAGE}");
            return Err(ExitCode::from(2));
        }
    }
    if paths.is_empty() || !matches!(cmd, "build" | "check") {
        eprintln!("{USAGE}");
        return Err(ExitCode::from(2));
    }
    if target != Target::Static && data_path.is_some() {
        eprintln!("--data doesn't apply to --target php or js: the page receives its data when it renders");
        return Err(ExitCode::from(2));
    }
    Ok(Options { paths, data_path, out_dir, json_format, target })
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
    let paths: Vec<&String> = args.iter().filter(|a| *a != "--check").collect();
    if paths.is_empty() || paths.iter().any(|f| f.starts_with("--")) {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let mut files = vec![];
    for path in paths {
        if !Path::new(path).is_dir() {
            files.push(path.clone());
            continue;
        }
        match folder(path) {
            Ok(found) => files.extend(found.iter().map(|f| f.to_string_lossy().into_owned())),
            Err(message) => {
                eprintln!("{message}");
                return ExitCode::from(2);
            }
        }
    }
    let mut failed = false;
    for file in &files {
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
/// With `--format json`, adds them to `all`, which is printed at the end.
fn report(diagnostics: &[Diagnostic], sources: &[Source], entry: &str, json_format: bool, all: &mut Vec<Json>) {
    let source_of =
        |d: &Diagnostic| sources.get(d.pos.file).map(|s| (s.path.as_str(), s.text.as_str())).unwrap_or((entry, ""));
    if json_format {
        all.extend(diagnostics.iter().map(|d| d.to_json(source_of(d).0)));
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
