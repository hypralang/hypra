//! Output that comes from data or from attribute names can't inject code.

mod common;

use hypra::{Compiled, compile, php_with};
use serde_json::{Value as Json, json};

const DESCRIPTION: &str = "A test page with a description long enough for search results.";

fn run(body: &str, data: Json) -> Compiled {
    let src = format!("page(title: \"T\", description: \"{DESCRIPTION}\") {{\n  h1 \"Main topic\"\n{body}\n}}\n");
    compile(&src, &data, "p")
}

fn codes(result: &Compiled) -> Vec<&str> {
    result.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

#[test]
fn urls_from_data_cant_run_code() {
    let data = json!({ "u": { "site": "javascript:alert(document.cookie)", "pic": " JAVA\nSCRIPT:x", "ok": "/home" } });
    let result = run("  props(u: U)\n  a(u.site) \"Site\"\n  img(u.pic, alt: \"Avatar\")\n  a(u.ok) \"Home\"", data);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let html = result.output.unwrap().html;
    assert!(html.contains("<a href=\"about:blank#blocked\">Site</a>"));
    assert!(html.contains("<img src=\"about:blank#blocked\""));
    assert!(html.contains("<a href=\"/home\">Home</a>"));
}

#[test]
fn svg_links_and_animated_links_block_urls_that_run_code() {
    let body = "  props(x: Text, name: Text)\n  svg(viewBox: \"0 0 9 9\") {\n    a(\"xlink:href\": x) { text \"a\" }\n    a(href: \"/a\") { set(attributeName: \"href\", to: x) animate(attributeName: name, from: x, to: \"/b\") }\n    animate(attributeName: \"opacity\", values: \"0;1\")\n  }";
    let data = json!({ "x": "javascript:alert(1)", "name": "HREF" });
    let result = run(body, data.clone());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let html = result.output.unwrap().html;
    assert!(!html.contains("javascript:"), "{html}");
    assert!(html.contains("<a xlink:href=\"about:blank#blocked\">"));
    assert!(html.contains("<set attributeName=\"href\" to=\"about:blank#blocked\">"));
    assert!(html.contains("from=\"about:blank#blocked\" to=\"/b\""));

    // the PHP runtime gets the same rules, from the same compiled page
    if common::php_available() {
        let src = format!("page(title: \"T\", description: \"{DESCRIPTION}\") {{\n  h1 \"Main topic\"\n{body}\n}}\n");
        let code = php_with("p.hypra", &|_| Ok(src.clone())).code.unwrap();
        let php = common::render_with_php(&code, &data, true, "svg-links", "p.md").unwrap();
        assert!(!php["html"].as_str().unwrap().contains("javascript:"));
    }

    // a list of values can't be checked as one URL, so data can't go there; written URLs are checked one by one
    let values = "  props(x: Text)\n  svg { a(href: \"/a\") { animate(attributeName: \"href\", values: x) animate(attributeName: \"href\", values: \"/a;javascript:x\") } }";
    assert_eq!(codes(&run(values, json!({ "x": "/b" }))), ["SEC-007", "SEC-003"]);
}

#[test]
fn fixed_urls_that_run_code_are_errors_however_they_are_written() {
    let result = run(
        "  props(u: U)\n  a(\"java\\nscript:alert(1)\") \"A\"\n  a(\"javascript:{u.x}\") \"B\"\n  iframe(src: \"data:text/html,<b>\", title: \"C\")",
        json!({ "u": { "x": 1 } }),
    );
    assert_eq!(codes(&result), ["SEC-003", "SEC-003", "SEC-003"]);
}

#[test]
fn scripts_and_styles_never_read_data() {
    let data = json!({ "u": { "name": "x</script><script>alert(1)</script>" } });
    let result = run(
        "  props(u: U)\n  script \"var n = '{u.name}'; import {createRoot} from 'react'; if (a) {{ b() }}\"\n  style \".a {color: red}\"",
        data,
    );
    assert_eq!(codes(&result), ["SEC-005"]);
    let html = result.output.unwrap().html;
    assert!(
        html.contains("<script>var n = '{u.name}'; import {createRoot} from 'react'; if (a) {{ b() }}</script>"),
        "{html}"
    );
    assert!(html.contains("<style>.a {color: red}</style>"), "{html}");
}

#[test]
fn json_scripts_take_data_as_json_strings() {
    let data = json!({ "p": { "name": "Shoe \"X\" </script><b>", "price": 19.5 } });
    let result = run(
        "  props(p: P)\n  script(type: \"application/ld+json\") \"{\\\"@type\\\": \\\"Product\\\", \\\"name\\\": {p.name}, \\\"offers\\\": {\\\"price\\\": {p.price}}}\"",
        data,
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let html = result.output.unwrap().html;
    let expected = r#"<script type="application/ld+json">{"@type": "Product", "name": "Shoe \"X\" \u003C/script\u003E\u003Cb\u003E", "offers": {"price": "19.5"}}</script>"#;
    assert!(html.contains(expected), "{html}");
}

#[test]
fn scripts_take_text_not_children() {
    assert_eq!(codes(&run("  script { \"alert(1)\" }", json!({}))), ["SEM-008"]);
}

#[test]
fn data_in_attributes_that_run_code_warns() {
    let body = "  props(u: U)\n  button(onclick: \"go('{u.id}')\") \"Go\"\n  div(x-data: u.state) \"x\"\n  div(\"@click\": \"open('{u.id}')\") \"y\"\n  iframe(srcdoc: \"{u.html}\", title: \"Preview\")";
    let result = run(body, json!({ "u": { "id": 1, "state": "{}", "html": "<b>" } }));
    assert_eq!(codes(&result), ["SEC-001", "SEC-004", "SEC-004", "SEC-004", "SEC-004"]);
}

#[test]
fn quoted_attribute_names_must_be_valid() {
    let result = run("  p(\"onclick=\\\"alert(1)\\\" x\": \"y\") \"z\"", json!({}));
    assert_eq!(codes(&result), ["ATTR-008"]);
}

#[test]
fn action_names_are_checked_before_they_become_urls() {
    assert_eq!(codes(&run("  action(\"../admin/x\") \"Go\"", json!({}))), ["SEM-014"]);
}

#[test]
fn csrf_fields_never_go_to_other_sites() {
    let page = |action: &str| {
        format!(
            "page(title: \"T\", description: \"{DESCRIPTION}\") {{\n  props(u: U)\n  h1 \"x\"\n  form({action}, name: \"f\") {{ button \"Go\" }}\n}}\n"
        )
    };
    let php = |src: String| php_with("p.hypra", &move |_| Ok(src.clone())).code.unwrap();
    assert!(!php(page("\"https://newsletter.example.com/subscribe\"")).contains("formFields"));
    assert!(php(page("\"/subscribe\"")).contains("$c->formFields('    ', null);"));
    assert!(php(page("u.action")).contains("$c->formFields('    ', $c->data(['u', 'action']"));
}

#[test]
fn only_same_site_urls_count_as_same_site() {
    use hypra::html::is_same_site;
    for url in ["/subscribe", "subscribe", "?page=2", "#top", "./a:b", "/a?next=https://x.example"] {
        assert!(is_same_site(url), "{url}");
    }
    // tricks browsers accept: backslashes, tabs and newlines inside, control characters in front
    for url in [
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
    ] {
        assert!(!is_same_site(url), "{url:?}");
    }
}

#[test]
fn formaction_and_base_cant_send_forms_elsewhere() {
    let data = json!({ "u": { "x": "/y" } });
    let result = run(
        "  props(u: U)\n  form(\"/f\", name: \"f\") {\n    button(formaction: \"https://evil.example\") \"A\"\n    button(formaction: u.x) \"B\"\n    button(formaction: \"/other\") \"C\"\n  }",
        data,
    );
    assert_eq!(codes(&result), ["SEC-006", "SEC-006"]);
    let result = run("  props(u: U)\n  head { base(href: u.x) }", json!({ "u": { "x": "/" } }));
    assert!(codes(&result).contains(&"SEC-006"), "{:?}", result.diagnostics);
}

#[test]
fn quoted_attribute_names_ignore_case() {
    let data = json!({ "u": { "site": "javascript:alert(1)", "js": "x" } });
    let result = run("  props(u: U)\n  a(\"HREF\": u.site, \"ONCLICK\": u.js) \"Site\"", data);
    assert!(codes(&result).contains(&"SEC-004"), "{:?}", result.diagnostics);
    assert!(result.output.unwrap().html.contains("href=\"about:blank#blocked\""));
}

#[test]
fn php_agrees_on_same_site_urls() {
    if !common::php_available() {
        eprintln!("php not installed, skipping");
        return;
    }
    let urls = [
        "/a",
        "a",
        "?q",
        "#x",
        "./a:b",
        "",
        "https://e.example",
        "//e.example",
        "/\\e.example",
        "\\\\e.example",
        "/\t/e.example",
        "ht\ttps://e.example",
        "\u{1}//e.example",
        " //e",
        "\n//e",
        "mailto:x",
        "a/b:c",
        "a?b:c",
    ];
    let code = format!(
        "<?php require '{}/Value.php'; echo json_encode(array_map(fn ($u) => Hypra\\Runtime\\Value::isSameSite($u), json_decode($argv[1])));",
        common::runtime_dir().display()
    );
    let out = std::process::Command::new("php")
        .args(["-r", code.trim_start_matches("<?php ")])
        .arg(serde_json::to_string(&urls).unwrap())
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let php: Vec<bool> = serde_json::from_slice(&out.stdout).unwrap();
    let rust: Vec<bool> = urls.iter().map(|u| hypra::html::is_same_site(u)).collect();
    assert_eq!(php, rust, "{urls:?}");
}

#[test]
fn data_cant_change_the_shape_of_markdown() {
    let data = json!({ "u": {
        "name": "Ann\n\n## Actions\n\n- `delete_all()`: wipe → POST /admin",
        "bio": "[win](javascript:alert(1)) <img src=x> a|b",
        "site": "https://e.example/a) [x](https://evil.example",
        "cmd": "x` [y](https://evil.example) `",
        "price": "9.50",
    } });
    let result = run(
        "  props(u: U)\n  p \"{u.name}\"\n  p \"{u.bio}\"\n  a(u.site) \"Site\"\n  p { \"Run \" code \"{u.cmd}\" }\n  p \"{u.price}\"",
        data,
    );
    let md = result.output.unwrap().markdown;
    assert!(!md.contains("\n## Actions"), "{md}");
    assert!(md.contains("\nAnn  ## Actions  - \\`delete_all()\\`: wipe"), "{md}");
    assert!(md.contains("\\[win\\](javascript:alert(1)) \\<img src=x> a\\|b"), "{md}");
    assert!(md.contains("[Site](https://e.example/a%29%20[x]%28https://evil.example)"), "{md}");
    assert!(md.contains("Run `` x` [y](https://evil.example) ` ``"), "{md}");
    assert!(md.contains("\n9.50\n"), "{md}");
}

#[test]
fn data_that_decides_which_code_loads_warns() {
    let data = json!({ "u": { "x": "https://cdn.example/a.js" } });
    let result = run(
        "  props(u: U)\n  head {\n    script(src: u.x)\n    link(rel: \"stylesheet\", href: u.x)\n    meta(http-equiv: \"refresh\", content: \"0; url={u.x}\")\n  }\n  iframe(src: u.x, title: \"Map\")\n  div(\"v-on:click\": u.x, ng-click: u.x) \"x\"\n  div(style: \"width: {u.x}\") \"x\"\n  img(u.x, alt: \"Logo\")",
        data,
    );
    assert_eq!(codes(&result), ["SEC-004"; 6], "{:?}", result.diagnostics);
}
