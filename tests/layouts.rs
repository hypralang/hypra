use hypra::{Compiled, compile_with};
use serde_json::json;
use std::collections::HashMap;

/// Compiles `site/page.hypra` from an in-memory set of files.
fn run(files: &[(&str, &str)]) -> Compiled {
    let files: HashMap<String, String> = files.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect();
    compile_with("site/page.hypra", &json!({}), &|path| files.get(path).cloned().ok_or_else(|| "not found".into()))
}

fn codes(result: &Compiled) -> Vec<&str> {
    result.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

const LAYOUT: &str = r#"
layout Base(lang: "de", title_suffix: " | Site", site_name: "Site", description: "The default description every page of this site can inherit.") {
  header { p "Header" }
  main { slot }
}
"#;

#[test]
fn page_renders_inside_its_layout_with_its_settings() {
    // A site-wide description is a deliberate choice, so the page has to opt in.
    let page = "use \"layouts/base.hypra\"\nallow(\"SEO-001\") \"home page uses the site description\"\nallow(\"SEO-004\") \"no h1 in this test\" page(layout: Base, title: \"Home\") {\n  h1 \"Hello\"\n}\n";
    let result = run(&[("site/page.hypra", page), ("site/layouts/base.hypra", LAYOUT)]);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let html = result.output.unwrap().html;
    assert!(html.contains("<html lang=\"de\">"));
    assert!(html.contains("<title>Home | Site</title>"));
    assert!(html.contains("<meta property=\"og:title\" content=\"Home\">"));
    assert!(html.contains(
        "<meta name=\"description\" content=\"The default description every page of this site can inherit.\">"
    ));
    assert!(html.contains("<main>\n    <h1>Hello</h1>\n  </main>"));
}

#[test]
fn page_settings_override_layout_settings() {
    let page = "use \"layouts/base.hypra\"\nallow(\"SEO-004\") \"no h1 in this test\" page(layout: Base, title: \"A\", description: \"Our own description, long enough for search engines to show.\") {\n}\n";
    let html = run(&[("site/page.hypra", page), ("site/layouts/base.hypra", LAYOUT)]).output.unwrap().html;
    assert!(html.contains("content=\"Our own description, long enough for search engines to show.\""));
    assert!(!html.contains("inherit."));
}

#[test]
fn imports_resolve_relative_to_the_importing_file() {
    let page = "use \"components/a.hypra\"\nallow(\"SEO-004\") \"no h1 in this test\" page(title: \"T\", description: \"A test page with a description long enough for search results.\") {\n  A()\n}\n";
    let a = "use \"../shared/b.hypra\"\ncomponent A() { B() }";
    let b = "use \"../components/a.hypra\"\ncomponent B() { p \"from b\" }";
    let result = run(&[("site/page.hypra", page), ("site/components/a.hypra", a), ("site/shared/b.hypra", b)]);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(result.output.unwrap().html.contains("<p>from b</p>"));
}

#[test]
fn errors_point_into_the_imported_file() {
    let page = "use \"parts.hypra\"\nallow(\"SEO-004\") \"no h1 in this test\" page(title: \"T\", description: \"A test page with a description long enough for search results.\") {\n}\n";
    let parts = "component Pic() {\n  img(\"a.jpg\")\n}";
    let result = run(&[("site/page.hypra", page), ("site/parts.hypra", parts)]);
    assert_eq!(codes(&result), ["A11Y-001"]);
    let d = &result.diagnostics[0];
    assert_eq!((d.pos.file, d.pos.line), (1, 2));
    assert_eq!(result.sources[d.pos.file].path, "site/parts.hypra");
}

#[test]
fn missing_import_is_reported_at_the_use_statement() {
    let page = "use \"nope.hypra\"\nallow(\"SEO-004\") \"no h1 in this test\" page(title: \"T\", description: \"A test page with a description long enough for search results.\") {\n}\n";
    let result = run(&[("site/page.hypra", page)]);
    assert_eq!(codes(&result), ["IMPORT-001"]);
    assert_eq!(result.diagnostics[0].message, "can't read `nope.hypra`: not found");
    assert_eq!(result.diagnostics[0].pos.line, 1);
}

#[test]
fn unknown_layout_suggests_a_close_name() {
    let page = "use \"layouts/base.hypra\"\nallow(\"SEO-004\") \"no h1 in this test\" page(layout: Bsae, title: \"T\", description: \"A test page with a description long enough for search results.\") {\n}\n";
    let result = run(&[("site/page.hypra", page), ("site/layouts/base.hypra", LAYOUT)]);
    assert_eq!(codes(&result), ["LAYOUT-001"]);
    assert_eq!(result.diagnostics[0].help.as_deref(), Some("did you mean `Base`?"));
}

#[test]
fn unknown_layout_setting_suggests_a_close_name() {
    let page = "allow(\"SEO-004\") \"no h1 in this test\" page(layout: Base, title: \"T\", description: \"A test page with a description long enough for search results.\") {\n}\nlayout Base(langg: \"de\") { main { slot } }\n";
    let result = run(&[("site/page.hypra", page)]);
    assert_eq!(codes(&result), ["ATTR-001"]);
    assert_eq!(result.diagnostics[0].help.as_deref(), Some("did you mean `lang`?"));
}

#[test]
fn layout_without_slot_is_an_error() {
    let page = "allow(\"SEO-004\") \"no h1 in this test\" page(layout: Empty, title: \"T\", description: \"A test page with a description long enough for search results.\") {\n}\nlayout Empty { header { p \"x\" } }\n";
    assert_eq!(codes(&run(&[("site/page.hypra", page)])), ["LAYOUT-002"]);
}

#[test]
fn imported_files_cannot_contain_a_page() {
    let page = "use \"other.hypra\"\nallow(\"SEO-004\") \"no h1 in this test\" page(title: \"T\", description: \"A test page with a description long enough for search results.\") {\n}\n";
    let other = "allow(\"SEO-004\") \"no h1 in this test\" page(title: \"X\", description: \"A test page with a description long enough for search results.\") {\n}\n";
    assert_eq!(codes(&run(&[("site/page.hypra", page), ("site/other.hypra", other)])), ["SEM-007"]);
}

#[test]
fn files_of_components_and_layouts_check_on_their_own_but_dont_build() {
    let library = "use \"layouts/base.hypra\"\ncomponent Card(title: Text) { img(\"/a.jpg\") h2 \"{title}\" }\n";
    let load = |path: &str| match path {
        "site/card.hypra" => Ok(library.to_string()),
        "site/layouts/base.hypra" => Ok(LAYOUT.to_string()),
        _ => Err("not found".to_string()),
    };
    // a check reports what's wrong inside the components, and no missing page
    let checked = hypra::check_with("site/card.hypra", &load);
    assert_eq!(codes(&checked), ["A11Y-001"]);
    // a build of a file without mistakes says there's nothing to build
    let clean = |_: &str| Ok("component Card(title: Text) { h2 \"{title}\" }\n".to_string());
    let built = compile_with("site/card.hypra", &json!({}), &clean);
    assert_eq!(codes(&built), ["SEM-002"]);
    assert!(built.diagnostics[0].message.contains("no page to build"));
    // a file with neither still needs a page
    let empty = hypra::check_with("site/empty.hypra", &|_| Ok(String::new()));
    assert_eq!(codes(&empty), ["SEM-002"]);
}

const NESTED: &str = r#"
layout Site(lang: "de", title_suffix: " | Site", site_name: "Site") {
  head { link(rel: "stylesheet", href: "/site.css") }
  body(class: "site")
  header { p "Site header" }
  main { slot }
}

layout Docs(layout: Site, title_suffix: " · Docs") {
  head { link(rel: "stylesheet", href: "/docs.css") }
  div(class: "docs") {
    aside { p "Docs menu" }
    article { slot }
  }
}
"#;

#[test]
fn layouts_sit_inside_layouts() {
    let src = format!("page(layout: Docs, title: \"Install\", robots: \"noindex\") {{\n  h1 \"Install\"\n}}\n{NESTED}");
    let result = hypra::compile(&src, &serde_json::json!({}), "p");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let html = result.output.unwrap().html;
    assert!(html.contains("<html lang=\"de\">"), "{html}");
    assert!(html.contains("<title>Install · Docs</title>"), "{html}");
    assert!(html.find("/site.css").unwrap() < html.find("/docs.css").unwrap(), "{html}");
    assert!(html.contains("<body class=\"site\">"), "{html}");
    assert!(html.contains("<main>\n    <div class=\"docs\">\n      <aside>\n        <p>Docs menu</p>\n      </aside>\n      <article>\n        <h1>Install</h1>"), "{html}");
}

#[test]
fn a_layout_cant_end_up_inside_itself() {
    let src = "page(layout: A, title: \"T\", robots: \"noindex\") {\n  h1 \"x\"\n}\nlayout A(layout: B) { main { slot } }\nlayout B(layout: A) { div { slot } }\n";
    let result = hypra::compile(src, &serde_json::json!({}), "p");
    let codes: Vec<&str> = result.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, ["LAYOUT-005", "LAYOUT-005"]);
}
