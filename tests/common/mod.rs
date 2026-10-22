//! Helpers shared by the test files that run generated PHP.
// each test file uses only some of these helpers
#![allow(dead_code)]

use serde_json::Value as Json;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn php_available() -> bool {
    Command::new("php").arg("--version").output().is_ok_and(|o| o.status.success())
}

/// The PHP runtime's source folder, for an autoloader without Composer.
pub fn runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("php/runtime/src")
}

/// Renders a generated page with PHP and returns its output as JSON.
/// `alternate` is where the page's Markdown is served; the static build links `<stem>.md`.
pub fn render_with_php(code: &str, data: &Json, strict: bool, name: &str, alternate: &str) -> Result<Json, String> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("php-{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("page.php"), code).unwrap();
    std::fs::write(dir.join("data.json"), data.to_string()).unwrap();
    let runtime = runtime_dir();
    let harness = format!(
        r#"<?php
spl_autoload_register(fn ($class) => require '{runtime}/' . substr(strrchr($class, '\\'), 1) . '.php');
$page = require __DIR__ . '/page.php';
try {{
    $out = $page->render(json_decode(file_get_contents(__DIR__ . '/data.json'), true), {strict}, null, '{alternate}');
    echo json_encode(['html' => $out->html, 'markdown' => $out->markdown, 'json' => $out->json, 'webmcp' => $out->webmcp]);
}} catch (Hypra\Runtime\DataError $e) {{
    fwrite(STDERR, $e->getMessage());
    exit(1);
}}
"#,
        runtime = runtime.display(),
        strict = strict,
        alternate = alternate,
    );
    std::fs::write(dir.join("harness.php"), harness).unwrap();
    let out = Command::new("php").arg(dir.join("harness.php")).output().unwrap();
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(serde_json::from_slice(&out.stdout).expect("harness prints JSON"))
}
