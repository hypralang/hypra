//! `hypra lsp`: a language server for editors, over stdin and stdout. It checks open pages as they're typed, finds
//! definitions and references, renames, shows help on hover and formats documents.
//!
//! Positions: the compiler counts lines and characters from 1; the protocol counts lines from 0 and characters in
//! UTF-16 code units, so a line's text is needed to convert between them.

use crate::diag::Diagnostic;
use crate::{Source, check_with, editor, fmt, help};
use serde_json::{Value as Json, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};

/// Runs the server until the editor says `exit`. Returns the process's exit code.
pub fn run() -> i32 {
    let mut server = Server::default();
    let stdin = io::stdin();
    let mut input = stdin.lock();
    while let Some(frame) = read_message(&mut input) {
        let Some(Ok(message)) = frame.map(|body| serde_json::from_slice::<Json>(&body)) else {
            write_message(
                &json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "not a message" } }),
            );
            continue;
        };
        let method = message["method"].as_str().unwrap_or_default();
        if method == "exit" {
            return if server.shut_down { 0 } else { 1 };
        }
        // a page that crashes the compiler must not take the server down with it
        let handled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| server.handle(method, &message)));
        if let Some(id) = message.get("id").filter(|_| message.get("method").is_some()) {
            let response = match handled {
                Ok(Ok(result)) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                Ok(Err((code, text))) => {
                    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": text } })
                }
                Err(_) => {
                    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32603, "message": "the compiler crashed" } })
                }
            };
            write_message(&response);
        }
    }
    1
}

/// Messages larger than this are skipped: no editor sends them, and reading one would exhaust memory.
const MAX_MESSAGE: usize = 64 << 20;

/// Reads one message: `Content-Length` headers, a blank line, then the body. `Some(None)` for a message that can't
/// be read, which is skipped; `None` when the editor is gone.
fn read_message(input: &mut impl BufRead) -> Option<Option<Vec<u8>>> {
    let mut length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().ok();
        }
    }
    let Some(length) = length else { return Some(None) };
    if length > MAX_MESSAGE {
        io::copy(&mut input.by_ref().take(length as u64), &mut io::sink()).ok()?;
        return Some(None);
    }
    let mut body = vec![0; length];
    input.read_exact(&mut body).ok()?;
    Some(Some(body))
}

fn write_message(message: &Json) {
    let body = message.to_string();
    let mut out = io::stdout().lock();
    // if the editor is gone, the next read ends the server
    let _ = write!(out, "Content-Length: {}\r\n\r\n{body}", body.len());
    let _ = out.flush();
}

fn notify(method: &str, params: Json) {
    write_message(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
}

type Reply = Result<Json, (i32, String)>;

#[derive(Default)]
struct Server {
    /// Open documents by path, as the editor has them, which may be unsaved.
    open: HashMap<String, String>,
    /// Files that were last sent diagnostics, so they can be cleared when their problems are gone.
    reported: HashSet<String>,
    /// The folders the editor has open, where references are looked for.
    roots: Vec<PathBuf>,
    shut_down: bool,
}

impl Server {
    fn handle(&mut self, method: &str, message: &Json) -> Reply {
        let params = &message["params"];
        match method {
            "initialize" => {
                let folders = params["workspaceFolders"].as_array().into_iter().flatten();
                self.roots = folders
                    .chain([&params["rootUri"]])
                    .filter_map(|f| path_of(&json!({ "uri": f.get("uri").unwrap_or(f) })))
                    .map(PathBuf::from)
                    .collect();
                self.roots.dedup();
                Ok(json!({
                    "capabilities": {
                        "textDocumentSync": { "openClose": true, "change": 1 },
                        "definitionProvider": true,
                        "referencesProvider": true,
                        "renameProvider": { "prepareProvider": true },
                        "hoverProvider": true,
                        "documentFormattingProvider": true,
                    },
                    "serverInfo": { "name": "hypra", "version": env!("CARGO_PKG_VERSION") },
                }))
            }
            "shutdown" => {
                self.shut_down = true;
                Ok(Json::Null)
            }
            "textDocument/didOpen" => {
                let document = &params["textDocument"];
                if let Some(path) = path_of(document) {
                    self.open.insert(path, document["text"].as_str().unwrap_or_default().to_string());
                    self.check();
                }
                Ok(Json::Null)
            }
            "textDocument/didChange" => {
                // the server asks for whole documents, so the last change is the new text
                let text = params["contentChanges"].as_array().and_then(|c| c.last()).and_then(|c| c["text"].as_str());
                if let (Some(path), Some(text)) = (path_of(&params["textDocument"]), text)
                    && let Some(open) = self.open.get_mut(&path)
                {
                    *open = text.to_string();
                    self.check();
                }
                Ok(Json::Null)
            }
            "textDocument/didClose" => {
                if let Some(path) = path_of(&params["textDocument"]) {
                    self.open.remove(&path);
                    self.check();
                }
                Ok(Json::Null)
            }
            "textDocument/definition" => Ok(self.definition(params).unwrap_or(Json::Null)),
            "textDocument/references" => Ok(self.references(params).unwrap_or(json!([]))),
            "textDocument/prepareRename" => Ok(self.prepare_rename(params).unwrap_or(Json::Null)),
            "textDocument/rename" => self.rename(params),
            "textDocument/hover" => Ok(self.hover(params).unwrap_or(Json::Null)),
            "textDocument/formatting" => Ok(self.format(params).unwrap_or(json!([]))),
            _ if message.get("id").is_some() => Err((-32601, format!("`{method}` isn't supported"))),
            _ => Ok(Json::Null),
        }
    }

    /// Reads a file the way the editor sees it: open documents as they are, others from disk.
    fn load(&self, path: &str) -> Result<String, String> {
        match self.open.get(path) {
            Some(text) => Ok(text.clone()),
            None => std::fs::read_to_string(path).map_err(|e| e.to_string()),
        }
    }

    /// The text of a line, counted from 1, of any file.
    fn line(&self, path: &str, line: usize) -> String {
        let text = self.load(path).unwrap_or_default();
        crate::line_of(&text, line).to_string()
    }

    /// The document and position a request is about, as the compiler counts them.
    fn position(&self, params: &Json) -> Option<(String, usize, usize)> {
        let path = path_of(&params["textDocument"])?;
        let line = usize::try_from(params["position"]["line"].as_u64()?).ok()?.checked_add(1)?;
        let character = usize::try_from(params["position"]["character"].as_u64()?).ok()?;
        let col = chars_before(&self.line(&path, line), character) + 1;
        Some((path, line, col))
    }

    /// Checks every open document and sends the problems found in it and in the files it imports.
    fn check(&mut self) {
        let mut found: BTreeMap<String, Vec<Json>> = BTreeMap::new();
        for path in self.open.keys() {
            // a page that crashes the compiler mustn't keep the others from being checked
            let check = std::panic::AssertUnwindSafe(|| check_with(path, &|p| self.load(p)));
            let Ok(checked) = std::panic::catch_unwind(check) else { continue };
            found.entry(path.clone()).or_default();
            for d in &checked.diagnostics {
                let Some(source) = checked.sources.get(d.pos.file) else { continue };
                let diagnostics = found.entry(source.path.clone()).or_default();
                let diagnostic = to_protocol(d, source);
                // a file imported by two open pages is checked twice
                if !diagnostics.contains(&diagnostic) {
                    diagnostics.push(diagnostic);
                }
            }
        }
        for gone in self.reported.iter().filter(|p| !found.contains_key(*p)) {
            notify("textDocument/publishDiagnostics", json!({ "uri": uri_of(gone), "diagnostics": [] }));
        }
        for (path, diagnostics) in &found {
            notify("textDocument/publishDiagnostics", json!({ "uri": uri_of(path), "diagnostics": diagnostics }));
        }
        self.reported = found.into_keys().collect();
    }

    fn definition(&self, params: &Json) -> Option<Json> {
        let (path, line, col) = self.position(params)?;
        let found = editor::definition(&path, line, col, &|p| self.load(p))?;
        Some(self.location(&found))
    }

    fn references(&self, params: &Json) -> Option<Json> {
        let (path, line, col) = self.position(params)?;
        let mut found = editor::references(&path, line, col, &self.files(), &|p| self.load(p));
        if params["context"]["includeDeclaration"] == false {
            let definition = editor::definition(&path, line, col, &|p| self.load(p));
            found.retain(|l| Some(l) != definition.as_ref());
        }
        Some(found.iter().map(|l| self.location(l)).collect())
    }

    fn prepare_rename(&self, params: &Json) -> Option<Json> {
        let (path, line, col) = self.position(params)?;
        let (at, name) = editor::renameable(&path, line, col, &|p| self.load(p))?;
        Some(json!({ "range": self.range(&at.path, at.line, at.col, at.len), "placeholder": name }))
    }

    fn rename(&self, params: &Json) -> Reply {
        let invalid = |text: &str| (-32602, text.to_string());
        let (path, line, col) = self.position(params).ok_or_else(|| invalid("no document or position"))?;
        let new_name = params["newName"].as_str().ok_or_else(|| invalid("no new name"))?;
        let places =
            editor::rename(&path, line, col, new_name, &self.files(), &|p| self.load(p)).map_err(|e| invalid(&e))?;
        let mut changes: BTreeMap<String, Vec<Json>> = BTreeMap::new();
        for place in places {
            let edit =
                json!({ "range": self.range(&place.path, place.line, place.col, place.len), "newText": new_name });
            changes.entry(uri_of(&place.path)).or_default().push(edit);
        }
        Ok(json!({ "changes": changes }))
    }

    /// The project's `.hypra` files: in the open folders, and the open documents.
    fn files(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = self.open.keys().map(PathBuf::from).collect();
        for root in &self.roots {
            hypra_files(root, &mut files);
        }
        files.sort();
        files.dedup();
        files
    }

    fn location(&self, l: &editor::Location) -> Json {
        json!({ "uri": uri_of(&l.path), "range": self.range(&l.path, l.line, l.col, l.len) })
    }

    fn hover(&self, params: &Json) -> Option<Json> {
        let (path, line, col) = self.position(params)?;
        let text = editor::hover(&path, line, col, &|p| self.load(p))?;
        Some(json!({ "contents": { "kind": "markdown", "value": text } }))
    }

    fn format(&self, params: &Json) -> Option<Json> {
        let path = path_of(&params["textDocument"])?;
        let text = self.load(&path).ok()?;
        let formatted = fmt::format(&text).ok()?;
        if formatted == text {
            return Some(json!([]));
        }
        let end = json!({ "line": text.lines().count() + 1, "character": 0 });
        Some(json!([{ "range": { "start": { "line": 0, "character": 0 }, "end": end }, "newText": formatted }]))
    }

    /// A protocol range for `len` characters from `line`:`col` of a file, as the compiler counts them.
    fn range(&self, path: &str, line: usize, col: usize, len: usize) -> Json {
        range_in(&self.line(path, line), line, col, len)
    }
}

/// Collects the `.hypra` files in a folder and its subfolders, leaving out hidden folders and those that hold
/// dependencies or build output.
fn hypra_files(dir: &Path, files: &mut Vec<PathBuf>) {
    const SKIP: [&str; 4] = ["node_modules", "vendor", "target", "dist"];
    // enough for any real project; a server pointed at a home folder stays responsive
    const MAX: usize = 20_000;
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if files.len() >= MAX {
            return;
        }
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() && !name.starts_with('.') && !SKIP.contains(&name.as_ref()) {
            hypra_files(&path, files);
        } else if kind.is_file() && name.ends_with(".hypra") {
            files.push(path);
        }
    }
}

/// A diagnostic as the protocol has it, with a link to where the docs explain its rule.
fn to_protocol(d: &Diagnostic, source: &Source) -> Json {
    let line_text = crate::line_of(&source.text, d.pos.line);
    let message = match &d.help {
        Some(help) => format!("{}\n{help}", d.message),
        None => d.message.clone(),
    };
    json!({
        "range": range_in(line_text, d.pos.line, d.pos.col, d.len),
        "severity": if d.is_error() { 1 } else { 2 },
        "code": d.code,
        "codeDescription": { "href": help::rule_url(&d.code) },
        "source": "hypra",
        "message": message,
    })
}

/// A protocol range within one line. A position of line 0 means the whole file: it's shown at its start.
fn range_in(line_text: &str, line: usize, col: usize, len: usize) -> Json {
    let start = col.saturating_sub(1);
    let at = |chars: usize| -> usize { line_text.chars().take(chars).map(char::len_utf16).sum() };
    let line = line.saturating_sub(1);
    json!({
        "start": { "line": line, "character": at(start) },
        "end": { "line": line, "character": at(start + len.max(1)) },
    })
}

/// How many characters come before a UTF-16 offset in a line.
fn chars_before(line_text: &str, utf16: usize) -> usize {
    let mut units = 0;
    line_text
        .chars()
        .take_while(|c| {
            units += c.len_utf16();
            units <= utf16
        })
        .count()
}

/// The path of a `file:` document, or `None` for unsaved documents without one. Paths are normalized the way the
/// compiler normalizes the files it loads, so an open document is found under the path the compiler asks for.
fn path_of(document: &Json) -> Option<String> {
    let rest = document["uri"].as_str()?.strip_prefix("file://")?;
    let (host, path) = rest.split_at(rest.find('/')?);
    let path = percent_decode(path);
    let path = match host {
        "" | "localhost" if cfg!(windows) => {
            // `file:///C:/Users/...`
            let drive = path.len() > 2 && path.as_bytes()[1].is_ascii_alphabetic() && path.as_bytes()[2] == b':';
            if drive { path[1..].replace('/', "\\") } else { path.replace('/', "\\") }
        }
        "" | "localhost" => path,
        // a Windows network share: `file://server/share/...`
        host if cfg!(windows) => format!("\\\\{host}{}", path.replace('/', "\\")),
        _ => return None,
    };
    Some(crate::normalize(Path::new(&path)).to_string_lossy().into_owned())
}

/// The `file:` URI of a path, the reverse of `path_of`.
fn uri_of(path: &str) -> String {
    let path = path.replace('\\', "/");
    // a Windows network share keeps its server as the URI's host
    let path = match path.strip_prefix("//") {
        Some(share) => share.to_string(),
        None if path.starts_with('/') => path,
        None => format!("/{path}"),
    };
    let mut uri = String::from("file://");
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' | b':' => uri.push(byte as char),
            _ => uri.push_str(&format!("%{byte:02X}")),
        }
    }
    uri
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
