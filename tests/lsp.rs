//! `hypra lsp`, driven the way an editor drives it: messages over stdin and stdout.

use serde_json::{Value as Json, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Editor {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next_id: u64,
    /// Notifications from the server, oldest first.
    notifications: Vec<Json>,
}

impl Editor {
    fn start() -> Self {
        Self::start_in("")
    }

    /// Starts the server for an editor with a folder open, or none for `""`.
    fn start_in(folder: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hypra"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("start hypra lsp");
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut editor = Editor { child, input, output, next_id: 0, notifications: vec![] };
        let root = if folder.is_empty() { Json::Null } else { json!(folder) };
        let init = editor.request("initialize", json!({ "capabilities": {}, "rootUri": root }));
        assert_eq!(init["capabilities"]["hoverProvider"], true);
        editor.notify("initialized", json!({}));
        editor
    }

    fn send(&mut self, message: Json) {
        let body = message.to_string();
        write!(self.input, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        self.input.flush().unwrap();
    }

    fn receive(&mut self) -> Json {
        let mut length = 0;
        loop {
            let mut line = String::new();
            assert!(self.output.read_line(&mut line).unwrap() > 0, "the server closed its output");
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length: ") {
                length = value.parse().unwrap();
            }
        }
        let mut body = vec![0; length];
        self.output.read_exact(&mut body).unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    fn notify(&mut self, method: &str, params: Json) {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    fn request(&mut self, method: &str, params: Json) -> Json {
        self.next_id += 1;
        let id = self.next_id;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let message = self.receive();
            if message["id"] == id {
                assert!(message.get("error").is_none(), "{message}");
                return message["result"].clone();
            }
            self.notifications.push(message);
        }
    }

    /// The diagnostics last published for a file, after the server has handled everything sent so far.
    fn diagnostics(&mut self, uri: &str) -> Vec<Json> {
        // a request is answered after every notification sent before it
        self.request(
            "textDocument/hover",
            json!({ "textDocument": { "uri": uri }, "position": { "line": 0, "character": 0 } }),
        );
        let published = self.notifications.iter().rev().find(|n| n["params"]["uri"] == uri);
        published.map(|n| n["params"]["diagnostics"].as_array().unwrap().clone()).unwrap_or_default()
    }

    fn at(&mut self, method: &str, uri: &str, line: u64, character: u64) -> Json {
        self.request(
            method,
            json!({ "textDocument": { "uri": uri }, "position": { "line": line, "character": character } }),
        )
    }

    fn stop(mut self) -> i32 {
        self.request("shutdown", Json::Null);
        self.notify("exit", Json::Null);
        self.child.wait().unwrap().code().unwrap()
    }
}

fn open(editor: &mut Editor, uri: &str, text: &str) {
    editor.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "hypra", "version": 1, "text": text } }),
    );
}

fn change(editor: &mut Editor, uri: &str, text: &str) {
    editor.notify(
        "textDocument/didChange",
        json!({ "textDocument": { "uri": uri, "version": 2 }, "contentChanges": [{ "text": text }] }),
    );
}

/// A folder with a page and a component it imports, on disk, as an editor would find them.
fn project() -> (tempdir::Dir, String, String) {
    let dir = tempdir::Dir::new();
    std::fs::write(dir.path("card.hypra"), "component Card(title: Text) {\n  h2 \"{title}\"\n}\n").unwrap();
    let page = dir.uri("page.hypra");
    let card = dir.uri("card.hypra");
    (dir, page, card)
}

const PAGE: &str = "use \"card.hypra\"\n\npage(title: \"Shoes\", description: \"All our running shoes, from light racers to trail shoes.\") {\n  props(product: Product)\n  h1 \"{product.name}\"\n  Card(title: product.name)\n  form(\"/a\", name: \"a\") { field(\"email\", type: Email, label: \"Email\") button \"Go\" }\n}\n";

#[test]
fn checks_as_you_type() {
    let (_dir, page, _) = project();
    let mut editor = Editor::start();
    open(&mut editor, &page, PAGE);
    assert_eq!(editor.diagnostics(&page), Vec::<Json>::new());

    // an image without alt text appears on the line it's typed, before anything is saved
    change(&mut editor, &page, &PAGE.replace("  h1 ", "  img(\"/a.jpg\")\n  h1 "));
    let found = editor.diagnostics(&page);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["code"], "A11Y-001");
    assert_eq!(found[0]["range"]["start"], json!({ "line": 4, "character": 6 }));
    assert_eq!(found[0]["codeDescription"]["href"], "https://hypralang.dev/rules/a11y-001/");

    // and goes away when it's fixed
    change(&mut editor, &page, PAGE);
    assert_eq!(editor.diagnostics(&page), Vec::<Json>::new());
    assert_eq!(editor.stop(), 0);
}

#[test]
fn problems_in_an_open_import_are_checked_with_its_unsaved_text() {
    let (_dir, page, card) = project();
    let mut editor = Editor::start();
    open(&mut editor, &page, PAGE);
    open(&mut editor, &card, "component Card(title: Text) {\n  img(\"/x.jpg\")\n}\n");
    let found = editor.diagnostics(&card);
    assert_eq!(found.len(), 1, "a file checked from two open pages reports each problem once: {found:?}");
    assert_eq!(found[0]["code"], "A11Y-001");
}

#[test]
fn definitions_hover_and_formatting() {
    let (_dir, page, card) = project();
    let mut editor = Editor::start();
    open(&mut editor, &page, PAGE);

    let definition = editor.at("textDocument/definition", &page, 5, 3);
    assert_eq!(
        definition,
        json!({ "uri": card, "range": { "start": { "line": 0, "character": 10 }, "end": { "line": 0, "character": 14 } } })
    );

    let hover = |editor: &mut Editor, line, character| {
        editor.at("textDocument/hover", &page, line, character)["contents"]["value"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    };
    assert_eq!(hover(&mut editor, 4, 8), "```hypra\nprops(product: Product)\n```");
    assert_eq!(hover(&mut editor, 5, 3), "```hypra\ncomponent Card(title: Text)\n```\n\nfrom `card.hypra`");
    assert!(hover(&mut editor, 6, 48).starts_with("**Field type** `Email`, `Url`, `Tel`: an email address"));
    assert!(hover(&mut editor, 6, 27).starts_with("**`field`**"));
    assert!(hover(&mut editor, 4, 2).contains("developer.mozilla.org/docs/Web/HTML/Element/h1"));

    open(&mut editor, &card, "component Card(title:Text){\nh2 \"{title}\"\n}\n");
    let edits = editor.request(
        "textDocument/formatting",
        json!({ "textDocument": { "uri": card }, "options": { "tabSize": 2, "insertSpaces": true } }),
    );
    assert_eq!(edits[0]["newText"], "component Card(title: Text) {\n  h2 \"{title}\"\n}\n");
}

#[test]
fn positions_count_utf16_like_editors() {
    let (_dir, page, _) = project();
    let mut editor = Editor::start();
    // the emoji is two UTF-16 units but one character
    open(
        &mut editor,
        &page,
        &PAGE.replace("  h1 \"{product.name}\"", "  p \"🏃\" img(\"/a.jpg\")\n  h1 \"{product.name}\""),
    );
    let found = editor.diagnostics(&page);
    assert_eq!(found[0]["range"]["start"], json!({ "line": 4, "character": 13 }));
    let definition = editor.at("textDocument/definition", &page, 5, 9);
    assert_eq!(definition["range"]["start"], json!({ "line": 3, "character": 8 }));
}

#[test]
fn references_and_rename_reach_files_that_arent_open() {
    let (dir, page, card) = project();
    std::fs::write(dir.path("page.hypra"), PAGE).unwrap();
    let mut editor = Editor::start_in(&dir.uri(""));
    // only the component is open; the page that uses it is found in the folder
    open(&mut editor, &card, "component Card(title: Text) {\n  h2 \"{title}\"\n}\n");

    // Ctrl+click on the definition leads to itself, which editors take as "show the references"
    let definition = editor.at("textDocument/definition", &card, 0, 11);
    assert_eq!(definition["uri"], card);
    let found = editor.request(
        "textDocument/references",
        json!({ "textDocument": { "uri": card }, "position": { "line": 0, "character": 11 }, "context": { "includeDeclaration": true } }),
    );
    let uris: Vec<&str> = found.as_array().unwrap().iter().map(|l| l["uri"].as_str().unwrap()).collect();
    assert_eq!(uris, [card.as_str(), page.as_str()]);

    let prepared = editor.at("textDocument/prepareRename", &card, 0, 11);
    assert_eq!(prepared["placeholder"], "Card");
    let edit = editor.request(
        "textDocument/rename",
        json!({ "textDocument": { "uri": card }, "position": { "line": 0, "character": 11 }, "newName": "Tile" }),
    );
    assert_eq!(edit["changes"][&page][0]["range"]["start"], json!({ "line": 5, "character": 2 }));
    assert_eq!(edit["changes"][&card][0]["newText"], "Tile");

    // a name that doesn't fit is refused, with the reason
    editor.next_id += 1;
    let id = editor.next_id;
    editor.send(json!({ "jsonrpc": "2.0", "id": id, "method": "textDocument/rename", "params": { "textDocument": { "uri": card }, "position": { "line": 0, "character": 11 }, "newName": "tile" } }));
    let refused = loop {
        let message = editor.receive();
        if message["id"] == id {
            break message;
        }
    };
    assert!(refused["error"]["message"].as_str().unwrap().contains("capitalized"), "{refused}");
}

#[test]
fn survives_broken_messages_and_odd_uris() {
    let (dir, page, _) = project();
    let mut editor = Editor::start();
    // a frame without a length, and a body that isn't JSON, are skipped
    write!(editor.input, "X-Other: 1\r\n\r\n").unwrap();
    write!(editor.input, "Content-Length: 4\r\n\r\n{{nop").unwrap();
    editor.input.flush().unwrap();
    // positions past anything a file has find nothing, without failing
    let far = json!({ "textDocument": { "uri": page }, "position": { "line": u64::MAX, "character": u64::MAX } });
    assert_eq!(editor.request("textDocument/hover", far), Json::Null);

    // `file://localhost/...` and `..` in a path name the same file as the plain URI
    let unusual = page.replace("file://", "file://localhost").replace("/page.hypra", "/sub/../page.hypra");
    open(&mut editor, &unusual, &PAGE.replace("  h1 ", "  img(\"/a.jpg\")\n  h1 "));
    let found = editor.diagnostics(&page);
    assert_eq!(found.len(), 1, "{found:?}");

    // a change to a document that was never opened is ignored
    let other = dir.uri("other.hypra");
    change(&mut editor, &other, "p \"never opened\"");
    assert_eq!(editor.diagnostics(&other), Vec::<Json>::new());
    assert_eq!(editor.stop(), 0);
}

#[test]
fn a_huge_length_is_skipped_instead_of_allocated() {
    let mut editor = Editor::start();
    // it would be skipped as its bytes arrive; here the editor goes away first, and the server ends normally
    write!(editor.input, "Content-Length: 99999999999999999\r\n\r\n{{}}").unwrap();
    drop(editor.input);
    assert_eq!(editor.child.wait().unwrap().code(), Some(1));
}

/// A temporary folder, removed when dropped.
mod tempdir {
    pub struct Dir(std::path::PathBuf);

    impl Dir {
        pub fn new() -> Self {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("hypra-lsp-{}-{n} with space", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Dir(path)
        }

        pub fn path(&self, name: &str) -> std::path::PathBuf {
            self.0.join(name)
        }

        pub fn uri(&self, name: &str) -> String {
            format!("file://{}", self.path(name).display()).replace(' ', "%20")
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
