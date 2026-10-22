//! Every rule can fire: one small page per rule code, and the table covers every code the compiler has.

use hypra::compile_with;
use hypra::diag::RULES;
use serde_json::{Value as Json, json};
use std::collections::BTreeSet;

const DESCRIPTION: &str = "A test page with a description long enough for search results.";

/// A page with a title, description and h1, so only the rule under test fires.
fn page(body: &str) -> String {
    format!("page(title: \"T\", description: \"{DESCRIPTION}\") {{\n  h1 \"Main topic\"\n{body}\n}}\n")
}

/// A page with its own settings and no h1.
fn page_with(settings: &str, body: &str) -> String {
    format!("page({settings}) {{\n{body}\n}}\n")
}

/// Components nested `levels` deep, each calling the one inside it ten times.
fn expanding(levels: usize) -> String {
    let mut src = String::from("component C0 { p \"x\" }\n");
    for level in 1..=levels {
        let calls = format!("C{} ", level - 1).repeat(10);
        src.push_str(&format!("component C{level} {{ {calls}}}\n"));
    }
    src + &page(&format!("  C{levels}"))
}

struct Case {
    code: &'static str,
    /// The page, then any files it imports.
    files: Vec<(&'static str, String)>,
    data: Json,
}

fn case(code: &'static str, src: String) -> Case {
    Case { code, files: vec![("p.hypra", src)], data: json!({}) }
}

fn cases() -> Vec<Case> {
    let card = "component Card(title: Text) { p \"{title}\" }";
    let form = |inside: &str| page(&format!("  form(\"/a\", name: \"a\") {{ {inside} button \"Go\" }}"));
    let field = "field(\"e\", type: Email, label: \"E\")";
    vec![
        case("A11Y-001", page("  img(\"/a.jpg\")")),
        case("A11Y-002", page("  a(\"/x\")")),
        case("A11Y-003", page("  video(\"/v.mp4\")")),
        case("A11Y-004", page("  action(\"cart.add\")")),
        case("A11Y-005", page("  button")),
        case("A11Y-006", page("  h3 \"Details\"")),
        case("A11Y-007", page("  a(\"/x\") \"click here\"")),
        case("A11Y-008", page("  img(\"/a.jpg\", alt: \"image\")")),
        case("A11Y-009", page("  iframe(src: \"/x\")")),
        case("A11Y-010", page("  div(tabindex: 1) \"x\"")),
        case("A11Y-011", page("  video(\"/v.mp4\", captions: \"/c.vtt\", autoplay: true)")),
        case("A11Y-012", page("  div(aria-label: \"Box\") \"x\"")),
        case("ALLOW-001", page("  allow(\"A11Y-001\")\n  img(\"/a.jpg\")")),
        case("ALLOW-002", page("  allow \"no rule\"\n  p \"x\"")),
        case("ALLOW-003", page("  allow(\"NOPE-001\") \"reason\"\n  p \"x\"")),
        case("ATTR-001", page("  p(colour: \"red\") \"x\"")),
        case("ATTR-002", page("  raw")),
        case("ATTR-003", page("  img(\"/a.jpg\", alt: \"Shoe\", priority: \"yes\")")),
        case("ATTR-004", page("  p(\"x\") \"y\"")),
        case("ATTR-005", page("  p(id: \"a\", id: \"b\") \"x\"")),
        // a void element's string is the text after it, so this needs an element that isn't void
        case("ATTR-006", page("  form(\"/a\", name: \"a\") \"text\" { button \"Go\" }")),
        case("ATTR-007", page("  p(id: \"a\" = \"b\") \"x\"")),
        case("ATTR-008", page("  p(\"a b\": \"x\") \"y\"")),
        case("COMP-001", page("  Missing")),
        case("COMP-002", format!("{card}\n{}", page("  Card"))),
        case("COMP-003", format!("{card}\n{}", page("  Card(title: \"x\", nope: \"y\")"))),
        case("COMP-004", format!("{card}\n{card}\n{}", page("  Card(title: \"x\")"))),
        case("COMP-005", format!("component card {{ p \"x\" }}\n{}", page(""))),
        case("COMP-006", format!("{card}\n{}", page("  Card(\"x\")"))),
        case("COMP-007", format!("component Loop {{ Loop }}\n{}", page("  Loop"))),
        case("COMP-008", format!("component Card(42) {{ p \"x\" }}\n{}", page("  Card"))),
        case("COMP-009", expanding(6)),
        case("COMP-010", format!("component Box {{ p \"x\" }}\n{}", page("  Box { p \"y\" }"))),
        case("FORM-001", form("field(label: \"E\")")),
        case("FORM-002", form("field(\"e\", type: Email)")),
        case("FORM-003", page(&format!("  form(\"/a\", name: \"a\") {{ {field} }}"))),
        case("FORM-004", page(&format!("  form(\"/a\") {{ {field} button \"Go\" }}"))),
        case("FORM-005", page(&format!("  {field}"))),
        case("FORM-006", page("  form(\"/a\", name: \"a\", method: \"fly\") { button \"Go\" }")),
        case("IMPORT-001", format!("use \"missing.hypra\"\n{}", page(""))),
        case(
            "LAYOUT-001",
            page_with(&format!("layout: Nope, title: \"T\", description: \"{DESCRIPTION}\""), "  h1 \"x\""),
        ),
        case(
            "LAYOUT-002",
            format!(
                "layout L {{ p \"x\" }}\n{}",
                page_with(&format!("layout: L, title: \"T\", description: \"{DESCRIPTION}\""), "  h1 \"x\"")
            ),
        ),
        case("LAYOUT-003", format!("layout L {{ slot }}\nlayout L {{ slot }}\n{}", page(""))),
        case("LAYOUT-004", format!("layout l {{ slot }}\n{}", page(""))),
        case(
            "LAYOUT-005",
            format!(
                "layout A(layout: A) {{ slot }}\n{}",
                page_with(&format!("layout: A, title: \"T\", description: \"{DESCRIPTION}\""), "  h1 \"x\"")
            ),
        ),
        case("LEX-001", page("  p \"x\" ~")),
        case("LEX-002", page("  p \"never closed")),
        case("LEX-003", page("  p \"\\q\"")),
        case("PARSE-001", page("  p (")),
        case("PARSE-002", page(&format!("{}{}", "div { ".repeat(300), "}".repeat(300)))),
        case("SEC-001", page("  button(onclick: \"go()\") \"Go\"")),
        case("SEC-002", page("  props(h: Text)\n  raw \"{h}\"")),
        case("SEC-003", page("  a(\"javascript:go()\") \"Go\"")),
        case("SEC-004", page("  props(x: Text)\n  div(x-data: x) \"y\"")),
        case("SEC-005", page("  props(x: Text)\n  script \"let a = '{x}'\"")),
        case("SEC-006", form("button(formaction: \"https://e.example\") \"Elsewhere\"")),
        case(
            "SEC-007",
            page("  props(x: Text)\n  svg { a(href: \"/a\") { animate(attributeName: \"href\", values: x) } }"),
        ),
        case("SEM-001", page("  buton \"Buy\"")),
        case("SEM-002", "p \"no page\"\n".to_string()),
        case("SEM-003", page("  component Inner { p \"x\" }")),
        case("SEM-004", page("  img(alt: \"Shoe\")")),
        case("SEM-005", page("  action \"Go\"")),
        case("SEM-006", page("  slot")),
        Case {
            code: "SEM-007",
            files: vec![("p.hypra", format!("use \"other.hypra\"\n{}", page(""))), ("other.hypra", page(""))],
            data: json!({}),
        },
        case("SEM-008", page("  img(\"/a.jpg\", alt: \"Shoe\") { p \"x\" }")),
        case("SEM-009", page("  body { p \"x\" }")),
        case("SEM-010", page("  div { head { meta(name: \"a\", content: \"b\") } }")),
        case("SEM-011", page("  head { p \"x\" }")),
        case("SEM-012", page("  p(id: \"a\") \"x\"\n  p(id: \"a\") \"y\"")),
        case(
            "SEM-013",
            page_with(&format!("lang: \"english\", title: \"T\", description: \"{DESCRIPTION}\""), "  h1 \"x\""),
        ),
        case("SEM-014", page("  action(\"bad name!\") \"Go\"")),
        case("SEO-001", page_with(&format!("description: \"{DESCRIPTION}\""), "  h1 \"x\"")),
        case(
            "SEO-002",
            page_with(&format!("title: \"{}\", description: \"{DESCRIPTION}\"", "Long ".repeat(20)), "  h1 \"x\""),
        ),
        case("SEO-003", page("  h1 \"Again\"")),
        case("SEO-004", page_with(&format!("title: \"T\", description: \"{DESCRIPTION}\""), "  p \"x\"")),
        case(
            "SEO-005",
            page_with(&format!("canonical: \"/x\", title: \"T\", description: \"{DESCRIPTION}\""), "  h1 \"x\""),
        ),
        case("SEO-006", page_with("title: \"T\", description: \"Short\"", "  h1 \"x\"")),
        case("TYPE-001", page("  props(x: Text)\n  p \"{x}\"")),
        case("TYPE-002", page("  p \"{nope}\"")),
        Case {
            code: "TYPE-003",
            data: json!({ "x": "text" }),
            ..case("TYPE-003", page("  props(x: List)\n  for i in x { p \"{i}\" }"))
        },
        case("TYPE-004", form("field(\"e\", type: Nope, label: \"E\")")),
        case("TYPE-005", form("field(\"n\", type: Int(\"x\"), label: \"N\")")),
    ]
}

#[test]
fn every_rule_fires() {
    // pages nested to the limits recurse deeply; the CLI gives the compiler this much stack too
    std::thread::Builder::new().stack_size(256 << 20).spawn(fire_every_rule).unwrap().join().unwrap();
}

fn fire_every_rule() {
    let mut wrong = vec![];
    for c in cases() {
        let load = |path: &str| {
            c.files.iter().find(|(p, _)| *p == path).map(|(_, s)| s.clone()).ok_or("not found".to_string())
        };
        let result = compile_with("p.hypra", &c.data, &load);
        let codes: Vec<&str> = result.diagnostics.iter().map(|d| d.code.as_str()).collect();
        if !codes.contains(&c.code) {
            wrong.push(format!("{}: got {codes:?}", c.code));
        }
    }
    assert!(wrong.is_empty(), "these pages don't fire their rule:\n{}", wrong.join("\n"));
}

#[test]
fn every_rule_has_a_page() {
    let tested: BTreeSet<&str> = cases().iter().map(|c| c.code).collect();
    let rules: BTreeSet<&str> = RULES.iter().map(|(code, _)| *code).collect();
    assert_eq!(tested, rules);
}
