//! Hypra: a strict markup language that compiles to HTML, Markdown, JSON and WebMCP tools.

pub mod check;
pub mod defs;
pub mod diag;
pub mod editor;
pub mod elements;
pub mod fmt;
pub mod help;
pub mod html;
pub mod interp;
pub mod ir;
pub mod lexer;
pub mod lower;
pub mod lsp;
pub mod nav;
pub mod parser;
pub mod php;
pub mod rules;
pub mod words;

use diag::Diagnostic;
use parser::{Item, Value};
use serde_json::Value as Json;
use std::path::{Component, Path, PathBuf};

/// A file that took part in a compilation. A diagnostic's `pos.file` indexes into these.
pub struct Source {
    pub path: String,
    pub text: String,
}

pub struct Compiled {
    /// `None` when any error was found: a broken page never produces output.
    pub output: Option<interp::Output>,
    pub diagnostics: Vec<Diagnostic>,
    pub sources: Vec<Source>,
}

/// Compiles a single page from memory. `use` imports are reported as unreadable.
/// `stem` is the output file name without extension, used for cross-links.
pub fn compile(src: &str, data: &Json, stem: &str) -> Compiled {
    let entry = format!("{stem}.hypra");
    compile_with(&entry, data, &|path| if path == entry { Ok(src.to_string()) } else { Err("no file loader".into()) })
}

/// Compiles a page from disk, following its `use` imports.
pub fn compile_file(path: &str, data: &Json) -> Compiled {
    compile_with(path, data, &read_disk)
}

/// Loads a source file from disk.
fn read_disk(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

/// Compiles the page at `entry`, loading it and its imports through `load`.
/// Import paths are relative to the importing file.
pub fn compile_with(entry: &str, data: &Json, load: &dyn Fn(&str) -> Result<String, String>) -> Compiled {
    let Frontend { program, mut diagnostics, sources } = frontend(entry, load, true);
    let Some(program) = program else { return Compiled { output: None, diagnostics: in_order(diagnostics), sources } };
    let mut output = interp::run(&program, data);
    diagnostics.append(&mut output.diagnostics);
    let ok = !diagnostics.iter().any(Diagnostic::is_error);
    Compiled { output: ok.then_some(output), diagnostics: in_order(diagnostics), sources }
}

pub struct Generated {
    /// `None` when any error was found.
    pub code: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub sources: Vec<Source>,
}

/// Checks a page from disk without data: every check except those that need the data itself.
pub fn check_file(path: &str) -> Compiled {
    check_with(path, &read_disk)
}

/// Checks the page at `entry` without data, loading it and its imports through `load`.
pub fn check_with(entry: &str, load: &dyn Fn(&str) -> Result<String, String>) -> Compiled {
    let Frontend { diagnostics, sources, .. } = frontend(entry, load, false);
    Compiled { output: None, diagnostics: in_order(diagnostics), sources }
}

/// Compiles a page from disk to a PHP file for the `hypra/runtime` package.
pub fn php_file(path: &str) -> Generated {
    php_with(path, &read_disk)
}

/// Compiles the page at `entry` to PHP, loading it and its imports through `load`.
pub fn php_with(entry: &str, load: &dyn Fn(&str) -> Result<String, String>) -> Generated {
    let Frontend { program, diagnostics, sources } = frontend(entry, load, true);
    let paths: Vec<&str> = sources.iter().map(|s| s.path.as_str()).collect();
    let code = program.map(|p| php::emit(&p, &paths));
    Generated { code, diagnostics: in_order(diagnostics), sources }
}

/// The shared front end: load, parse, check and lower a page and its imports.
struct Frontend {
    program: Option<ir::Program>,
    diagnostics: Vec<Diagnostic>,
    sources: Vec<Source>,
}

/// `build` is false for a check, which also accepts a file of components and layouts without a page.
fn frontend(entry: &str, load: &dyn Fn(&str) -> Result<String, String>, build: bool) -> Frontend {
    let stem = Path::new(entry).file_stem().and_then(|s| s.to_str()).unwrap_or("page").to_string();
    let failed = |diagnostics, sources| Frontend { program: None, diagnostics, sources };
    let Project { modules, sources, mut diagnostics } = load_project(entry, load);
    if diagnostics.iter().any(Diagnostic::is_error) {
        return failed(diagnostics, sources);
    }

    let (defs, defs_diagnostics) = defs::collect(&modules);
    diagnostics.extend(defs_diagnostics);
    // components in imported files are checked in the language of the page that uses them
    let lang = check::page_language(&modules[0], &defs);
    for (i, items) in modules.iter().enumerate() {
        diagnostics.extend(check::check(items, &defs, i == 0, lang.as_deref()));
    }
    if diagnostics.iter().any(Diagnostic::is_error) {
        return failed(diagnostics, sources);
    }

    if check::is_library(&modules[0]) {
        if build {
            let d = Diagnostic::error("SEM-002", format!("`{stem}.hypra` has no page to build"), Default::default(), 1)
                .help("it holds components and layouts: build a page that imports it with `use`");
            diagnostics.push(d);
        }
        return failed(diagnostics, sources);
    }

    match lower::lower(&modules[0], &defs, &stem) {
        Ok(program) => Frontend { program: Some(program), diagnostics, sources },
        Err(errors) => {
            diagnostics.extend(errors);
            failed(diagnostics, sources)
        }
    }
}

/// A page and the files it imports, parsed. A file that doesn't parse has no items and a diagnostic.
pub(crate) struct Project {
    pub(crate) modules: Vec<Vec<Item>>,
    pub(crate) sources: Vec<Source>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

pub(crate) fn load_project(entry: &str, load: &dyn Fn(&str) -> Result<String, String>) -> Project {
    let entry = normalize(Path::new(entry)).to_string_lossy().into_owned();
    let mut diagnostics = vec![];
    let mut sources = vec![];

    match load(&entry) {
        Ok(text) => sources.push(Source { path: entry, text }),
        Err(e) => {
            let d = Diagnostic::error("IMPORT-001", format!("can't read `{entry}`: {e}"), Default::default(), 1);
            return Project { modules: vec![], sources, diagnostics: vec![d] };
        }
    }

    // Load files breadth-first; `sources` grows as imports are discovered.
    let mut modules: Vec<Vec<Item>> = vec![];
    let mut i = 0;
    while i < sources.len() {
        let items = match lexer::lex(&sources[i].text, i).and_then(parser::parse) {
            Ok(items) => items,
            Err(d) => {
                diagnostics.push(d);
                vec![]
            }
        };
        let dir = Path::new(&sources[i].path).parent().map(Path::to_path_buf).unwrap_or_default();
        for import in imports(&items) {
            let Some(rel) = import.as_str() else { continue };
            let path = normalize(&dir.join(rel)).to_string_lossy().into_owned();
            if sources.iter().any(|s| s.path == path) {
                continue;
            }
            match load(&path) {
                Ok(text) => sources.push(Source { path, text }),
                Err(e) => diagnostics.push(Diagnostic::error(
                    "IMPORT-001",
                    format!("can't read `{rel}`: {e}"),
                    import.pos,
                    import.len,
                )),
            }
        }
        modules.push(items);
        i += 1;
    }
    Project { modules, sources, diagnostics }
}

/// Line `line` of a text, counted from 1; empty past its end.
pub(crate) fn line_of(text: &str, line: usize) -> &str {
    line.checked_sub(1).and_then(|i| text.lines().nth(i)).unwrap_or_default()
}

/// Diagnostics in the order of the source, file by file.
fn in_order(mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics.sort_by_key(|d| (d.pos.file, d.pos.line, d.pos.col));
    diagnostics
}

/// The path values of top-level `use "..."` statements.
fn imports(items: &[Item]) -> impl Iterator<Item = &Value> {
    items.iter().filter_map(|i| match i {
        Item::Use(path) => Some(path),
        _ => None,
    })
}

/// Resolves `.` and `..` without touching the file system, so in-memory loaders work too.
pub(crate) fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir if matches!(out.components().next_back(), Some(Component::Normal(_))) => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}
