//! The compiler for the playground on hypralang.dev. The page writes a request as JSON into `input`, calls
//! `compile` or `format`, and reads the answer, also JSON, from `output`. The module keeps one buffer for each.

use hypra::Source;
use hypra::diag::Diagnostic;
use serde_json::{Value as Json, json};
use std::cell::RefCell;

thread_local! {
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Makes room for a request of `len` bytes and returns where to write it.
#[unsafe(no_mangle)]
pub extern "C" fn input(len: usize) -> *mut u8 {
    INPUT.with_borrow_mut(|buf| {
        buf.clear();
        buf.resize(len, 0);
        buf.as_mut_ptr()
    })
}

/// Where the last answer is; `compile` and `format` return its length.
#[unsafe(no_mangle)]
pub extern "C" fn output() -> *const u8 {
    OUTPUT.with_borrow(|buf| buf.as_ptr())
}

/// `{"files": {"page.hypra": "...", ...}, "data": {...}}` → the outputs of the first file, and every message.
#[unsafe(no_mangle)]
pub extern "C" fn compile() -> usize {
    answer(|request| {
        let files = request["files"].as_object().cloned().unwrap_or_default();
        let entry = files.keys().next().cloned().unwrap_or_else(|| "page.hypra".into());
        let load = |path: &str| match files.get(path).and_then(Json::as_str) {
            Some(text) => Ok(text.to_string()),
            None => Err("there's no such file in the playground".into()),
        };
        let compiled = hypra::compile_with(&entry, &request["data"], &load);
        let pretty = |j: &Json| serde_json::to_string_pretty(j).unwrap_or_default() + "\n";
        let mut out = json!({ "ok": compiled.output.is_some(), "diagnostics": messages(&compiled.diagnostics, &compiled.sources) });
        if let Some(output) = &compiled.output {
            out["html"] = output.html.clone().into();
            out["markdown"] = output.markdown.clone().into();
            out["json"] = pretty(&output.json).into();
            out["tools"] = pretty(&output.webmcp).into();
        }
        out
    })
}

/// `{"source": "..."}` → the source as `hypra fmt` writes it, or why it can't.
#[unsafe(no_mangle)]
pub extern "C" fn format() -> usize {
    answer(|request| {
        let source = request["source"].as_str().unwrap_or_default();
        match hypra::fmt::format(source) {
            Ok(formatted) => json!({ "ok": true, "source": formatted }),
            Err(d) => {
                let sources = [Source { path: "page.hypra".into(), text: source.into() }];
                json!({ "ok": false, "diagnostics": messages(&[d], &sources) })
            }
        }
    })
}

/// Each message as `hypra check --format json` writes it, with the text the terminal shows next to it.
fn messages(diagnostics: &[Diagnostic], sources: &[Source]) -> Json {
    let list = diagnostics.iter().map(|d| {
        let source = sources.get(d.pos.file);
        let (file, text) = source.map(|s| (s.path.as_str(), s.text.as_str())).unwrap_or(("page.hypra", ""));
        let mut message = d.to_json(file);
        message["text"] = d.render(file, text).into();
        message
    });
    list.collect()
}

/// Runs `f` on the request in `input` and leaves its answer in `output`.
fn answer(f: impl FnOnce(&Json) -> Json) -> usize {
    let request = INPUT.with_borrow(|buf| serde_json::from_slice(buf).unwrap_or(Json::Null));
    let bytes = serde_json::to_vec(&f(&request)).unwrap_or_default();
    OUTPUT.with_borrow_mut(|buf| {
        *buf = bytes;
        buf.len()
    })
}
