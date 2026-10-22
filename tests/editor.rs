//! What editors ask about names: definitions, references and renames, across imports and scopes.

use hypra::editor::{self, Location};
use std::path::PathBuf;

const PAGE: &str = r#"use "parts.hypra"

page(layout: Shop, title: "{product.name}", description: "x") {
  props(product: Product)
  for size in product.sizes { Badge(label: size) "{size}" }
  p "{product.name}"
}
"#;

const PARTS: &str = r#"layout Shop(lang: "en") {
  main { slot }
}

component Badge(label: Text) {
  span "{label}"
  p "{product}"
}
"#;

fn files(path: &str) -> Result<String, String> {
    match path {
        "site/page.hypra" => Ok(PAGE.into()),
        "site/parts.hypra" => Ok(PARTS.into()),
        _ => Err("not found".into()),
    }
}

/// The definition of whatever is at the first `needle` on `line` of `file`, plus `offset` characters.
fn at(file: &str, line: usize, needle: &str, offset: usize) -> Option<Location> {
    let text = files(file).unwrap();
    let col = text.lines().nth(line - 1).unwrap().find(needle).expect("needle on the line") + 1 + offset;
    editor::definition(file, line, col, &files)
}

fn location(path: &str, line: usize, col: usize, len: usize) -> Option<Location> {
    Some(Location { path: path.into(), line, col, len })
}

#[test]
fn imports_layouts_and_components() {
    assert_eq!(at("site/page.hypra", 1, "parts", 0), location("site/parts.hypra", 1, 1, 0));
    assert_eq!(at("site/page.hypra", 3, "Shop", 2), location("site/parts.hypra", 1, 8, 4));
    assert_eq!(at("site/page.hypra", 5, "Badge", 0), location("site/parts.hypra", 5, 11, 5));
}

#[test]
fn data_in_values_and_text() {
    let product = location("site/page.hypra", 4, 9, 7);
    // in a page setting, before `props` is written
    assert_eq!(at("site/page.hypra", 3, "{product", 3), product);
    // a value, and text
    assert_eq!(at("site/page.hypra", 5, "product.sizes", 2), product);
    assert_eq!(at("site/page.hypra", 6, "{product", 1), product);
    // a loop variable, in an argument and in text
    let size = location("site/page.hypra", 5, 7, 4);
    assert_eq!(at("site/page.hypra", 5, "size)", 0), size);
    assert_eq!(at("site/page.hypra", 5, "{size}", 1), size);
}

#[test]
fn scopes() {
    // a component sees its parameters, never the page's props
    assert_eq!(at("site/parts.hypra", 6, "{label}", 1), location("site/parts.hypra", 5, 17, 5));
    assert_eq!(at("site/parts.hypra", 7, "{product}", 1), None);
    // types and plain text are defined nowhere
    assert_eq!(at("site/page.hypra", 4, "Product", 0), None);
    assert_eq!(at("site/page.hypra", 3, "title", 0), None);
}

#[test]
fn unknown_components_and_files_that_dont_parse() {
    let broken = |path: &str| match path {
        "a.hypra" => Ok("page(title: \"x\") {\n  Missing\n  p \"never closed\n}\n".into()),
        _ => Err("not found".into()),
    };
    assert_eq!(editor::definition("a.hypra", 2, 3, &broken), None);
    let missing = |path: &str| match path {
        "a.hypra" => Ok("use \"gone.hypra\"\npage(title: \"x\") { Card }\n".into()),
        _ => Err("not found".into()),
    };
    assert_eq!(editor::definition("a.hypra", 2, 20, &missing), None);
    // an import that's missing is still a file to open: the editor offers to create it
    assert_eq!(
        editor::definition("a.hypra", 1, 6, &missing),
        Some(Location { path: "gone.hypra".into(), line: 1, col: 1, len: 0 })
    );
}

/// Every place found, as `file:line:col`.
fn places(found: &[Location]) -> Vec<String> {
    found.iter().map(|l| format!("{}:{}:{}", l.path, l.line, l.col)).collect()
}

fn project() -> Vec<PathBuf> {
    vec![PathBuf::from("site/page.hypra"), PathBuf::from("site/parts.hypra")]
}

#[test]
fn a_definition_leads_to_itself_so_editors_show_its_references() {
    assert_eq!(at("site/parts.hypra", 5, "Badge", 0), location("site/parts.hypra", 5, 11, 5));
    assert_eq!(at("site/page.hypra", 4, "product", 0), location("site/page.hypra", 4, 9, 7));
}

#[test]
fn references_across_files() {
    let text = files("site/parts.hypra").unwrap();
    let col = text.lines().nth(4).unwrap().find("Badge").unwrap() + 1;
    let found = editor::references("site/parts.hypra", 5, col, &project(), &files);
    // the definition, then the call in the page, which the component's file doesn't know about
    assert_eq!(places(&found), ["site/parts.hypra:5:11", "site/page.hypra:5:31"]);

    // a prop: where it's declared, and each use, in values and in text
    let found = editor::references("site/page.hypra", 4, 9, &project(), &files);
    assert_eq!(
        places(&found),
        ["site/page.hypra:3:29", "site/page.hypra:4:9", "site/page.hypra:5:15", "site/page.hypra:6:7"]
    );

    // a component parameter, and the argument that sets it where the component is called
    let found = editor::references("site/parts.hypra", 5, 17, &project(), &files);
    assert_eq!(places(&found), ["site/parts.hypra:5:17", "site/parts.hypra:6:10", "site/page.hypra:5:37"]);
}

#[test]
fn renames() {
    let found = editor::rename("site/page.hypra", 5, 32, "Tag", &project(), &files).unwrap();
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|l| l.len == 5));

    // components stay capitalized and data lowercase; keywords and files can't be names
    let rename = |line, col, name| editor::rename("site/page.hypra", line, col, name, &project(), &files);
    assert!(rename(5, 32, "tag").unwrap_err().contains("capitalized"));
    assert!(rename(4, 9, "Product").unwrap_err().contains("lowercase"));
    assert!(rename(4, 9, "for").is_err());
    assert!(rename(4, 9, "my item").is_err());
    assert!(rename(1, 6, "x").is_err());
    assert_eq!(rename(4, 9, "item").unwrap().len(), 4);
}
