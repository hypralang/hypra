//! Every rule has a page in docs/rules, named after its code, and its examples are real: a block marked
//! ```hypra fails breaks the rule and nothing else, or only the other rules listed after it, like
//! ```hypra fails SEO-004; a block marked ```hypra passes compiles without a single message. ```hypra file <path> adds a file the examples import, and ```json data the data the examples after it
//! are built with.

use hypra::diag::{RULES, Severity};
use serde_json::Value as Json;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/rules")
}

/// The code blocks of a page: the info string after the fence, and the code.
fn blocks(text: &str) -> Vec<(String, String)> {
    let mut out = vec![];
    let mut current: Option<(String, String)> = None;
    for line in text.lines() {
        match (&mut current, line.strip_prefix("```")) {
            (None, Some(info)) => current = Some((info.trim().to_string(), String::new())),
            (Some(_), Some("")) => out.extend(current.take()),
            (Some((_, code)), _) => {
                code.push_str(line);
                code.push('\n');
            }
            _ => {}
        }
    }
    out
}

/// The messages for one example, with the page's other files and data.
fn messages(example: &str, files: &[(String, String)], data: Option<&Json>) -> Vec<hypra::diag::Diagnostic> {
    let load = |path: &str| match path {
        "page.hypra" => Ok(example.to_string()),
        _ => files.iter().find(|(p, _)| p == path).map(|(_, code)| code.clone()).ok_or("not found".to_string()),
    };
    match data {
        Some(data) => hypra::compile_with("page.hypra", data, &load).diagnostics,
        None => hypra::check_with("page.hypra", &load).diagnostics,
    }
}

/// Rules whose example would be too big to show, like a page nested 257 levels deep.
const NO_EXAMPLE: &[&str] = &["PARSE-002"];

#[test]
fn every_rule_has_a_page_with_real_examples() {
    // pages nested to the limits recurse deeply; the CLI gives the compiler this much stack too
    std::thread::Builder::new().stack_size(256 << 20).spawn(check_every_page).unwrap().join().unwrap();
}

fn check_every_page() {
    let mut wrong = vec![];
    for (code, _) in RULES {
        let path = root().join(format!("{code}.md"));
        let Ok(text) = std::fs::read_to_string(&path) else {
            wrong.push(format!("{code}: no page at docs/rules/{code}.md"));
            continue;
        };
        if !text.starts_with(&format!("# {code}: ")) {
            wrong.push(format!("{code}: the page starts with `# {code}: ` and what the rule checks"));
        }
        let blocks = blocks(&text);
        let files: Vec<(String, String)> = blocks
            .iter()
            .filter_map(|(info, code)| Some((info.strip_prefix("hypra file ")?.to_string(), code.clone())))
            .collect();
        let mut data: Option<Json> = None;
        let (mut fails, mut passes) = (0, 0);
        for (info, example) in &blocks {
            // examples are shown as `hypra fmt` writes them, except the syntax errors, which show the mistake as typed
            if info.starts_with("hypra")
                && !code.starts_with("PARSE-")
                && !code.starts_with("LEX-")
                && let Ok(formatted) = hypra::fmt::format(example)
                && formatted != *example
            {
                wrong.push(format!("{code}: an example isn't formatted like `hypra fmt` writes it:\n{example}"));
            }
            let words: Vec<&str> = info.split_whitespace().collect();
            match words.as_slice() {
                ["json", "data"] => data = Some(serde_json::from_str(example).unwrap()),
                ["hypra", "fails", also @ ..] => {
                    fails += 1;
                    let found = messages(example, &files, data.as_ref());
                    let codes: Vec<&str> = found.iter().map(|d| d.code.as_str()).collect();
                    if !codes.contains(code) || codes.iter().any(|c| c != code && !also.contains(c)) {
                        wrong.push(format!("{code}: the failing example gives {codes:?}:\n{example}"));
                    }
                    // the page says whether the rule stops the build
                    let says =
                        if found.iter().any(|d| d.severity == Severity::Error) { "**Error**" } else { "**Warning**" };
                    if !text.contains(says) {
                        wrong.push(format!("{code}: the page doesn't say {says}"));
                    }
                }
                ["hypra", "passes"] => {
                    passes += 1;
                    let found = messages(example, &files, data.as_ref());
                    let codes: Vec<String> = found.into_iter().map(|d| d.code).collect();
                    if !codes.is_empty() {
                        wrong.push(format!("{code}: the passing example gives {codes:?}:\n{example}"));
                    }
                }
                _ => {}
            }
        }
        if fails == 0 && !NO_EXAMPLE.contains(code) {
            wrong.push(format!("{code}: no ```hypra fails example"));
        }
        if passes == 0 {
            wrong.push(format!("{code}: no ```hypra passes example"));
        }
    }
    // and no page for a rule that doesn't exist
    for entry in std::fs::read_dir(root()).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        let code = name.trim_end_matches(".md");
        if !RULES.iter().any(|(c, _)| *c == code) {
            wrong.push(format!("docs/rules/{name}: there's no rule {code}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n\n"));
}
