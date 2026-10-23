//! Runs generated JavaScript with the runtime in js/ and compares it to the Rust interpreter.
//! Skipped when `node` isn't installed.

use hypra::{compile_file, js_file, js_with};
use serde_json::{Value as Json, json};
use std::path::{Path, PathBuf};
use std::process::Command;

fn node_available() -> bool {
    Command::new("node").arg("--version").output().is_ok_and(|o| o.status.success())
}

/// A folder where `import "hypra/runtime"` finds the runtime in js/, as it would in an app's node_modules.
fn work_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("js-{name}"));
    let modules = dir.join("node_modules");
    std::fs::create_dir_all(&modules).unwrap();
    let link = modules.join("hypra");
    if !link.exists() {
        let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("js");
        #[cfg(unix)]
        std::os::unix::fs::symlink(runtime, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(runtime, &link).unwrap();
    }
    dir
}

/// Renders a generated page with Node and returns its output as JSON, or the DataError's message.
/// `alternate` is where the page's Markdown is served; the static build links `<stem>.md`.
fn render_with_node(code: &str, data: &Json, strict: bool, name: &str, alternate: &str) -> Result<Json, String> {
    let dir = work_dir(name);
    std::fs::write(dir.join("page.mjs"), code).unwrap();
    std::fs::write(dir.join("data.json"), data.to_string()).unwrap();
    let harness = format!(
        r#"import {{ readFileSync }} from "node:fs";
import page from "./page.mjs";
const data = JSON.parse(readFileSync(new URL("./data.json", import.meta.url), "utf8"));
try {{
  const out = page.render(data, {{ strict: {strict}, alternateUrl: {alternate} }});
  process.stdout.write(JSON.stringify({{ html: out.html, markdown: out.markdown, json: out.json, webmcp: out.webmcp }}));
}} catch (e) {{
  if (e.name !== "DataError") throw e;
  process.stderr.write(e.message);
  process.exit(1);
}}
"#,
        alternate = Json::String(alternate.to_string()),
    );
    std::fs::write(dir.join("harness.mjs"), harness).unwrap();
    let out = Command::new("node").arg(dir.join("harness.mjs")).output().unwrap();
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(serde_json::from_slice(&out.stdout).expect("harness prints JSON"))
}

/// Runs a script next to the runtime and returns what it prints.
fn run_node(name: &str, script: &str) -> String {
    let dir = work_dir(name);
    std::fs::write(dir.join("script.mjs"), script).unwrap();
    let out = Command::new("node").arg(dir.join("script.mjs")).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn js_output_matches_the_interpreter() {
    if !node_available() {
        eprintln!("node not installed, skipping");
        return;
    }
    let data: Json = serde_json::from_str(include_str!("../examples/product.json")).unwrap();
    let expected = compile_file("examples/product.hypra", &data).output.unwrap();
    let code = js_file("examples/product.hypra").code.unwrap();

    let actual = render_with_node(&code, &data, true, "product", "product.md").unwrap();
    assert_eq!(actual["html"].as_str().unwrap(), expected.html);
    assert_eq!(actual["markdown"].as_str().unwrap(), expected.markdown);
    assert_eq!(actual["json"], expected.json);
    assert_eq!(actual["webmcp"], expected.webmcp);
}

#[test]
fn strict_js_reports_data_errors_with_their_source_location() {
    if !node_available() {
        eprintln!("node not installed, skipping");
        return;
    }
    let page = "page(title: \"T\", description: \"D\") {\n  props(user: User)\n  p \"{user.name}\"\n}\n";
    let code = js_with("site/page.hypra", &|_| Ok(page.to_string())).code.unwrap();

    let missing = render_with_node(&code, &json!({}), true, "missing", "page.md").unwrap_err();
    assert_eq!(missing, "site/page.hypra:2:9: prop `user` is missing from the data");

    let error = render_with_node(&code, &json!({ "user": {} }), true, "strict", "page.md").unwrap_err();
    assert_eq!(error, "site/page.hypra:3:6: `user` has no field `name`");

    let lenient = render_with_node(&code, &json!({ "user": {} }), false, "lenient", "page.md").unwrap();
    assert!(lenient["html"].as_str().unwrap().contains("<p></p>"));
}

#[test]
fn special_characters_survive_js_quoting() {
    if !node_available() {
        eprintln!("node not installed, skipping");
        return;
    }
    let page = r#"allow("SEO-004") "no h1 in this test" page(title: "Cost: $5 \\ \"quoted\" 'single' ${{x}} `tick`", description: "Tabs\tand\nlines {note.text}, and enough words for a description.") {
  props(note: Note)
  p "$note {note.text} <b> & ünïcödé ✓ LS </script>"
  p "{note.text}"
  allow("SEC-002") "this test checks that raw output survives JavaScript quoting"
  raw "<i>{note.text}</i>"
}
"#
    .replace("LS", "\u{2028}");
    let data = json!({ "note": { "text": "a ${b} \\ \"c\" 'd' `e` <f> & g\nh \u{2028}" } });
    let load = |_: &str| Ok(page.to_string());
    let expected = hypra::compile_with("page.hypra", &data, &load);
    assert!(expected.diagnostics.is_empty(), "{:?}", expected.diagnostics);
    let expected = expected.output.unwrap();
    let code = js_with("page.hypra", &load).code.unwrap();

    let actual = render_with_node(&code, &data, true, "quoting", "page.md").unwrap();
    assert_eq!(actual["html"].as_str().unwrap(), expected.html);
    assert_eq!(actual["markdown"].as_str().unwrap(), expected.markdown);
    assert_eq!(actual["json"], expected.json);
}

/// Values whose display could differ between JavaScript and Rust.
#[test]
fn edge_values_render_the_same_in_js() {
    if !node_available() {
        eprintln!("node not installed, skipping");
        return;
    }
    let page = r#"allow("SEO-004") "no h1 in this test" page(title: "Edge values", description: "Values that JavaScript and Rust could display differently, side by side.") {
  props(v: V)
  if v.empty { p "empty object is true" } else { p "empty object is false" }
  if v.zero { p "zero is true" } else { p "zero is false" }
  p "{v.big} {v.small} {v.tiny} {v.half} {v.negative} {v.decimal} {v.object} {v.list}"
  a(v.url) "Link"
}
"#;
    let data = json!({ "v": {
        "empty": {}, "zero": 0, "big": 1e20, "small": 1.5e-7, "tiny": 0.00001, "half": 0.5, "negative": -3,
        "decimal": 123456789.125, "object": { "b": 1, "a": [true, null, "x"] }, "list": [1, 2.5],
        "url": " JaVaScRiPt:alert(1)",
    } });
    let load = |_: &str| Ok(page.to_string());
    let expected = hypra::compile_with("page.hypra", &data, &load).output.unwrap();
    let code = js_with("page.hypra", &load).code.unwrap();
    let actual = render_with_node(&code, &data, false, "edge", "page.md").unwrap();
    assert_eq!(actual["html"].as_str().unwrap(), expected.html);
    assert!(expected.html.contains("empty object is false") && expected.html.contains("1e+20 1.5e-7 0.00001 0.5 -3"));
}

/// What only JavaScript has: Maps, Sets, class instances with getters, dates, and the Accept header.
#[test]
fn js_runtime_reads_javascript_values() {
    if !node_available() {
        eprintln!("node not installed, skipping");
        return;
    }
    let page = r#"allow("SEO-004") "no h1 in this test" page(title: "Values", description: "Values that only JavaScript has, read the way a JSON value would be.") {
  props(user: User, tags: List, when: Text)
  p "{user.name} {user.initial}"
  for tag in tags { p "{tag}" }
  p "{when}"
  p "{user.toString}"
}
"#;
    let code = js_with("page.hypra", &|_| Ok(page.to_string())).code.unwrap();
    let dir = work_dir("values");
    std::fs::write(dir.join("page.mjs"), code).unwrap();
    let out = run_node(
        "values",
        r#"import page from "./page.mjs";
class User { constructor(name) { this.name = name; } get initial() { return this.name[0]; } }
const out = page.render(new Map([["user", new User("Ana")], ["tags", new Set(["road", "trail"])], ["when", new Date(Date.UTC(2026, 0, 2))]]));
console.log(out.html.match(/<p>.*<\/p>/g).join(" "));
// a method every object inherits isn't data
try { page.render({ user: {}, tags: [], when: "" }, { strict: true }); } catch (e) { console.log(e.message); }
console.log(out.negotiate("text/markdown").contentType, out.negotiate("application/json;q=0.9, text/html").format, out.negotiate("").format);
"#,
    );
    assert_eq!(
        out,
        "<p>Ana A</p> <p>road</p> <p>trail</p> <p>2026-01-02T00:00:00.000Z</p> <p></p>\npage.hypra:3:6: `user` has no field `name`\ntext/markdown; charset=utf-8 html html\n"
    );
}

#[test]
fn hidden_form_fields_only_go_to_this_site() {
    if !node_available() {
        eprintln!("node not installed, skipping");
        return;
    }
    let page = r#"page(title: "Forms", description: "Forms sent to this site and to others, with a token for this site only.") {
  props(target: Text)
  h1 "Forms"
  form("/local", name: "local") { button "Send" }
  form(target, name: "elsewhere") { button "Send" }
}
"#;
    let code = js_with("page.hypra", &|_| Ok(page.to_string())).code.unwrap();
    let dir = work_dir("forms");
    std::fs::write(dir.join("page.mjs"), code).unwrap();
    // the same URLs as `only_same_site_urls_count_as_same_site` in tests/security.rs, all in one run of node
    let same = ["/subscribe", "subscribe", "?page=2", "#top", "./a:b", "/a?next=https://x.example"];
    let elsewhere = [
        "https://evil.example/c",
        "//evil.example/c",
        "/\\evil.example/c",
        "\\\\evil.example/c",
        "/\t/evil.example/c",
        "ht\ttps://evil.example/c",
        "\u{1}//evil.example/c",
        " //evil.example/c",
        "mailto:a@b.example",
        "HTTPS://evil.example",
    ];
    let targets: Vec<&str> = same.iter().chain(&elsewhere).copied().collect();
    let script = format!(
        r#"import page from "./page.mjs";
for (const target of {targets}) {{
  const out = page.render({{ target }}, {{ formFields: () => ({{ _csrf: "t<" }}) }});
  console.log(out.html.split('name="_csrf"').length - 1);
}}
"#,
        targets = Json::from(targets.clone())
    );
    let counts: Vec<usize> = run_node("forms", &script).lines().map(|l| l.parse().unwrap()).collect();
    for (target, count) in targets.iter().zip(counts) {
        // the local form always gets the token; the other one only when its target is this site
        let expected = if same.contains(target) { 2 } else { 1 };
        assert_eq!(count, expected, "{target:?}");
    }
}

/// Data from an app can do what JSON can't: refer to itself, or have a field named like a JavaScript internal.
#[test]
fn js_runtime_survives_unusual_data() {
    if !node_available() {
        eprintln!("node not installed, skipping");
        return;
    }
    let page = r#"allow("SEO-004") "no h1 in this test" page(title: "Data", description: "Data that refers to itself, and a form field named like a JavaScript internal.") {
  props(node: Node)
  p "{node}"
  form("/a", name: "a") {
    field("__proto__", type: String, label: "Odd name")
    button "Send"
  }
}
"#;
    let code = js_with("page.hypra", &|_| Ok(page.to_string())).code.unwrap();
    let dir = work_dir("unusual");
    std::fs::write(dir.join("page.mjs"), code).unwrap();
    let out = run_node(
        "unusual",
        r#"import page from "./page.mjs";
const node = { name: "root", children: [] };
node.children.push({ name: "child", parent: node });
const out = page.render({ node });
console.log(out.html.match(/<p>.*<\/p>/)[0]);
console.log(Object.keys(out.json.actions[0].params).join(","), JSON.stringify(out.webmcp[0].inputSchema.properties));
"#,
    );
    assert_eq!(
        out,
        "<p>{&quot;name&quot;:&quot;root&quot;,&quot;children&quot;:[{&quot;name&quot;:&quot;child&quot;,&quot;parent&quot;:null}]}</p>\n__proto__ {\"__proto__\":{\"type\":\"string\"}}\n"
    );
}
