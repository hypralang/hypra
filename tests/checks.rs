//! The SEO, accessibility and security checks: each warns where it should and stays quiet otherwise.

use hypra::{Compiled, compile, php_with};
use serde_json::{Value as Json, json};

const DESCRIPTION: &str = "A test page with a description long enough for search results.";

/// A valid page with an h1, so only the rule under test can fire.
fn run(body: &str) -> Compiled {
    run_with(body, "", json!({}))
}

fn run_with(body: &str, extra: &str, data: Json) -> Compiled {
    let src =
        format!("page(title: \"T\", description: \"{DESCRIPTION}\") {{\n  h1 \"Main topic\"\n{body}\n}}\n{extra}");
    compile(&src, &data, "p")
}

fn codes(result: &Compiled) -> Vec<&str> {
    result.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

#[test]
fn a_page_without_h1_warns() {
    let result =
        compile(&format!("page(title: \"T\", description: \"{DESCRIPTION}\") {{\n  p \"x\"\n}}\n"), &json!({}), "p");
    assert_eq!(codes(&result), ["SEO-004"]);
}

#[test]
fn an_h1_from_a_component_or_layout_counts() {
    let component = format!(
        "page(title: \"T\", description: \"{DESCRIPTION}\") {{\n  Hero()\n}}\ncomponent Hero() {{ h1 \"Hi\" }}\n"
    );
    assert_eq!(codes(&compile(&component, &json!({}), "p")), Vec::<&str>::new());
    let layout = format!(
        "page(layout: Base, title: \"T\", description: \"{DESCRIPTION}\") {{\n  p \"x\"\n}}\nlayout Base {{ h1 \"Site\" main {{ slot }} }}\n"
    );
    assert_eq!(codes(&compile(&layout, &json!({}), "p")), Vec::<&str>::new());
}

#[test]
fn canonical_must_be_absolute() {
    let page = |canonical: &str| {
        format!("page(title: \"T\", description: \"{DESCRIPTION}\", canonical: \"{canonical}\") {{\n  h1 \"x\"\n}}\n")
    };
    assert_eq!(codes(&compile(&page("/shoes"), &json!({}), "p")), ["SEO-005"]);
    assert_eq!(codes(&compile(&page("https://shop.test/shoes"), &json!({}), "p")), Vec::<&str>::new());
}

#[test]
fn metadata_from_data_is_checked_when_built() {
    let src =
        "page(title: \"T\", description: \"{p.summary}\", canonical: \"{p.url}\") {\n  props(p: P)\n  h1 \"x\"\n}\n";
    let result = compile(src, &json!({ "p": { "summary": "Too short.", "url": "/relative" } }), "p");
    assert_eq!(codes(&result), ["SEO-006", "SEO-005"]);
    assert!(result.output.is_some(), "warnings don't stop the build");
}

#[test]
fn fixed_metadata_is_checked_without_data() {
    let src = format!("page(title: \"{}\", description: \"Short.\") {{\n  h1 \"x\"\n}}\n", "x".repeat(70));
    let result = php_with("p.hypra", &|_| Ok(src.clone()));
    let codes: Vec<&str> = result.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, ["SEO-002", "SEO-006"]);
    assert!(result.code.is_some());
}

#[test]
fn skipped_heading_levels_warn_even_across_components() {
    assert_eq!(codes(&run("  h2 \"Section\"\n  h4 \"Detail\"")), ["A11Y-006"]);
    let card = "component Card() { h3 \"Card\" }";
    assert_eq!(codes(&run_with("  h2 \"Section\"\n  Card()", card, json!({}))), Vec::<&str>::new());
    assert_eq!(codes(&run_with("  Card()", card, json!({}))), ["A11Y-006"]);
    assert_eq!(codes(&run("  h2 \"A\"\n  allow(\"A11Y-006\") \"design\"\n  h4 \"B\"")), Vec::<&str>::new());
}

#[test]
fn vague_link_text_warns() {
    assert_eq!(codes(&run("  a(\"/guide\") \"Click here\"\n  a(\"/more\") \"Read more…\"")), ["A11Y-007", "A11Y-007"]);
    assert_eq!(codes(&run("  a(\"/guide\") \"Read the setup guide\"")), Vec::<&str>::new());
}

#[test]
fn meaningless_alt_text_warns() {
    let result =
        run("  img(\"a.jpg\", alt: \"image\")\n  img(\"b.jpg\", alt: \"IMG_0042.JPG\")\n  img(\"c.jpg\", alt: \"\")");
    assert_eq!(codes(&result), ["A11Y-008", "A11Y-008", "A11Y-008"]);
    assert_eq!(
        result.diagnostics[2].help.as_deref(),
        Some("for an image without meaning, use decorative: true instead of an empty alt")
    );
    assert_eq!(codes(&run("  img(\"a.jpg\", alt: \"Blue running shoe, side view\")")), Vec::<&str>::new());
}

#[test]
fn iframe_needs_a_title() {
    assert_eq!(codes(&run("  iframe(\"https://maps.test/embed\")")), ["A11Y-009"]);
    assert_eq!(codes(&run("  iframe(\"https://maps.test/embed\", title: \"Map of our store\")")), Vec::<&str>::new());
}

#[test]
fn positive_tabindex_warns() {
    assert_eq!(codes(&run("  div(tabindex: 3) \"x\"")), ["A11Y-010"]);
    assert_eq!(codes(&run("  div(tabindex: 0) \"x\"\n  div(tabindex: -1) \"y\"")), Vec::<&str>::new());
}

#[test]
fn autoplaying_video_must_be_muted() {
    let video = |extra: &str| format!("  video(\"/a.mp4\", captions: \"/a.vtt\", autoplay: true{extra})");
    assert_eq!(codes(&run(&video(""))), ["A11Y-011"]);
    assert_eq!(codes(&run(&video(", muted: true"))), Vec::<&str>::new());
}

#[test]
fn duplicate_ids_warn_including_repeated_components() {
    assert_eq!(codes(&run("  div(id: \"news\") \"a\"\n  div(id: \"news\") \"b\"")), ["SEM-012"]);
    let banner = "component Banner() { div(id: \"banner\") \"Sale\" }";
    assert_eq!(codes(&run_with("  Banner()\n  Banner()", banner, json!({}))), ["SEM-012"]);
    assert_eq!(codes(&run_with("  Banner()", banner, json!({}))), Vec::<&str>::new());
}

#[test]
fn raw_html_from_data_warns() {
    let data = json!({ "c": { "body": "<b>hi</b>" } });
    assert_eq!(codes(&run_with("  props(c: C)\n  raw \"{c.body}\"", "", data)), ["SEC-002"]);
    assert_eq!(codes(&run("  raw \"<hr>\"")), Vec::<&str>::new());
}

#[test]
fn javascript_urls_are_errors() {
    let result = run("  a(\"javascript:alert(1)\") \"Go\"\n  a(href: \" JavaScript:void(0)\") \"Go\"");
    assert_eq!(codes(&result), ["SEC-003", "SEC-003"]);
    assert!(result.output.is_none());
}

#[test]
fn lang_must_be_a_language_code() {
    let page = |lang: &str| {
        format!(
            "page(title: \"T\", description: \"{DESCRIPTION}\", lang: \"{lang}\") {{\n  h1 \"x\"\n  p(lang: \"fr\") \"Bonjour\"\n}}\n"
        )
    };
    assert_eq!(codes(&compile(&page("English"), &json!({}), "p")), ["SEM-013"]);
    for valid in ["en", "de-AT", "pt-BR", "zh-Hant-TW"] {
        assert_eq!(codes(&compile(&page(valid), &json!({}), "p")), Vec::<&str>::new(), "{valid}");
    }
    let layout = format!(
        "page(layout: Base, title: \"T\", description: \"{DESCRIPTION}\") {{\n  h1 \"x\"\n}}\nlayout Base(lang: \"German\") {{ main {{ slot }} }}\n"
    );
    assert_eq!(codes(&compile(&layout, &json!({}), "p")), ["SEM-013"]);
}

#[test]
fn allow_with_an_unknown_rule_warns() {
    let result = run("  allow(\"SEO-03\") \"typo\"\n  h1 \"Second\"");
    assert_eq!(codes(&result), ["ALLOW-003", "SEO-003"]);
    assert_eq!(result.diagnostics[0].help.as_deref(), Some("did you mean `SEO-003`?"));
}

/// `diag::RULES` lists exactly the codes the compiler can report.
#[test]
fn every_rule_code_is_listed() {
    let mut used = std::collections::BTreeSet::new();
    for file in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src")).unwrap() {
        let path = file.unwrap().path();
        // diag.rs holds the list itself
        if path.ends_with("diag.rs") {
            continue;
        }
        let text = std::fs::read_to_string(path).unwrap();
        // a code is written as "KIND-123": uppercase letters or digits, a dash, three digits
        for (start, _) in text.match_indices('"') {
            let rest = &text[start + 1..];
            let Some(end) = rest.find(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')) else {
                continue;
            };
            let candidate = &rest[..end];
            let is_code = rest[end..].starts_with('"')
                && candidate.split_once('-').is_some_and(|(kind, number)| {
                    kind.len() >= 3 && number.len() == 3 && number.chars().all(|c| c.is_ascii_digit())
                });
            if is_code {
                used.insert(candidate.to_string());
            }
        }
    }
    let listed: std::collections::BTreeSet<String> = hypra::diag::RULES.iter().map(|(c, _)| c.to_string()).collect();
    assert_eq!(used, listed);
}

/// The spec documents every rule code, so the docs can't fall behind the compiler.
#[test]
fn every_rule_code_is_documented() {
    let spec = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/spec.md")).unwrap();
    let missing: Vec<&str> =
        hypra::diag::RULES.iter().map(|(code, _)| *code).filter(|code| !spec.contains(&format!("`{code}`"))).collect();
    assert!(missing.is_empty(), "not in docs/spec.md: {missing:?}");
}

#[test]
fn h1_counts_where_it_renders() {
    let data = json!({ "rows": ["a", "b"] });
    let in_loop = run_with("  props(rows: List)\n  for r in rows { h1 \"{r}\" }", "", data);
    assert_eq!(codes(&in_loop), ["SEO-003"]);
    let twice = run_with("  Hero()\n  Hero()", "component Hero() { h1 \"Hero\" }", json!({}));
    assert_eq!(codes(&twice), ["SEO-003", "SEO-003"]);
}

#[test]
fn recursive_components_are_reported_quickly() {
    let started = std::time::Instant::now();
    let tree = "component T(n: N) { if n.left { T(n: n.left) } if n.right { T(n: n.right) } }";
    let result = run_with("  props(root: N)\n  T(n: root)", tree, json!({ "root": {} }));
    assert_eq!(codes(&result), ["COMP-007", "COMP-007"]);
    assert!(started.elapsed().as_secs() < 2, "took {:?}", started.elapsed());
}

#[test]
fn pages_that_expand_without_bound_are_stopped() {
    let started = std::time::Instant::now();
    let mut extra = String::new();
    for i in 0..22 {
        extra.push_str(&format!("component C{i}() {{ C{}() C{}() }}\n", i + 1, i + 1));
    }
    extra.push_str("component C22() { p \"leaf\" }\n");
    let result = run_with("  C0()", &extra, json!({}));
    assert_eq!(codes(&result), ["COMP-009"]);
    assert!(started.elapsed().as_secs() < 10, "took {:?}", started.elapsed());
}

#[test]
fn aria_names_and_states_need_a_role_on_generic_elements() {
    let result = run("  div(aria-label: \"Filters\") \"x\"\n  span(aria-expanded: true) \"y\"");
    assert_eq!(codes(&result), ["A11Y-012", "A11Y-012"]);
    let fine = run(
        "  div(role: \"group\", aria-label: \"Filters\") \"x\"\n  nav(aria-label: \"Main\") { a(\"/\") \"Home\" }\n  div(aria-hidden: true) \"z\"",
    );
    assert_eq!(codes(&fine), Vec::<&str>::new());
}

#[test]
fn headings_and_ids_in_other_branches_dont_clash() {
    let src = "page(title: \"T\", description: \"A test page with a description long enough for search results.\") {\n  props(u: U)\n  if u.admin { h1(id: \"top\") \"Admin\" } else { h1(id: \"top\") \"Guest\" }\n  h2 \"Section\"\n  if u.x { h2 \"A\" } else { h4 \"B\" }\n}\n";
    let result = hypra::compile(src, &serde_json::json!({ "u": { "admin": true, "x": true } }), "p");
    let codes: Vec<&str> = result.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, ["A11Y-006"], "{:?}", result.diagnostics);
}

#[test]
fn a_fixed_id_in_a_loop_repeats() {
    let src = "page(title: \"T\", description: \"A test page with a description long enough for search results.\") {\n  props(u: U)\n  h1 \"x\"\n  for i in u.list { p(id: \"row\") \"{i}\" }\n}\n";
    let result = hypra::compile(src, &serde_json::json!({ "u": { "list": [1, 2] } }), "p");
    let codes: Vec<&str> = result.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, ["SEM-012"]);
}

#[test]
fn a_description_can_come_from_the_layout_or_be_left_out_of_search() {
    let layout = "layout Site(description: \"The shop for running shoes, with free returns and fast delivery.\") { main { slot } }\n";
    let src = format!("page(title: \"T\", layout: Site) {{\n  h1 \"x\"\n}}\n{layout}");
    let result = hypra::compile(&src, &serde_json::json!({}), "p");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(result.output.unwrap().html.contains("free returns"));

    let result =
        hypra::compile("page(title: \"Admin\", robots: \"noindex\") {\n  h1 \"x\"\n}\n", &serde_json::json!({}), "p");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let result = hypra::compile("page(title: \"Admin\") {\n  h1 \"x\"\n}\n", &serde_json::json!({}), "p");
    assert_eq!(result.diagnostics.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(), ["SEO-001"]);
}

#[test]
fn allow_before_the_page_covers_checks_on_data() {
    let src = "allow(\"SEO-006\") \"descriptions come from the CMS\"\npage(title: \"T\", description: \"{p.d}\") {\n  props(p: P)\n  h1 \"x\"\n}\n";
    let result = hypra::compile(src, &serde_json::json!({ "p": { "d": "Short" } }), "p");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn a_file_has_one_page() {
    let one = "page(title: \"T\", robots: \"noindex\") {\n  h1 \"x\"\n}\n";
    let result = hypra::compile(&format!("{one}{one}"), &serde_json::json!({}), "p");
    assert_eq!(result.diagnostics.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(), ["SEM-003"]);
}

#[test]
fn hypra_flags_are_fixed() {
    let src = "page(title: \"T\", robots: \"noindex\") {\n  props(p: P)\n  h1 \"x\"\n  img(\"/a.jpg\", alt: \"A red shoe\", priority: p.f)\n}\n";
    let result = hypra::compile(src, &serde_json::json!({ "p": { "f": true } }), "p");
    assert_eq!(result.diagnostics.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(), ["ATTR-003"]);
}

#[test]
fn buttons_and_links_can_be_named_in_other_ways() {
    let src = "page(title: \"T\", robots: \"noindex\") {\n  h1 \"x\"\n  span(id: \"l\") \"Close\"\n  button(aria-labelledby: \"l\")\n  button(title: \"Close\")\n  a(\"/guide\") { \"Read more\" }\n  a(\"/setup\", aria-label: \"Read the setup guide\") \"Read more\"\n}\n";
    let result = hypra::compile(src, &serde_json::json!({}), "p");
    assert_eq!(result.diagnostics.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(), ["A11Y-007"]);
}

#[test]
fn props_belong_directly_in_the_page() {
    let src = "page(title: \"T\", robots: \"noindex\") {\n  h1 \"x\"\n  main { props(u: U) }\n}\ncomponent C() { props(v: V) }\n";
    let result = hypra::compile(src, &serde_json::json!({}), "p");
    assert_eq!(result.diagnostics.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(), ["SEM-003", "SEM-003"]);
}

#[test]
fn common_mistakes_get_specific_help() {
    let page = |body: &str| {
        format!(
            "page(title: \"T\", robots: \"noindex\") {{\n  props(post: P)\n  h1 \"x\"\n{body}\n}}\ncomponent ProductCard(title: Text) {{ p \"{{title}}\" }}\n"
        )
    };
    let help = |body: &str| {
        let result = hypra::compile(&page(body), &serde_json::json!({ "post": { "title": "t" } }), "p");
        result.diagnostics.iter().filter_map(|d| d.help.clone()).collect::<Vec<_>>().join(" | ")
    };
    for (body, expected) in [
        ("  a(href: \"/x\", \"Label\")", "text goes after the parentheses"),
        ("  else { p \"x\" }", "`else` belongs right after the `}` of an if"),
        ("  h2 post.title", "data in text goes inside quotes: \"{post.title}\""),
        ("  p \"Hi\"\n  {", "`{` goes on the same line"),
        ("  button(@click: \"go()\") \"Go\"", "attribute names with characters like @ go in quotes"),
        (
            "  form(\"/f\", name: \"f\") {\n    field(\"e\", type: email, label: \"E\")\n    button \"Go\"\n  }",
            "types start with a capital letter: Email",
        ),
        ("  productCard(title: \"x\")", "components start with a capital letter: ProductCard"),
        ("  p \"{x}\"", "double the braces to write them as text: {{x}}"),
    ] {
        assert!(help(body).contains(expected), "{body}: {}", help(body));
    }
}

#[test]
fn a_missing_brace_points_at_the_block_it_belongs_to() {
    let src = "page(title: \"T\", robots: \"noindex\") {\n  h1 \"x\"\n  div {\n    p \"x\"\n  section {\n    p \"y\"\n  }\n}\n";
    let result = hypra::compile(src, &serde_json::json!({}), "p");
    assert_eq!(result.diagnostics[0].message, "this `{` is never closed");
    assert_eq!((result.diagnostics[0].pos.line, result.diagnostics[0].pos.col), (3, 7));
}

#[test]
fn allow_at_the_start_of_a_page_covers_the_whole_page() {
    let src = "page(title: \"Dashboard\", robots: \"noindex\") {\n  allow(\"SEO-004\") \"an app screen has no main heading\"\n  props(u: U)\n  h2 \"Stats\"\n}\n";
    let result = hypra::compile(src, &serde_json::json!({ "u": {} }), "p");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn data_in_text_is_located_past_escapes() {
    // `\t` and `\"` take two characters as written; a """ string points at its start
    let result = run("  p \"\\t\\\"{nope}\"\n  p \"\"\"\n    {nada}\n    \"\"\"");
    let at: Vec<(usize, usize)> = result.diagnostics.iter().map(|d| (d.pos.line, d.pos.col)).collect();
    assert_eq!(codes(&result), ["TYPE-002", "TYPE-002"]);
    assert_eq!(at, [(3, 10), (4, 5)]);
}
