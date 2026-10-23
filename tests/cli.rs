//! The command line: usage errors exit with 2 and never write anything.

use std::process::Command;

fn hypra(args: &[&str]) -> (i32, String) {
    let out =
        Command::new(env!("CARGO_BIN_EXE_hypra")).current_dir(env!("CARGO_MANIFEST_DIR")).args(args).output().unwrap();
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn options_need_their_values() {
    for (args, message) in [
        (&["build", "examples/product.hypra", "--out"][..], "--out needs a value"),
        (&["check", "examples/product.hypra", "--data"], "--data needs a value"),
        (&["check", "examples/product.hypra", "--format", "xml"], "unknown format `xml`"),
        (&["build", "examples/product.hypra", "--target", "python"], "unknown target `python`"),
        (&["build", "examples/product.hypra", "--target", "js", "--data", "x.json"], "--data doesn't apply"),
        (&["build", "examples/product.hypra", "--out", "--data", "x.json"], "--out needs a value"),
        (&["check", "examples", "--data", "examples/product.json"], "--data is the data of one page"),
        (&["check", "examples/product.hypra", "examples/broken.hypra", "--data", "x.json"], "data of one page"),
        (&["check", "src"], "src has no .hypra files"),
        (&["fmt", "--check", "src"], "src has no .hypra files"),
        (&["build", "examples/layouts", "--target", "js"], "no pages to build in examples/layouts"),
    ] {
        let (code, stderr) = hypra(args);
        assert_eq!(code, 2, "{args:?}");
        assert!(stderr.contains(message), "{args:?}: {stderr}");
    }
}

#[test]
fn check_reports_problems_with_exit_code_1() {
    let (code, stderr) = hypra(&["check", "examples/broken.hypra"]);
    assert_eq!(code, 1, "{stderr}");
}

#[test]
fn the_php_target_lists_its_sources() {
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli-php");
    let (code, stderr) = hypra(&["build", "examples/product.hypra", "--target", "php", "--out", out.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    let sources: Vec<String> =
        serde_json::from_str(&std::fs::read_to_string(out.join("product.sources.json")).unwrap()).unwrap();
    assert!(sources[0].ends_with("examples/product.hypra"), "{sources:?}");
    assert!(sources.len() > 1, "imports are listed too: {sources:?}");
    assert!(out.join("product.php").exists());
}

#[test]
fn the_js_target_writes_a_module_with_its_types() {
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli-js");
    let (code, stderr) = hypra(&["build", "examples/product.hypra", "--target", "js", "--out", out.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    let module = std::fs::read_to_string(out.join("product.mjs")).unwrap();
    assert!(module.contains("import { Page } from \"hypra/runtime\";"), "{module}");
    let types = std::fs::read_to_string(out.join("product.d.mts")).unwrap();
    assert!(types.contains("declare const page: Page;"), "{types}");
    assert!(out.join("product.sources.json").exists());
}

#[test]
fn a_folder_is_every_hypra_file_in_it() {
    let (code, stderr) = hypra(&["check", "examples"]);
    assert_eq!(code, 1, "broken.hypra has errors: {stderr}");
    assert!(stderr.contains("4 files, 6 errors, 0 warnings"), "{stderr}");
}

#[test]
fn building_a_folder_keeps_its_subfolders_and_skips_components() {
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli-folder");
    let (code, stderr) = hypra(&["build", "site/pages", "--target", "js", "--out", out.to_str().unwrap()]);
    assert_eq!(code, 0, "{stderr}");
    assert!(out.join("index.mjs").exists() && out.join("docs/index.mjs").exists(), "{stderr}");

    // the examples' layouts and components are built into the pages that import them
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli-folder-php");
    let (_, stderr) = hypra(&["build", "examples", "--target", "php", "--out", out.to_str().unwrap()]);
    assert!(stderr.contains("2 files"), "{stderr}");
    assert!(out.join("product.php").exists() && !out.join("components").exists(), "{stderr}");
}

#[test]
fn a_build_passes_over_folders_of_components() {
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli-components");
    let args = ["build", "site/pages", "examples/layouts", "examples/components", "--target", "js", "--out"];
    let (code, stderr) = hypra(&[&args[..], &[out.to_str().unwrap()]].concat());
    assert_eq!(code, 0, "{stderr}");
    assert!(out.join("index.mjs").exists(), "{stderr}");
}

#[test]
fn fmt_takes_folders() {
    let (code, stderr) = hypra(&["fmt", "--check", "examples", "site/pages"]);
    assert_eq!(code, 0, "{stderr}");
}

#[test]
fn two_pages_with_one_name_dont_overwrite_each_other() {
    let out = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli-twice");
    let args = ["build", "site/pages/index.hypra", "site/pages/docs/index.hypra", "--target", "js", "--out"];
    let (code, stderr) = hypra(&[&args[..], &[out.to_str().unwrap()]].concat());
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("two pages build to"), "{stderr}");
    assert!(!out.exists(), "nothing is written");
}

#[test]
fn fmt_reads_stdin_for_editors() {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_hypra"))
        .args(["fmt", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"p(class:\"x\"){b \"y\"}").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "p(class: \"x\") { b \"y\" }\n");
}
