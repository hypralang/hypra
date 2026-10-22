//! The examples in the docs are real Hypra: every `hypra` code block parses, and every block that is a whole
//! page compiles without warnings.

use hypra::{lexer, parser};

fn blocks(file: &str) -> Vec<(usize, String)> {
    let text = std::fs::read_to_string(format!("{}/{file}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let mut out = vec![];
    let mut current: Option<(usize, String)> = None;
    for (i, line) in text.lines().enumerate() {
        match (&mut current, line.trim_start()) {
            (None, "```hypra") => current = Some((i + 2, String::new())),
            (Some(_), "```") => out.extend(current.take()),
            (Some((_, code)), _) => {
                code.push_str(line);
                code.push('\n');
            }
            _ => {}
        }
    }
    out
}

#[test]
fn every_example_parses() {
    for file in ["README.md", "docs/spec.md"] {
        for (line, code) in blocks(file) {
            // `{ ... }` and a line of `...` stand for content left out
            let code = code.replace("{ ... }", "{ }").replace("(...)", "()");
            let code: String = code.lines().filter(|l| l.trim() != "...").map(|l| format!("{l}\n")).collect();
            let parsed = lexer::lex(&code, 0).and_then(parser::parse);
            assert!(parsed.is_ok(), "{file}:{line}: {:?}\n{code}", parsed.err());
        }
    }
}

/// The file a block says it is, from a first line like `// layouts/shop.hypra`.
fn file_name(code: &str) -> Option<&str> {
    let first = code.lines().next()?.strip_prefix("// ")?;
    first.split([' ', ':']).next().filter(|name| name.ends_with(".hypra"))
}

#[test]
fn whole_page_examples_compile_cleanly() {
    for file in ["README.md", "docs/spec.md"] {
        let blocks = blocks(file);
        // blocks that name their file can be imported by the others
        let files: Vec<(&str, &str)> =
            blocks.iter().filter_map(|(_, code)| Some((file_name(code)?, code.as_str()))).collect();
        for (line, code) in &blocks {
            // `...` stands for content left out, so the block isn't a whole page
            if !code.contains("page(") || code.contains("...") {
                continue;
            }
            // a block that imports a file the docs don't show can't be compiled alone
            let imports = code.lines().filter_map(|l| l.trim().strip_prefix("use \"")?.split('"').next());
            if !imports.into_iter().all(|path| files.iter().any(|(name, _)| path.ends_with(name))) {
                continue;
            }
            let load = |path: &str| match files.iter().find(|(name, _)| path.ends_with(name)) {
                _ if path == "page.hypra" => Ok(code.clone()),
                Some((_, imported)) => Ok(imported.to_string()),
                None => Err(format!("{path} isn't one of the examples")),
            };
            let result = hypra::check_with("page.hypra", &load);
            assert!(result.diagnostics.is_empty(), "{file}:{line}: {:?}\n{code}", result.diagnostics);
        }
    }
}
