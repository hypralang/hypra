//! The guides in docs/guides show real Hypra: their example pages are files in docs/guides/examples, and every
//! code block marked as output (```html output, ```markdown output, ```json output, ```console output) is what
//! the compiler writes for them.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/guides")
}

/// The code blocks of a guide: the info string after the fence, and the code. A block indented under a list item
/// loses that indentation.
fn blocks(text: &str) -> Vec<(String, String)> {
    let mut out = vec![];
    let mut current: Option<(String, String, usize)> = None;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let fence = line.trim_start().strip_prefix("```");
        match (&mut current, fence) {
            (None, Some(info)) => current = Some((info.trim().to_string(), String::new(), indent)),
            (Some(_), Some("")) => out.extend(current.take().map(|(info, code, _)| (info, code))),
            (Some((_, code, indent)), _) => {
                code.push_str(line.get(*indent..).unwrap_or_default());
                code.push('\n');
            }
            _ => {}
        }
    }
    out
}

/// Every line trimmed, so an excerpt matches whatever it's indented by in the full output.
fn lines(text: &str) -> String {
    text.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
}

/// Everything the compiler writes for the examples, and what `hypra check` says about each, as one text per kind.
struct Outputs {
    html: String,
    markdown: String,
    json: String,
    console: String,
}

fn compile_examples() -> Outputs {
    let examples = root().join("examples");
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("guides");
    let _ = std::fs::remove_dir_all(&out);
    let mut outputs =
        Outputs { html: String::new(), markdown: String::new(), json: String::new(), console: String::new() };
    let mut pages: Vec<PathBuf> = std::fs::read_dir(&examples).unwrap().map(|e| e.unwrap().path()).collect();
    pages.retain(|p| p.extension().is_some_and(|e| e == "hypra"));
    pages.sort();
    for page in pages {
        let name = page.file_name().unwrap().to_str().unwrap();
        let check =
            Command::new(env!("CARGO_BIN_EXE_hypra")).arg("check").arg(name).current_dir(&examples).output().unwrap();
        outputs.console += &format!("$ hypra check {name}\n{}\n", String::from_utf8_lossy(&check.stderr));
        let data = page.with_extension("json");
        let mut build = Command::new(env!("CARGO_BIN_EXE_hypra"));
        build.arg("build").arg(name).arg("--out").arg(&out).current_dir(&examples);
        if data.exists() {
            build.arg("--data").arg(&data);
        }
        if !build.output().unwrap().status.success() {
            continue;
        }
        let stem = Path::new(name).with_extension("");
        let read = |ext: &str| std::fs::read_to_string(out.join(format!("{}.{ext}", stem.display()))).unwrap();
        outputs.html += &read("html");
        outputs.markdown += &read("md");
        outputs.json += &read("webmcp.json");
    }
    outputs
}

fn guides() -> Vec<(String, String)> {
    let mut guides: Vec<(String, String)> = std::fs::read_dir(root())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), std::fs::read_to_string(&p).unwrap()))
        .collect();
    guides.sort();
    guides
}

#[test]
fn shown_output_is_real() {
    let outputs = compile_examples();
    for (guide, text) in guides() {
        for (info, code) in blocks(&text) {
            let Some(kind) = info.strip_suffix(" output") else { continue };
            let real = match kind {
                "html" => &outputs.html,
                "markdown" => &outputs.markdown,
                "json" => &outputs.json,
                "console" => &outputs.console,
                other => panic!("{guide}: no output of kind `{other}`"),
            };
            assert!(lines(real).contains(&lines(&code)), "{guide}: this isn't what the compiler writes:\n{code}");
        }
    }
}

#[test]
fn every_example_is_shown_and_every_page_compiles() {
    let guides = guides();
    let shown: Vec<String> = guides
        .iter()
        .flat_map(|(_, text)| blocks(text))
        .filter(|(info, _)| info == "hypra")
        .map(|(_, code)| code)
        .collect();
    let examples = root().join("examples");
    for entry in std::fs::read_dir(&examples).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "hypra") {
            let source = std::fs::read_to_string(&path).unwrap();
            assert!(shown.contains(&source), "{} isn't shown in a guide as it is", path.display());
        }
    }
    // pages written only in a guide, like a fixed version of an example, compile without a warning
    let example_sources: Vec<String> = std::fs::read_dir(&examples)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "hypra"))
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect();
    for code in shown.iter().filter(|c| c.contains("page(") && !example_sources.contains(c)) {
        let result = hypra::check_with("page.hypra", &|_| Ok(code.clone()));
        assert!(result.diagnostics.is_empty(), "{:?}\n{code}", result.diagnostics);
    }
}
