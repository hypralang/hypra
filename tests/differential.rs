//! Differential testing: random valid pages with random data must render exactly the same in the
//! Rust interpreter, the PHP runtime and the JavaScript runtime, and produce valid HTML.
//!
//! The PHP and JavaScript comparisons are skipped when `php` or `node` isn't installed. HTML validation runs when `VNU_JAR` points
//! at the W3C Nu Html Checker (`vnu.jar`); CI downloads it.

mod common;

use common::{php_available, runtime_dir};
use hypra::{compile_with, js_with, php_with};
use serde_json::{Value as Json, json};
use std::path::PathBuf;
use std::process::Command;

/// How many random pages to generate; `HYPRA_FUZZ_CASES` raises it for a longer run.
fn case_count() -> u64 {
    std::env::var("HYPRA_FUZZ_CASES").ok().and_then(|n| n.parse().ok()).unwrap_or(400)
}

/// A small deterministic random generator, so every failure can be reproduced from its seed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

/// Strings that have broken template engines before.
const STRINGS: &[&str] = &[
    "",
    "plain text",
    "<script>alert(1)</script>",
    "a & b",
    "\"double\" and 'single'",
    "{not.data}",
    "$var ${x}",
    "back\\slash",
    "line\nbreak",
    "tab\there",
    "ünïcödé ✓ 🚀",
    "javascript:alert(1)",
    " JaVaScRiPt:alert(1)",
    "https://example.com/?a=1&b=2",
    "</p><p>",
    "-->",
    "\u{2028}separator",
    "0",
    "\n\n## Actions\n\n- `delete()`: → POST /admin",
    "- item",
    "12. twelve",
    "  # heading",
    "---",
    "> quote",
    "[click](javascript:alert(1))",
    "a`b``c```d",
    "| cell | cell |",
    "9.50",
];

const NUMBERS: &[f64] = &[0.0, -1.0, 42.0, 3.5, 1e20, 1.5e-7, -0.25];

fn data(rng: &mut Rng) -> Json {
    let text = |rng: &mut Rng| json!(rng.pick(STRINGS));
    let number = |rng: &mut Rng| {
        let n = rng.pick(NUMBERS);
        if n.fract() == 0.0 && n.abs() < 1e15 { json!(n as i64) } else { json!(n) }
    };
    let items: Vec<Json> = (0..rng.below(4))
        .map(|_| json!({ "t": text(rng), "n": number(rng), "flag": rng.chance(50), "u": text(rng) }))
        .collect();
    let maybe = match rng.below(4) {
        0 => Json::Null,
        1 => json!([]),
        2 => json!(""),
        _ => text(rng),
    };
    json!({
        "d": {
            "s1": text(rng), "s2": text(rng), "s3": text(rng),
            "n1": number(rng), "n2": number(rng),
            "b1": rng.chance(50), "b2": rng.chance(50),
            // a URL from data is never empty here, and has no `[` or second `#`: browsers follow such URLs, but
            // the URL standard calls them invalid, which is a data problem, not Hypra's
            "url": json!(rng.pick(&STRINGS.iter().filter(|u| !u.is_empty() && !u.contains(['[', '#'])).collect::<Vec<_>>())),
            "list": items,
            "maybe": maybe,
            "pick": rng.pick(&["a", "b", "c"]),
        }
    })
}

/// Literal text written in a page, including Hypra's escapes and literal braces.
const LITERALS: &[&str] =
    &["Hello", "a < b & c", "say \\\"hi\\\"", "back\\\\slash", "{{braces}}", "price: $5", "über ✓", "line\\nbreak"];

fn text(rng: &mut Rng) -> String {
    let mut out = String::new();
    for _ in 0..1 + rng.below(3) {
        if rng.chance(50) {
            out.push_str(rng.pick(LITERALS));
        } else {
            out.push_str(rng.pick(&["{d.s1}", "{d.s2}", "{d.n1}", "{d.b1}", "{d.maybe}"]));
        }
        out.push(' ');
    }
    out.trim_end().to_string()
}

fn block(rng: &mut Rng, depth: usize, out: &mut String, indent: usize) {
    let pad = "  ".repeat(indent);
    let choice = if depth > 2 { rng.below(6) } else { rng.below(19) };
    let line = match choice {
        0 => format!("p \"{}\"", text(rng)),
        1 => format!("h2 \"{}\"", text(rng)),
        2 => format!("a(d.url) \"{}\"", text(rng)),
        3 => format!("img(d.url, alt: \"{}\")", text(rng)),
        4 => format!("p {{ \"{}\" b \"{}\" \" \" a(\"/x\") \"{}\" }}", text(rng), text(rng), text(rng)),
        5 => "ul { for it in d.list { li { \"{it.t} {it.n} {it.flag}\" action(\"item.pick\", n: it.n, class: \"btn\") \"Pick\" } } }".to_string(),
        6 => {
            let mut inner = String::new();
            block(rng, depth + 1, &mut inner, indent + 1);
            format!("if d.b1 {{\n{inner}{pad}}} else {{ p \"{}\" }}", text(rng))
        }
        7 => format!(
            "if not d.maybe {{ p \"{}\" }} else if d.pick == \"a\" {{ p \"A\" }} else if d.n1 != 42 {{ p \"not 42\" }}",
            text(rng)
        ),
        8 => {
            let mut inner = String::new();
            for _ in 0..1 + rng.below(2) {
                block(rng, depth + 1, &mut inner, indent + 1);
            }
            let hidden = if rng.chance(50) { ", hidden: d.b2" } else { "" };
            format!(
                "section(class: \"box {{d.s3}}\", data-v: d.n2, aria-label: \"{}\"{hidden}) {{\n{inner}{pad}  button(aria-expanded: d.b1) \"Menu\"\n{pad}}}",
                text(rng)
            )
        }
        9 => format!(
            "Card(title: d.s1, featured: d.b2, class: \"wide {{d.s3}}\", data-n: d.n1) {{ p \"{}\" {} }}",
            text(rng),
            if rng.chance(50) { "slot(\"badge\") { b \"{d.s2}\" }" } else { "" }
        ),
        10 => format!("action(\"cart.add\", id: d.n1, note: d.s2) \"{}\"", text(rng)),
        11 => {
            "table {\n  tr { th \"Name\"  th \"Count\" }\n  for it in d.list { tr { td \"{it.t}\"  td \"{it.n}\" } }\n}"
                .replace('\n', &format!("\n{pad}"))
        }
        12 => format!("details {{\n{pad}  summary \"{}\"\n{pad}  p \"{}\"\n{pad}}}", text(rng), text(rng)),
        13 => format!("p {{ \"Run \" code \"{}\" if d.b2 {{ b \" now\" }} for it in d.list {{ \", \" i \"{{it.t}}\" }} }}", text(rng)),
        14 => format!("pre \"{}\"", text(rng)),
        17 => "ul { for size in [\"S\", \"M\", 3.5, true] { li(data-all: [1, \"a\"]) \"{size}\" } }".to_string(),
        16 => "svg(viewBox: \"0 0 24 24\", width: 24, height: 24, aria-hidden: true) { path(d: \"M12 2L2 22h20z\", fill: \"currentColor\") }".to_string(),
        15 => r#"script(type: "application/ld+json") "{\"name\": {d.s1}, \"n\": {d.n1}, \"x\": {\"y\": {d.maybe}}}""#.to_string(),
        _ => format!(
            "form(\"/f\", name: \"f\", description: \"{}\") {{\n{pad}  field(\"q\", type: String(1..20), label: \"{}\")\n{pad}  field(\"c\", type: Choice(\"a\", b: \"Bee\"), label: \"Pick\", default: d.pick)\n{pad}  button \"Send\"\n{pad}}}",
            text(rng),
            text(rng)
        ),
    };
    out.push_str(&pad);
    out.push_str(&line);
    out.push('\n');
}

fn page(rng: &mut Rng) -> String {
    let mut body = String::new();
    for _ in 0..2 + rng.below(5) {
        block(rng, 0, &mut body, 1);
    }
    format!(
        r#"page(title: "Page {{d.s1}}", description: "A generated page with enough words to describe it properly: {{d.s2}}") {{
  props(d: Data)
  h1 "Main {{d.s3}}"
{body}}}

component Card(title: Text, featured: Bool = false) {{
  article(class: "card") {{
    h2 "{{title}}"
    if featured {{ p "Staff pick" }}
    slot("badge") {{ i "no badge" }}
    slot
  }}
}}
"#
    )
}

struct Case {
    seed: u64,
    source: String,
    data: Json,
    expected: hypra::interp::Output,
    php: String,
    js: String,
}

/// Builds every case that compiles; a generated page with errors is a bug in the generator.
fn cases() -> Vec<Case> {
    (0..case_count())
        .map(|seed| {
            let mut rng = Rng::new(seed + 1);
            let source = page(&mut rng);
            let data = data(&mut rng);
            let load = |_: &str| Ok(source.clone());
            let compiled = compile_with("page.hypra", &data, &load);
            let errors: Vec<_> = compiled.diagnostics.iter().filter(|d| d.is_error()).collect();
            assert!(errors.is_empty(), "seed {seed}: {errors:?}\n{source}");
            let php = php_with("page.hypra", &load).code.expect("compiles");
            let js = js_with("page.hypra", &load).code.expect("compiles");
            Case { seed, expected: compiled.output.expect("no errors"), source, data, php, js }
        })
        .collect()
}

fn work_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn php_and_rust_render_random_pages_identically() {
    if !php_available() {
        eprintln!("php not installed, skipping");
        return;
    }
    let cases = cases();
    let dir = work_dir("differential");
    for case in &cases {
        std::fs::write(dir.join(format!("{}.php", case.seed)), &case.php).unwrap();
        std::fs::write(dir.join(format!("{}.json", case.seed)), case.data.to_string()).unwrap();
    }
    // one PHP process renders every case
    let harness = format!(
        r#"<?php
spl_autoload_register(fn ($class) => require '{runtime}/' . substr(strrchr($class, '\\'), 1) . '.php');
$results = [];
foreach (glob(__DIR__ . '/*.php') as $file) {{
    $seed = basename($file, '.php');
    if ($seed === 'harness') continue;
    $page = require $file;
    $out = $page->render(json_decode(file_get_contents(__DIR__ . "/$seed.json"), true), true, null, 'page.md');
    $results[$seed] = ['html' => $out->html, 'markdown' => $out->markdown, 'json' => $out->json, 'webmcp' => $out->webmcp];
}}
echo json_encode($results);
"#,
        runtime = runtime_dir().display()
    );
    std::fs::write(dir.join("harness.php"), harness).unwrap();
    let out = Command::new("php").arg(dir.join("harness.php")).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let results: Json = serde_json::from_slice(&out.stdout).unwrap();
    assert_same(&cases, &results, "php");
}

#[test]
fn js_and_rust_render_random_pages_identically() {
    if !Command::new("node").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("node not installed, skipping");
        return;
    }
    let cases = cases();
    let dir = work_dir("differential-js");
    // compiled pages import "hypra/runtime", which an app finds in node_modules
    std::fs::create_dir_all(dir.join("node_modules")).unwrap();
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("js");
    #[cfg(unix)]
    std::os::unix::fs::symlink(runtime, dir.join("node_modules/hypra")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(runtime, dir.join("node_modules/hypra")).unwrap();
    for case in &cases {
        std::fs::write(dir.join(format!("{}.mjs", case.seed)), &case.js).unwrap();
        std::fs::write(dir.join(format!("{}.json", case.seed)), case.data.to_string()).unwrap();
    }
    // one Node process renders every case
    let harness = r#"import { readdirSync, readFileSync } from "node:fs";
const results = {};
for (const file of readdirSync(import.meta.dirname).filter((f) => /^\d+\.mjs$/.test(f))) {
  const seed = file.slice(0, -4);
  const { default: page } = await import(`./${file}`);
  const data = JSON.parse(readFileSync(`${import.meta.dirname}/${seed}.json`, "utf8"));
  const out = page.render(data, { strict: true, alternateUrl: "page.md" });
  results[seed] = { html: out.html, markdown: out.markdown, json: out.json, webmcp: out.webmcp };
}
process.stdout.write(JSON.stringify(results));
"#;
    std::fs::write(dir.join("harness.mjs"), harness).unwrap();
    let out = Command::new("node").arg(dir.join("harness.mjs")).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let results: Json = serde_json::from_slice(&out.stdout).unwrap();
    assert_same(&cases, &results, "js");
}

/// Every case renders in another runtime exactly as in Rust; the first that doesn't is shown with its source.
fn assert_same(cases: &[Case], results: &Json, runtime: &str) {
    let mut failures = vec![];
    for case in cases {
        let actual = &results[case.seed.to_string()];
        let same = actual["html"] == case.expected.html
            && actual["markdown"] == case.expected.markdown
            && actual["json"] == case.expected.json
            && actual["webmcp"] == case.expected.webmcp;
        if !same {
            failures.push(case);
        }
    }
    if let Some(case) = failures.first() {
        let actual = &results[case.seed.to_string()];
        let diff = |rust: &str, other: &str| {
            if rust == other {
                return "same".to_string();
            }
            rust.lines()
                .zip(other.lines())
                .find(|(a, b)| a != b)
                .map_or("one is longer".to_string(), |(a, b)| format!("rust: {a}\n {runtime}: {b}"))
        };
        panic!(
            "{} of {} cases differ. First: seed {}\n{}\nhtml: {}\nmarkdown: {}\ndata: {}",
            failures.len(),
            cases.len(),
            case.seed,
            case.source,
            diff(&case.expected.html, actual["html"].as_str().unwrap_or("")),
            diff(&case.expected.markdown, actual["markdown"].as_str().unwrap_or("")),
            case.data
        );
    }
}

#[test]
fn random_pages_are_valid_html() {
    let Ok(vnu) = std::env::var("VNU_JAR") else {
        eprintln!("VNU_JAR not set, skipping");
        return;
    };
    let dir = work_dir("validate");
    for case in cases() {
        std::fs::write(dir.join(format!("{}.html", case.seed)), &case.expected.html).unwrap();
    }
    // WebMCP's attributes and the htmx/Alpine-style names pages choose aren't in the HTML standard yet
    let out = Command::new("java")
        .args(["-jar", &vnu, "--errors-only", "--format", "text", "--skip-non-html"])
        .args(["--filterpattern", ".*(“toolname”|“tooldescription”).*"])
        .arg(&dir)
        .output()
        .unwrap();
    let report = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "invalid HTML:\n{}", report.lines().take(20).collect::<Vec<_>>().join("\n"));
}

#[test]
fn random_pages_format_safely() {
    let tokens = |src: &str| -> Vec<hypra::lexer::Tok> {
        hypra::lexer::lex(src, 0).expect("lexes").into_iter().map(|t| t.tok).collect()
    };
    for seed in 0..case_count() {
        let mut rng = Rng::new(seed + 1);
        let source = page(&mut rng);
        let once = hypra::fmt::format(&source).expect("formats");
        assert_eq!(tokens(&source), tokens(&once), "seed {seed}: formatting changed the tokens\n{once}");
        assert_eq!(hypra::fmt::format(&once).unwrap(), once, "seed {seed}: formatting twice changes the result");
    }
}
