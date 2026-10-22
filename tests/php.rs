//! Runs generated PHP with the real PHP runtime and compares it to the Rust interpreter.
//! Skipped when `php` isn't installed.

mod common;

use common::{php_available, render_with_php, runtime_dir};
use hypra::{compile_file, php_file, php_with};
use serde_json::{Value as Json, json};
use std::process::Command;

#[test]
fn php_output_matches_the_interpreter() {
    if !php_available() {
        eprintln!("php not installed, skipping");
        return;
    }
    let data: Json = serde_json::from_str(include_str!("../examples/product.json")).unwrap();
    let expected = compile_file("examples/product.hypra", &data).output.unwrap();
    let code = php_file("examples/product.hypra").code.unwrap();

    let actual = render_with_php(&code, &data, true, "product", "product.md").unwrap();
    assert_eq!(actual["html"].as_str().unwrap(), expected.html);
    assert_eq!(actual["markdown"].as_str().unwrap(), expected.markdown);
    assert_eq!(actual["json"], expected.json);
    assert_eq!(actual["webmcp"], expected.webmcp);
}

#[test]
fn strict_php_reports_data_errors_with_their_source_location() {
    if !php_available() {
        eprintln!("php not installed, skipping");
        return;
    }
    let page = "page(title: \"T\", description: \"D\") {\n  props(user: User)\n  p \"{user.name}\"\n}\n";
    let code = php_with("site/page.hypra", &|_| Ok(page.to_string())).code.unwrap();

    let missing = render_with_php(&code, &json!({}), true, "missing", "page.md").unwrap_err();
    assert_eq!(missing, "site/page.hypra:2:9: prop `user` is missing from the data");

    let error = render_with_php(&code, &json!({ "user": {} }), true, "strict", "page.md").unwrap_err();
    assert_eq!(error, "site/page.hypra:3:6: `user` has no field `name`");

    let lenient = render_with_php(&code, &json!({ "user": {} }), false, "lenient", "page.md").unwrap();
    assert!(lenient["html"].as_str().unwrap().contains("<p></p>"));
}

#[test]
fn constants_are_folded_into_static_output() {
    let code = php_file("examples/product.hypra").code.unwrap();
    assert!(!code.contains("htmlText('"), "constant values should be written as static HTML");
    assert!(!code.contains("$c->concat($c->concat("), "interpolations should be flat");
}

#[test]
fn special_characters_survive_php_quoting() {
    if !php_available() {
        eprintln!("php not installed, skipping");
        return;
    }
    let page = r#"allow("SEO-004") "no h1 in this test" page(title: "Cost: $5 \\ \"quoted\" 'single'", description: "Tabs\tand\nlines {note.text}, and enough words for a description.") {
  props(note: Note)
  p "$note {note.text} <b> & ünïcödé ✓"
  p "{note.text}"
  allow("SEC-002") "this test checks that raw output survives PHP quoting"
  raw "<i>{note.text}</i>"
}
"#;
    let data = json!({ "note": { "text": "a $b \\ \"c\" 'd' <e> & f\ng" } });
    let load = |_: &str| Ok(page.to_string());
    let expected = hypra::compile_with("page.hypra", &data, &load);
    assert!(expected.diagnostics.is_empty(), "{:?}", expected.diagnostics);
    let expected = expected.output.unwrap();
    let code = php_with("page.hypra", &load).code.unwrap();

    let actual = render_with_php(&code, &data, true, "quoting", "page.md").unwrap();
    assert_eq!(actual["html"].as_str().unwrap(), expected.html);
    assert_eq!(actual["markdown"].as_str().unwrap(), expected.markdown);
    assert_eq!(actual["json"], expected.json);
}

/// Values whose display could differ between the two runtimes.
#[test]
fn edge_values_render_the_same_in_php() {
    if !php_available() {
        eprintln!("php not installed, skipping");
        return;
    }
    let page = r#"allow("SEO-004") "no h1 in this test" page(title: "Edge values", description: "Values that PHP and Rust could display differently, checked side by side.") {
  props(v: V)
  if v.empty { p "empty object is true" } else { p "empty object is false" }
  p "{v.big} {v.small} {v.half} {v.negative}"
  a(v.url) "Link"
}
"#;
    let data = json!({ "v": { "empty": {}, "big": 1e20, "small": 1.5e-7, "half": 0.5, "negative": -3, "url": "javascript:alert(1)" } });
    let load = |_: &str| Ok(page.to_string());
    let expected = hypra::compile_with("page.hypra", &data, &load).output.unwrap();
    let code = php_with("page.hypra", &load).code.unwrap();
    let actual = render_with_php(&code, &data, true, "edge", "page.md").unwrap();
    assert_eq!(actual["html"].as_str().unwrap(), expected.html);
    assert!(expected.html.contains("empty object is false") && expected.html.contains("1e+20 1.5e-7 0.5 -3"));
}

/// PHP-only values: enums from Eloquent casts, invalid UTF-8, maps where a list is expected.
#[test]
fn php_runtime_handles_values_json_cannot_carry() {
    if !php_available() {
        eprintln!("php not installed, skipping");
        return;
    }
    let runtime = runtime_dir();
    let script = format!(
        r#"spl_autoload_register(fn ($c) => require '{runtime}/' . substr(strrchr($c, '\\'), 1) . '.php');
enum Status: string {{ case Paid = 'paid'; }}
enum Size {{ case Large; }}
use Hypra\Runtime\{{Context, Value}};
echo Value::display(Status::Paid), ' ', Value::display(Size::Large), ' ', Value::json("bad \xB1 utf8"), "\n";
$c = new Context([], true);
try {{ iterator_to_array($c->each(['a' => 1], 'v.map', 'p.hypra:1:1')); }} catch (Hypra\Runtime\DataError $e) {{ echo $e->getMessage(), "\n"; }}
$c = new Context([], false, fn () => ['_token' => 't']);
$c->formFields('', 'https://other.example/x'); $c->formFields('', '//other.example/x'); $c->formFields('', '/local');
echo $c->html;"#,
        runtime = runtime.display()
    );
    let out = Command::new("php").args(["-r", &script]).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        stdout,
        "paid Large \"bad \u{fffd} utf8\"\np.hypra:1:1: `v.map` isn't a list\n<input type=\"hidden\" name=\"_token\" value=\"t\">\n",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
