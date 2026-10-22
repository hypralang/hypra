use hypra::{Compiled, compile};
use serde_json::json;

fn run(body: &str, extra: &str) -> Compiled {
    let src = format!(
        "allow(\"SEO-004\") \"no h1 in this test\" page(title: \"T\", description: \"A test page with a description long enough for search results.\") {{\n  props(user: User)\n{body}\n}}\n{extra}"
    );
    compile(&src, &json!({ "user": { "name": "Ana" } }), "p")
}

fn codes(result: &Compiled) -> Vec<&str> {
    result.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

const CARD: &str = r#"
component Card(title: Text, highlight: Bool = false) {
  article(class: "card") {
    h3 "{title}"
    if highlight { p "Staff pick" }
    slot
  }
}
"#;

#[test]
fn renders_params_defaults_and_slot_content() {
    let result =
        run("  Card(title: \"One\") { p \"Hello {user.name}\" }\n  Card(title: \"Two\", highlight: true)", CARD);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let html = result.output.unwrap().html;
    assert!(html.contains("<article class=\"card\">\n    <h3>One</h3>\n    <p>Hello Ana</p>\n  </article>"));
    assert!(html.contains("<h3>Two</h3>\n    <p>Staff pick</p>"));
    assert_eq!(html.matches("Staff pick").count(), 1);
}

#[test]
fn components_only_see_their_parameters() {
    let result = run("  Leaky()", "component Leaky() { p \"{user.name}\" }");
    assert_eq!(codes(&result), ["TYPE-002"]);
    assert_eq!(
        result.diagnostics[0].help.as_deref(),
        Some("components only see their parameters; pass it as an argument")
    );
}

#[test]
fn unknown_component_suggests_a_close_name() {
    let result = run("  Crad(title: \"x\")", CARD);
    assert_eq!(codes(&result), ["COMP-001"]);
    assert_eq!(result.diagnostics[0].help.as_deref(), Some("did you mean `Card`?"));
}

#[test]
fn arguments_are_checked_against_parameters() {
    let result = run("  Card(titel: \"x\")\n  Card(\"x\")", CARD);
    assert_eq!(codes(&result), ["COMP-002", "COMP-003", "COMP-002", "COMP-006"]);
    assert_eq!(result.diagnostics[1].help.as_deref(), Some("did you mean `title`?"));
}

#[test]
fn parameter_types_are_capitalized() {
    let result = run("", "component Bad(title: text) { p \"x\" }");
    assert_eq!(codes(&result), ["COMP-008"]);
}

#[test]
fn slot_outside_a_component_is_an_error() {
    assert_eq!(codes(&run("  slot", "")), ["SEM-006"]);
}

#[test]
fn component_names_must_be_capitalized_and_unique() {
    let result = run("", "component card() { p \"a\" }\ncomponent Card() { p \"a\" }\ncomponent Card() { p \"b\" }");
    assert_eq!(codes(&result), ["COMP-005", "COMP-004"]);
}

#[test]
fn headings_inside_components_count_where_they_render() {
    // the page's own h1 plus the component's h1 render as two
    let result = run("  h1 \"Page\"\n  Hero()", "component Hero() { h1 \"Hero\" }");
    assert_eq!(codes(&result), ["SEO-003"]);
    let result = run("  Hero()", "component Hero() { h1 \"Hero\" }");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn endless_recursion_is_stopped() {
    let result = run("  Loop()", "component Loop() { Loop() }");
    assert_eq!(codes(&result), ["COMP-007"]);
    assert!(result.output.is_none());
}

#[test]
fn text_after_a_call_is_its_slot_content() {
    let result = run("  Card(title: \"One\") \"Hello\"", CARD);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(result.output.unwrap().html.contains("<h3>One</h3>\n    Hello\n  </article>"));
}

#[test]
fn content_for_a_component_without_a_slot_is_an_error() {
    let icon = "component Icon(name: Text) { i(class: \"icon-{name}\", aria-hidden: true) }";
    assert_eq!(codes(&run("  button { Icon(name: \"x\") \"Close\" }", icon)), ["COMP-010"]);
    let result = run("  button {\n    Icon(name: \"x\")\n    \"Close\"\n  }", icon);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn parameters_and_arguments_are_named_once() {
    assert_eq!(codes(&run("  Twice(a: \"x\")", "component Twice(a: Text, a: Text) { p \"{a}\" }")), ["COMP-008"]);
    assert_eq!(codes(&run("  Card(title: \"x\", title: \"y\")", CARD)), ["ATTR-005"]);
}

#[test]
fn defaults_never_read_the_callers_data() {
    let result = run("  Hello()", "component Hello(who: Text = user.name) { p \"{who}\" }");
    assert_eq!(codes(&result), ["TYPE-002"]);
}

#[test]
fn html_attributes_go_to_the_root_element() {
    let src = format!(
        "allow(\"SEO-004\") \"no h1 in this test\" page(title: \"T\", robots: \"noindex\") {{\n  props(items: List)\n  for i in items {{ Card(title: \"x\", class: \"mt-{{i}}\", id: \"card-{{i}}\", data-kind: \"shoe\") }}\n}}\n{CARD}"
    );
    let result = compile(&src, &json!({ "items": [1, 2] }), "p");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let html = result.output.unwrap().html;
    assert!(html.contains("<article class=\"card mt-1\" id=\"card-1\" data-kind=\"shoe\">"), "{html}");
    assert!(html.contains("<article class=\"card mt-2\" id=\"card-2\" data-kind=\"shoe\">"), "{html}");
}

#[test]
fn attributes_need_a_single_root_and_typos_are_still_caught() {
    let two = "component Two() {\n  p \"a\"\n  p \"b\"\n}";
    assert_eq!(codes(&run("  Two(class: \"x\")", two)), ["COMP-003"]);
    assert_eq!(codes(&run("  Card(titel: \"x\")", CARD)), ["COMP-002", "COMP-003"]);
}

#[test]
fn components_bring_their_head_once() {
    let island =
        "component Map(lat: Text) {\n  head { script(\"/map.js\", type: \"module\") }\n  map-view(lat: lat)\n}";
    let result = run("  Map(lat: \"52.5\")\n  Map(lat: \"48.1\")", island);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let html = result.output.unwrap().html;
    let head = &html[..html.find("</head>").unwrap()];
    assert_eq!(head.matches("<script src=\"/map.js\" type=\"module\"></script>").count(), 1, "{html}");
    assert_eq!(html.matches("<map-view").count(), 2);
}

const PANEL: &str = r#"
component Panel(title: Text) {
  section {
    header {
      h2 "{title}"
      slot("actions") { a("/help") "Help" }
    }
    slot
  }
}
"#;

#[test]
fn named_slots_take_their_own_content_or_show_a_fallback() {
    let result = run(
        "  Panel(title: \"Orders\") {\n    p \"Recent orders of {user.name}\"\n    slot(\"actions\") { a(\"/orders/new\") \"New order\" }\n  }\n  Panel(title: \"Help\") { p \"No actions given\" }",
        PANEL,
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let html = result.output.unwrap().html;
    assert!(html.contains("<h2>Orders</h2>\n      <a href=\"/orders/new\">New order</a>\n    </header>\n    <p>Recent orders of Ana</p>"), "{html}");
    assert!(
        html.contains("<h2>Help</h2>\n      <a href=\"/help\">Help</a>\n    </header>\n    <p>No actions given</p>"),
        "{html}"
    );
}

#[test]
fn named_slots_must_exist_and_have_names() {
    assert_eq!(codes(&run("  Panel(title: \"x\") { slot(\"footer\") { p \"x\" } }", PANEL)), ["COMP-010"]);
    assert_eq!(codes(&run("  Panel(title: \"x\") { slot { p \"x\" } }", PANEL)), ["SEM-006"]);
}
