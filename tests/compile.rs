use hypra::{compile, compile_file, diag::Diagnostic};
use serde_json::{Value as Json, json};

fn product_data() -> Json {
    serde_json::from_str(include_str!("../examples/product.json")).unwrap()
}

fn codes(diags: &[Diagnostic]) -> Vec<&str> {
    diags.iter().map(|d| d.code.as_str()).collect()
}

/// Wraps body content in a page that passes all page-level checks.
fn page(body: &str) -> String {
    format!(
        "allow(\"SEO-004\") \"no h1 in this test\" page(title: \"T\", description: \"A test page with a description long enough for search results.\") {{\n{body}\n}}\n"
    )
}

fn html(body: &str, data: Json) -> String {
    let result = compile(&page(body), &data, "p");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.output.unwrap().html
}

#[test]
fn product_example_builds_all_outputs() {
    let result = compile_file("examples/product.hypra", &product_data());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let out = result.output.unwrap();

    assert!(out.html.contains("<title>Runner X – Running Shoes | Shop</title>"));
    assert!(out.html.contains("<link rel=\"stylesheet\" href=\"/app.css\">"));
    assert!(out.html.contains("alt=\"Runner X, side view\""));
    assert!(out.html.contains("toolname=\"cart.add\""));
    assert!(out.html.contains("<input type=\"number\" name=\"stars\" min=\"1\" max=\"5\" required>"));
    assert!(out.markdown.contains("- `cart.add(id: 42)`: Add to cart → POST /_a/cart.add"));
    assert_eq!(out.webmcp[1]["inputSchema"]["required"], json!(["stars", "email"]));
    assert_eq!(out.json["actions"][0]["endpoint"], "/_a/cart.add");
}

#[test]
fn data_is_html_escaped() {
    let out = compile_file("examples/product.hypra", &product_data()).output.unwrap();
    assert!(out.html.contains("Light &amp; comfortable &lt;3"));
    assert!(!out.html.contains("<3"));
}

#[test]
fn broken_example_reports_every_problem_and_no_output() {
    let result = compile_file("examples/broken.hypra", &json!({}));
    assert!(result.output.is_none());
    assert_eq!(codes(&result.diagnostics), ["SEO-001", "A11Y-001", "SEO-003", "SEM-001", "FORM-002", "TYPE-004"]);
    assert_eq!(result.diagnostics[3].help.as_deref(), Some("did you mean `button`?"));
}

#[test]
fn any_html_element_and_attribute_passes_through() {
    let out = html(
        "  div(class: \"card p-6 md:flex\", data-test: \"x\", \"x-on:click\": \"open = true\", hidden: true, inert: false) \"Hi\"\n  model-viewer(src: \"a.glb\", ar: true, camera-controls: true)",
        json!({}),
    );
    assert!(out.contains("<div class=\"card p-6 md:flex\" data-test=\"x\" x-on:click=\"open = true\" hidden>Hi</div>"));
    assert!(out.contains("<model-viewer src=\"a.glb\" ar camera-controls></model-viewer>"));
}

#[test]
fn unknown_attributes_are_written_with_a_warning() {
    let result = compile(&page("  div(clas: \"x\", frobnicate: \"menu\") \"Hi\""), &json!({}), "p");
    assert_eq!(codes(&result.diagnostics), ["ATTR-001", "ATTR-001"]);
    assert!(result.diagnostics.iter().all(|d| !d.is_error()));
    assert_eq!(
        result.diagnostics[0].help.as_deref(),
        Some("did you mean `class`? If not, quote the name: \"clas\": ...")
    );
    assert_eq!(result.diagnostics[1].help.as_deref(), Some("quote the name if it's meant: \"frobnicate\": ..."));
    assert!(result.output.unwrap().html.contains("<div clas=\"x\" frobnicate=\"menu\">Hi</div>"));
}

#[test]
fn a_near_miss_of_a_hypra_setting_is_an_error() {
    let result = compile(&page("  img(\"/a.png\", alt: \"A red shoe\", decortive: true)"), &json!({}), "p");
    assert!(result.diagnostics[0].is_error(), "{:?}", result.diagnostics);
    assert_eq!(result.diagnostics[0].help.as_deref(), Some("did you mean `decorative`?"));
}

#[test]
fn pages_set_html_and_body_attributes_and_replace_head_tags() {
    let src = "page(title: \"T\", robots: \"noindex\", dir: \"rtl\", class: \"dark\", layout: Site) {\n  body(class: \"page\")\n  head {\n    meta(name: \"viewport\", content: \"width=device-width, viewport-fit=cover\")\n    meta(name: \"twitter:card\", content: \"summary_large_image\")\n  }\n  h1 \"x\"\n}\nlayout Site {\n  body(class: \"site\", hx-boost: true)\n  main { slot }\n}\n";
    let result = compile(src, &json!({}), "p");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let html = result.output.unwrap().html;
    assert!(html.starts_with("<!doctype html>\n<html lang=\"en\" dir=\"rtl\" class=\"dark\">"), "{html}");
    assert!(html.contains("<body hx-boost=\"true\" class=\"page\">\n"), "{html}");
    assert_eq!(html.matches("name=\"viewport\"").count(), 1, "{html}");
    assert!(html.contains("viewport-fit=cover"));
    assert_eq!(html.matches("twitter:card").count(), 1, "{html}");
}

#[test]
fn text_with_inline_elements_stays_on_one_line() {
    let out = compile(
        &page("  p { \"Weighs only \" b \"220 g\" \", see \" a(\"/specs\") \"specs\" \".\" }"),
        &json!({}),
        "p",
    );
    let out = out.output.unwrap();
    assert!(out.html.contains("<p>Weighs only <b>220 g</b>, see <a href=\"/specs\">specs</a>.</p>"));
    assert!(out.markdown.contains("Weighs only **220 g**, see [specs](/specs)."));
}

#[test]
fn head_takes_scripts_styles_and_any_meta() {
    let out = html(
        "  head {\n    script(\"/app.js\", type: \"module\")\n    meta(name: \"theme-color\", content: \"#3b5bdb\")\n  }\n  p \"Hi\"",
        json!({}),
    );
    let head = &out[..out.find("</head>").unwrap()];
    assert!(head.contains("<script src=\"/app.js\" type=\"module\"></script>"));
    assert!(head.contains("<meta name=\"theme-color\" content=\"#3b5bdb\">"));
}

#[test]
fn head_rejects_body_content() {
    let result = compile(&page("  head { p \"Hi\" }"), &json!({}), "p");
    assert_eq!(codes(&result.diagnostics), ["SEM-011"]);
}

#[test]
fn raw_html_is_written_unescaped() {
    let out = html("  raw \"<marquee>Sale</marquee>\"", json!({}));
    assert!(out.contains("<marquee>Sale</marquee>"));
}

#[test]
fn conditions_support_else_and_not() {
    let body = "  props(user: User)\n  if user.admin { p \"Admin\" } else { p \"Guest\" }\n  if not user.admin { p \"Not admin\" }";
    let out = html(body, json!({ "user": { "admin": false } }));
    assert!(out.contains("<p>Guest</p>") && out.contains("<p>Not admin</p>") && !out.contains("<p>Admin</p>"));
}

#[test]
fn boolean_attributes_can_come_from_data() {
    let body = "  props(user: User)\n  input(type: \"checkbox\", name: \"admin\", checked: user.admin)";
    assert!(
        html(body, json!({ "user": { "admin": true } })).contains("<input type=\"checkbox\" name=\"admin\" checked>")
    );
    assert!(html(body, json!({ "user": { "admin": false } })).contains("<input type=\"checkbox\" name=\"admin\">"));
}

#[test]
fn names_are_capitalized_and_text_is_quoted() {
    let result = compile(
        "allow(\"SEO-004\") \"no h1 in this test\" page(title: \"T\", description: \"A test page with a description long enough for search results.\", layout: shop, lang: en) {\n}\n",
        &json!({}),
        "p",
    );
    assert_eq!(codes(&result.diagnostics), ["ATTR-003"]);
    assert_eq!(result.diagnostics[0].message, "expected a name, e.g. Shop");

    let result = compile(
        "allow(\"SEO-004\") \"no h1 in this test\" page(title: \"T\", description: \"A test page with a description long enough for search results.\", lang: en) {\n}\n",
        &json!({}),
        "p",
    );
    assert_eq!(codes(&result.diagnostics), ["TYPE-002"]);
    assert_eq!(
        result.diagnostics[0].help.as_deref(),
        Some("declare it with props(en: ...), or quote it if it's text: \"en\"")
    );
}

#[test]
fn void_elements_cannot_have_children() {
    let result = compile(&page("  img(\"a.jpg\", alt: \"A\") { p \"no\" }"), &json!({}), "p");
    assert_eq!(codes(&result.diagnostics), ["SEM-008"]);
}

#[test]
fn title_belongs_to_the_page() {
    let result = compile(&page("  title \"Hi\""), &json!({}), "p");
    assert_eq!(codes(&result.diagnostics), ["SEM-009"]);
    assert_eq!(result.diagnostics[0].help.as_deref(), Some("page(title: \"...\")"));
}

#[test]
fn inline_event_handlers_are_a_warning() {
    let result = compile(&page("  button(onclick: \"go()\") \"Go\""), &json!({}), "p");
    assert_eq!(codes(&result.diagnostics), ["SEC-001"]);
    assert!(result.output.unwrap().html.contains("<button onclick=\"go()\">Go</button>"));
}

#[test]
fn data_typo_suggests_the_right_field() {
    let result = compile(&page("  props(product: Product)\n  h1 \"{product.nmae}\""), &product_data(), "p");
    assert_eq!(codes(&result.diagnostics), ["TYPE-002"]);
    assert_eq!(result.diagnostics[0].message, "`product` has no field `nmae`");
    assert_eq!(result.diagnostics[0].help.as_deref(), Some("did you mean `name`?"));
    assert!(result.output.is_none());
}

#[test]
fn undeclared_data_is_a_compile_error() {
    let result = compile(&page("  props(product: Product)\n  h1 \"{prodcut.name}\""), &product_data(), "p");
    assert_eq!(codes(&result.diagnostics), ["TYPE-002"]);
    assert_eq!(result.diagnostics[0].help.as_deref(), Some("did you mean `product`?"));
}

#[test]
fn data_errors_inside_loops_are_reported_once() {
    let src = page("  props(product: Product)\n  for r in product.reviews { p \"{r.rating}\" }");
    let result = compile(&src, &product_data(), "p");
    assert_eq!(codes(&result.diagnostics), ["TYPE-002"]);
    assert_eq!(result.diagnostics[0].message, "`r` has no field `rating`");
}

#[test]
fn missing_prop_is_reported() {
    let result = compile(&page("  props(product: Product)\n  h1 \"Hi\""), &json!({}), "p");
    assert_eq!(codes(&result.diagnostics), ["TYPE-001"]);
}

#[test]
fn allow_with_reason_suppresses_one_rule_for_the_next_element() {
    let src = page("  h1 \"Run\"\n  allow(\"SEO-003\") \"hero split needs two h1\"\n  h1 \"faster\"\n  h1 \"again\"");
    let result = compile(&src, &json!({}), "p");
    // the second h1 is allowed, the third is not
    assert_eq!(codes(&result.diagnostics), ["SEO-003"]);
    assert_eq!(result.diagnostics[0].pos.line, 5);
}

#[test]
fn allow_without_reason_is_an_error() {
    let result = compile(&page("  allow(\"SEO-003\")\n  h1 \"A\""), &json!({}), "p");
    assert_eq!(codes(&result.diagnostics), ["ALLOW-001"]);
}

#[test]
fn decorative_images_need_no_alt() {
    let out = compile(&page("  img(\"bg.jpg\", decorative: true)"), &json!({}), "p").output.unwrap();
    assert!(out.html.contains("<img src=\"bg.jpg\" alt=\"\" loading=\"lazy\">"));
    assert!(!out.markdown.contains("bg.jpg"));
}

#[test]
fn long_title_is_a_warning_not_an_error() {
    let src = format!(
        "allow(\"SEO-004\") \"no h1 in this test\" page(title: \"{}\", description: \"A test page with a description long enough for search results.\") {{\n}}\n",
        "x".repeat(70)
    );
    let result = compile(&src, &json!({}), "p");
    assert_eq!(codes(&result.diagnostics), ["SEO-002"]);
    assert!(result.output.is_some());
}

#[test]
fn robots_noindex_is_rendered() {
    let out = compile(
        "allow(\"SEO-004\") \"no h1 in this test\" page(title: \"Checkout\", description: \"A test page with a description long enough for search results.\", robots: \"noindex\") {\n}\n",
        &json!({}),
        "p",
    );
    assert!(out.output.unwrap().html.contains("<meta name=\"robots\" content=\"noindex\">"));
}

#[test]
fn unknown_page_settings_suggest_the_right_one() {
    let result = compile(
        "allow(\"SEO-004\") \"no h1 in this test\" page(title: \"T\", descripton: \"D\") {\n}\n",
        &json!({}),
        "p",
    );
    assert_eq!(codes(&result.diagnostics), ["SEO-001", "ATTR-001"]);
    assert_eq!(result.diagnostics[1].help.as_deref(), Some("did you mean `description`?"));
}

#[test]
fn syntax_errors_point_at_the_token() {
    let result = compile(
        "allow(\"SEO-004\") \"no h1 in this test\" page(title: \"T\", description: \"A test page with a description long enough for search results.\") {\n  p \"unclosed\n}",
        &json!({}),
        "p",
    );
    assert_eq!(codes(&result.diagnostics), ["LEX-002"]);
    assert_eq!((result.diagnostics[0].pos.line, result.diagnostics[0].pos.col), (2, 5));
}

#[test]
fn double_braces_are_literal() {
    let out = html("  props(user: User)\n  p \"Write {{name}} to show {user}\"", json!({ "user": "Ana" }));
    assert!(out.contains("<p>Write {name} to show Ana</p>"));
}

#[test]
fn braces_around_code_are_text() {
    let out = html("  div(x-data: \"{ open: false }\", data-json: \"{\\\"a\\\": 1}\") \"x\"", json!({}));
    assert!(out.contains("<div x-data=\"{ open: false }\" data-json=\"{&quot;a&quot;: 1}\">x</div>"));
}

#[test]
fn numbers_are_written_as_in_the_source() {
    let out = html("  bakery-map(lat: 52.52, lng: 13.40, zip: 01067, zoom: 12)", json!({}));
    assert!(out.contains("<bakery-map lat=\"52.52\" lng=\"13.40\" zip=\"01067\" zoom=\"12\"></bakery-map>"));
}

#[test]
fn tables_become_markdown_tables() {
    let src = page(
        "  props(orders: List)\n  table {\n    thead { tr { th \"Order\"  th \"Total\" } }\n    tbody { for o in orders { tr { td \"{o.id}\"  td \"{o.total}\" } } }\n  }",
    );
    let md =
        compile(&src, &json!({ "orders": [{ "id": "A-1", "total": "9.50" }, { "id": "A-2", "total": "3.00" }] }), "p")
            .output
            .unwrap()
            .markdown;
    assert!(md.contains("| Order | Total |\n| --- | --- |\n| A-1 | 9.50 |\n| A-2 | 3.00 |\n"), "{md}");
}

#[test]
fn only_boolean_attributes_drop_their_value() {
    let out = html(
        "  props(m: M)\n  button(aria-expanded: false, aria-hidden: true, hx-boost: true, disabled: true, hidden: false) \"x\"\n  button(aria-expanded: m.open, hidden: m.open) \"y\"\n  model-viewer(ar: true)",
        json!({ "m": { "open": false } }),
    );
    assert!(out.contains("<button aria-expanded=\"false\" aria-hidden=\"true\" hx-boost=\"true\" disabled>x</button>"));
    assert!(out.contains("<button aria-expanded=\"false\">y</button>"));
    assert!(out.contains("<model-viewer ar></model-viewer>"));
}

#[test]
fn main_arguments_can_also_be_named() {
    let src = page(
        "  allow(rule: \"SEO-003\") \"named rule\"\n  h1 \"Second\"\n  action(name: \"cart.add\", id: 1) \"Add\"\n  action(id: 2, name: \"cart.remove\") \"Remove\"\n  form(\"/s\", name: \"s\") {\n    field(name: \"q\", type: String, label: \"Search\")\n    button \"Go\"\n  }",
    );
    let result = compile(&src, &json!({}), "p");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let html = result.output.unwrap().html;
    assert!(html.contains("action=\"/_a/cart.add\""));
    assert!(html.contains("action=\"/_a/cart.remove\"") && html.contains("name=\"id\" value=\"2\""));
    assert!(html.contains("<input type=\"text\" name=\"q\" required>"));
}

#[test]
fn names_needed_at_compile_time_cannot_come_from_data() {
    let src = page(
        "  props(cfg: C)\n  action(cfg.act) \"Go\"\n  form(\"/x\", name: cfg.form) {\n    field(cfg.field, type: String, label: \"L\")\n    button \"Go\"\n  }",
    );
    let result = compile(&src, &json!({ "cfg": {} }), "p");
    assert_eq!(codes(&result.diagnostics), ["ATTR-003", "ATTR-003", "ATTR-003"]);
    assert_eq!(result.diagnostics[0].message, "expected fixed text in quotes, without data");
}

#[test]
fn fields_cannot_hide_in_loops_or_conditions() {
    let src = page(
        "  props(u: U)\n  form(\"/x\", name: \"f\") {\n    if u.admin { field(\"role\", type: String, label: \"Role\") }\n    button \"Go\"\n  }",
    );
    assert_eq!(codes(&compile(&src, &json!({ "u": {} }), "p").diagnostics), ["FORM-005"]);
}

#[test]
fn markdown_keeps_links_bullets_and_quotes_front_matter() {
    let src = "allow(\"SEO-004\") \"md test\" page(title: \"T\", description: \"A test page with a description long enough for search results.\", canonical: \"https://x.test/a: b\") {\n  section { a(\"/about\") \"About us\" }\n  ul { li { a(\"/x\") \"X\" } }\n  video(\"/v.mp4\", captions: \"/v.vtt\") \"Your browser can't play this video.\"\n}\n";
    let out = compile(src, &json!({}), "p").output.unwrap();
    assert!(out.markdown.contains("canonical: \"https://x.test/a: b\""));
    assert!(out.markdown.contains("[About us](/about)"));
    assert!(out.markdown.contains("- [X](/x)"));
    assert!(out.html.contains("Your browser can't play this video."));
}

#[test]
fn strings_know_four_escapes_and_reject_the_rest() {
    let out = html("  p \"tab\\there \\\"quoted\\\" back\\\\slash\"", json!({}));
    assert!(out.contains("<p>tab\there &quot;quoted&quot; back\\slash</p>"));
    let result = compile(&page("  p \"C:\\path\""), &json!({}), "p");
    assert_eq!(codes(&result.diagnostics), ["LEX-003"]);
}

#[test]
fn required_choices_start_empty() {
    let src = page(
        "  form(\"/x\", name: \"f\") {\n    field(\"plan\", type: Choice(\"free\", \"pro\"), label: \"Plan\")\n    button \"Go\"\n  }",
    );
    let out = html_with(&src);
    assert!(out.contains(
        "<select name=\"plan\" required><option value=\"\">Plan</option><option value=\"free\">free</option>"
    ));
}

fn html_with(src: &str) -> String {
    compile(src, &json!({}), "p").output.unwrap().html
}

#[test]
fn text_after_a_void_element_is_next_to_it() {
    let out = html(
        "  p { \"Line one\" br \"Line two\" }\n  label { input(type: \"checkbox\", name: \"t\") \" Terms\" }",
        json!({}),
    );
    assert!(out.contains("<p>Line one<br>Line two</p>"), "{out}");
    assert!(out.contains("<input type=\"checkbox\" name=\"t\"> Terms</label>"), "{out}");
}

#[test]
fn fields_can_sit_in_fieldsets_and_wrappers() {
    let src = page(
        "  form(\"/signup\", name: \"signup\") {\n    fieldset {\n      legend \"You\"\n      div(class: \"grid\") { field(\"email\", type: Email, label: \"Email\") }\n    }\n    button \"Go\"\n  }",
    );
    let result = compile(&src, &json!({}), "p");
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let out = result.output.unwrap();
    assert!(out.html.contains("<fieldset>\n      <legend>You</legend>\n      <div class=\"grid\">"), "{}", out.html);
    assert_eq!(out.json["actions"][0]["params"]["email"]["format"], "email");
}

#[test]
fn dialog_forms_and_unnamed_forms_are_plain_html() {
    let out = html(
        "  dialog {\n    form(method: \"dialog\") { button \"Close\" }\n  }\n  form(\"/search\", method: \"get\") {\n    input(type: \"search\", name: \"q\", aria-label: \"Search\")\n    button \"Search\"\n  }",
        json!({}),
    );
    assert!(out.contains("<form method=\"dialog\">\n      <button>Close</button>\n    </form>"), "{out}");
    assert!(out.contains("<form method=\"get\" action=\"/search\">\n"), "{out}");
    assert!(!out.contains("toolname") && !out.contains("_method"), "{out}");
}

#[test]
fn form_methods_are_checked() {
    let result = compile(&page("  form(\"/x\", method: \"fetch\") { button \"Go\" }"), &json!({}), "p");
    assert_eq!(codes(&result.diagnostics), ["FORM-006"]);
}

#[test]
fn a_form_with_fields_and_no_name_warns() {
    let src = page("  form(\"/x\") {\n    field(\"q\", type: String, label: \"Query\")\n    button \"Go\"\n  }");
    assert_eq!(codes(&compile(&src, &json!({}), "p").diagnostics), ["FORM-004"]);
}

#[test]
fn submit_buttons_count_in_inputs_and_components() {
    let src = format!(
        "{}component Submit() {{ button \"Send\" }}\n",
        page(
            "  form(\"/a\") { input(type: \"submit\", value: \"Go\") }\n  form(\"/b\") { Submit() }\n  form(\"/c\") { button(type: \"button\") \"Menu\" }"
        )
    );
    assert_eq!(codes(&compile(&src, &json!({}), "p").diagnostics), ["FORM-003"]);
}

#[test]
fn forms_and_actions_cannot_nest() {
    let src =
        page("  form(\"/a\") {\n    action(\"cart.add\") \"Add\"\n    button \"Go\"\n  }\n  p { action(\"x\") \"X\" }");
    assert_eq!(codes(&compile(&src, &json!({}), "p").diagnostics), ["SEM-003", "SEM-003"]);
}

#[test]
fn head_cannot_hide_in_conditions() {
    let src = page("  props(u: U)\n  if u.x { head { meta(name: \"x\", content: \"y\") } }");
    assert_eq!(codes(&compile(&src, &json!({ "u": { "x": true } }), "p").diagnostics), ["SEM-010"]);
}

#[test]
fn videos_take_sources_and_muted_videos_need_no_captions() {
    let out = html(
        "  video(captions: \"/c.vtt\") {\n    source(src: \"/a.webm\", type: \"video/webm\")\n    source(src: \"/a.mp4\", type: \"video/mp4\")\n  }\n  video(\"/bg.mp4\", muted: true, autoplay: true, loop: true, playsinline: true, controls: false)",
        json!({}),
    );
    assert!(
        out.contains(
            "<video controls>\n    <track kind=\"captions\" src=\"/c.vtt\" default>\n    <source src=\"/a.webm\""
        ),
        "{out}"
    );
    assert!(out.contains("<video src=\"/bg.mp4\" muted autoplay loop playsinline>\n  </video>"), "{out}");
    let result = compile(&page("  video(\"/talk.mp4\")"), &json!({}), "p");
    assert_eq!(codes(&result.diagnostics), ["A11Y-003"]);
}

#[test]
fn field_types_take_bounds_options_and_files() {
    let result = compile(
        &page(
            "  props(u: U)\n  form(\"/p\", name: \"profile\") {\n    field(\"age\", type: Int(min: 18), label: \"Age\")\n    field(\"price\", type: Number(0..9.99), label: \"Price\")\n    field(\"bio\", type: Text(max: 500), label: \"Bio\", optional: true)\n    field(\"phone\", type: Tel, label: \"Phone\", optional: true)\n    field(\"day\", type: Date(min: \"2025-01-01\"), label: \"Day\")\n    field(\"country\", type: Choice(de: \"Germany\", at: \"Austria\"), label: \"Country\", default: u.country)\n    field(\"cv\", type: File, label: \"CV\", accept: \".pdf\")\n    button \"Save\"\n  }",
        ),
        &json!({ "u": { "country": "at" } }),
        "p",
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let out = result.output.unwrap();
    for part in [
        "<form method=\"post\" action=\"/p\" enctype=\"multipart/form-data\" toolname=\"profile\"",
        "<input type=\"number\" name=\"age\" min=\"18\" required>",
        "<input type=\"number\" name=\"price\" min=\"0\" max=\"9.99\" step=\"any\" required>",
        "<textarea name=\"bio\" maxlength=\"500\"></textarea>",
        "<input type=\"tel\" name=\"phone\">",
        "<input type=\"date\" name=\"day\" min=\"2025-01-01\" required>",
        "<option value=\"de\">Germany</option><option value=\"at\" selected>Austria</option>",
        "<input type=\"file\" name=\"cv\" accept=\".pdf\" required>",
    ] {
        assert!(out.html.contains(part), "{part}\n{}", out.html);
    }
    let params = &out.json["actions"][0]["params"];
    assert_eq!(params["age"], json!({ "type": "integer", "minimum": 18 }));
    assert_eq!(params["price"], json!({ "type": "number", "minimum": 0, "maximum": 9.99 }));
    assert_eq!(params["country"], json!({ "type": "string", "enum": ["de", "at"] }));
}

#[test]
fn field_type_arguments_are_checked() {
    let src = page(
        "  form(\"/p\", name: \"p\") {\n    field(\"a\", type: Int(max: 5.5), label: \"A\")\n    field(\"b\", type: String(size: 80), label: \"B\")\n    field(\"c\", type: Accept(1), label: \"C\")\n    field(\"d\", type: Choice, label: \"D\")\n    field(\"e\", type: Password, label: \"E\", default: \"secret\")\n    button \"Go\"\n  }",
    );
    let result = compile(&src, &json!({}), "p");
    assert_eq!(codes(&result.diagnostics), ["TYPE-005"; 5], "{:?}", result.diagnostics);
}

#[test]
fn actions_style_their_button_and_merge_when_repeated() {
    let result = compile(
        &page(
            "  props(cart: Cart)\n  for item in cart.items { action(\"cart.remove\", id: item.id, class: \"btn\", hx-boost: true) \"Remove {item.name}\" }",
        ),
        &json!({ "cart": { "items": [{ "id": 1, "name": "A" }, { "id": 2, "name": "B" }, { "id": 1, "name": "A" }] } }),
        "p",
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let out = result.output.unwrap();
    assert!(out.html.contains("<input type=\"hidden\" name=\"id\" value=\"2\">\n    <button type=\"submit\" class=\"btn\" hx-boost=\"true\">Remove B</button>"), "{}", out.html);
    assert!(!out.html.contains("name=\"class\""));
    let actions = out.json["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0]["params"]["id"], json!({ "type": "integer", "enum": [1, 2] }));
    assert!(out.markdown.contains("- `cart.remove(id: 1|2)`: Remove A → POST /_a/cart.remove"), "{}", out.markdown);
}

#[test]
fn markdown_handles_code_blocks_links_in_headings_and_cells_and_article_headers() {
    let out = compile(
        &page(
            "  article {\n    header {\n      h2 { a(\"/post\") \"Post title\" }\n      p \"By Ann\"\n    }\n    pre { code(class: \"language-rust\") \"fn main() {{}}\" }\n    table {\n      tr { th \"Name\" }\n      tr { td { a(\"/a\") \"Open A\" } }\n    }\n  }",
        ),
        &json!({}),
        "p",
    )
    .output
    .unwrap();
    assert!(out.html.contains("<pre><code class=\"language-rust\">fn main() {}</code></pre>"), "{}", out.html);
    for part in [
        "## [Post title](/post)\n\nBy Ann\n\n",
        "```rust\nfn main() {}\n```\n",
        "| Name |\n| --- |\n| [Open A](/a) |\n",
    ] {
        assert!(out.markdown.contains(part), "{part}\n{}", out.markdown);
    }
}

#[test]
fn strings_can_span_lines() {
    let out = html(
        "  props(u: U)\n  p \"\"\"\n    Dear {u.name},\n      thanks for your order.\n\n    See you soon\n    \"\"\"\n  pre \"\"\"\n    fn main() {\n        println!(\"hi\");\n    }\n    \"\"\"",
        json!({ "u": { "name": "Ann" } }),
    );
    assert!(out.contains("Dear Ann,\n  thanks for your order.\n\nSee you soon"), "{out}");
    assert!(out.contains("<pre>fn main() {\n    println!(&quot;hi&quot;);\n}</pre>"), "{out}");
    let script = html("  script \"\"\"\n    console.log(\"a\\nb\")\n    \"\"\"", json!({}));
    assert!(script.contains("<script>console.log(\"a\\nb\")</script>"), "{script}");
}

#[test]
fn multiline_strings_must_be_closed_and_indented() {
    for (body, message) in [
        ("  p \"\"\"text\"\"\"", "unterminated string"),
        ("  p \"\"\"\n    text\n", "unterminated string"),
        ("  p \"\"\"\n  text\n    \"\"\"", "a line is indented less than the closing \"\"\""),
    ] {
        let result = compile(&page(body), &json!({}), "p");
        assert_eq!(result.diagnostics[0].message, message, "{body}");
    }
}

#[test]
fn true_writes_an_attribute_bare_unless_it_takes_text() {
    let out = html(
        "  a(\"/f.pdf\", download: true, translate: false) \"Get it\"\n  div(popover: true, aria-hidden: true, data-open: false, hx-boost: true, contenteditable: true) \"x\"",
        json!({}),
    );
    assert!(out.contains("<a href=\"/f.pdf\" download>Get it</a>"), "{out}");
    assert!(
        out.contains(
            "<div popover aria-hidden=\"true\" data-open=\"false\" hx-boost=\"true\" contenteditable=\"true\">x</div>"
        ),
        "{out}"
    );
}

#[test]
fn svg_math_and_shadow_dom_templates_are_written_as_is() {
    let out = html(
        "  svg(viewBox: \"0 0 24 24\", width: 24, aria-hidden: true) {\n    path(d: \"M12 2L2 22h20z\", fill: \"currentColor\")\n    use(href: \"#icon\")\n  }\n  math { mi \"x\" }\n  my-card {\n    template(shadowrootmode: \"open\") { slot(name: \"title\") }\n  }",
        json!({}),
    );
    assert!(out.contains("<svg viewBox=\"0 0 24 24\" width=\"24\" aria-hidden=\"true\">\n    <path d=\"M12 2L2 22h20z\" fill=\"currentColor\"></path>\n    <use href=\"#icon\"></use>\n  </svg>"), "{out}");
    assert!(out.contains("<math>\n    <mi>x</mi>\n  </math>"), "{out}");
    assert!(
        out.contains("<template shadowrootmode=\"open\">\n      <slot name=\"title\"></slot>\n    </template>"),
        "{out}"
    );
}

#[test]
fn conditions_and_loops_can_sit_inside_a_line() {
    let out = compile(
        &page("  props(u: U)\n  p { \"Hello\" if u.vip { b \" (VIP)\" } \"! Tags: \" for t in u.tags { code \"{t}\" \" \" } }"),
        &json!({ "u": { "vip": true, "tags": ["a", "b"] } }),
        "p",
    )
    .output
    .unwrap();
    assert!(out.html.contains("<p>Hello<b> (VIP)</b>! Tags: <code>a</code> <code>b</code> </p>"), "{}", out.html);
    assert!(out.markdown.contains("Hello** (VIP)**! Tags: `` a `` `` b ``\n"), "{}", out.markdown);
}

#[test]
fn strings_split_into_text_and_data() {
    use hypra::parser::{Braces, Piece, pieces};
    let text = |t: &str| Piece::Text(t.to_string());
    assert_eq!(
        pieces("Hi {user.name}!", Braces::Doubled),
        [text("Hi "), Piece::Data { name: "user.name", at: 3 }, text("!")]
    );
    assert_eq!(pieces("{{x}} { open: false } }", Braces::Doubled), [text("{x} { open: false } }")]);
    assert_eq!(
        pieces("{\"a\": {\"b\": {x}}}", Braces::AsWritten),
        [text("{\"a\": {\"b\": "), Piece::Data { name: "x", at: 12 }, text("}}"),]
    );
}

#[test]
fn blocks_inside_table_cells_stay_on_the_row() {
    let out = compile(
        &page("  table {\n    tr { th \"Item\"  th \"\" }\n    tr { td \"Socks\"  td { action(\"cart.remove\", id: 3) \"Remove\" } }\n  }"),
        &json!({}),
        "p",
    )
    .output
    .unwrap();
    assert!(out.markdown.contains("| Item |  |\n| --- | --- |\n| Socks | [Action: Remove]   |\n"), "{}", out.markdown);
}

#[test]
fn conditions_compare_with_fixed_values() {
    let page_with = |data: Json| {
        html(
            "  props(u: U)\n  if u.role == \"admin\" { p \"Admin\" } else if u.age != 18 { p \"Not 18\" } else { p \"18\" }\n  if u.vip == true { p \"VIP\" }",
            data,
        )
    };
    assert!(page_with(json!({ "u": { "role": "admin", "age": 18, "vip": false } })).contains("<p>Admin</p>"));
    let out = page_with(json!({ "u": { "role": "user", "age": 30, "vip": true } }));
    assert!(out.contains("<p>Not 18</p>") && out.contains("<p>VIP</p>"), "{out}");
    assert!(page_with(json!({ "u": { "role": "user", "age": 18, "vip": false } })).contains("<p>18</p>"));
}

#[test]
fn comparisons_take_fixed_values() {
    for (body, message) in [
        ("  props(u: U)\n  if u.a == u.b { p \"x\" }", "compare with fixed text, a number, true or false"),
        ("  props(u: U)\n  if not u.a == \"x\" { p \"x\" }", "use `!=` instead of `not` with a comparison"),
    ] {
        let result = compile(&page(body), &json!({ "u": {} }), "p");
        assert_eq!(result.diagnostics[0].message, message);
    }
}

#[test]
fn lists_can_be_written_in_the_page() {
    let out = html(
        "  ul { for size in [\"S\", \"M\", 42] { li \"{size}\" } }\n  div(data-sizes: [\"S\", \"M\"], data-steps: [\n    1,\n    2,\n  ]) \"x\"",
        json!({}),
    );
    assert!(out.contains("<li>S</li>\n    <li>M</li>\n    <li>42</li>"), "{out}");
    assert!(out.contains("<div data-sizes=\"[&quot;S&quot;,&quot;M&quot;]\" data-steps=\"[1,2]\">x</div>"), "{out}");
    let result = compile(&page("  props(u: U)\n  for x in [u.a] { p \"{x}\" }"), &json!({ "u": { "a": 1 } }), "p");
    assert_eq!(codes(&result.diagnostics), ["ATTR-003"]);
}
