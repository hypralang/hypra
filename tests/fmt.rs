//! `hypra fmt` changes only whitespace, and formatting twice changes nothing.

use hypra::fmt::format;
use hypra::lexer::{Tok, lex};

/// The tokens the parser sees: formatting must leave them exactly as they were.
fn tokens(src: &str) -> Vec<Tok> {
    lex(src, 0).expect("the source lexes").into_iter().map(|t| t.tok).collect()
}

fn assert_formats_safely(name: &str, src: &str) {
    let once = format(src).unwrap_or_else(|d| panic!("{name}: {d:?}"));
    assert_eq!(tokens(src), tokens(&once), "{name}: formatting changed the tokens\n{once}");
    assert_eq!(format(&once).unwrap(), once, "{name}: formatting twice changes the result");
}

#[test]
fn messy_code_gets_one_layout() {
    let messy = r#"// a page
page( title:"Shop" ,description : "A test page with a description long enough for search results." ){
props( items : List )


    h1 "Shoes"   // the main heading
ul{
for item in items { li( class:"item" ) "{item}" }
        }
  div(data-steps: [
1,
      2 ,
 ])   {   p "x"  b "y"     i "z" }
pre """
      line one
        indented
      """
}
"#;
    let clean = r#"// a page
page(title: "Shop", description: "A test page with a description long enough for search results.") {
  props(items: List)

  h1 "Shoes"   // the main heading
  ul {
    for item in items { li(class: "item") "{item}" }
  }
  div(data-steps: [
    1,
    2,
  ]) { p "x"  b "y"  i "z" }
  pre """
        line one
          indented
        """
}
"#;
    assert_eq!(format(messy).unwrap(), clean);
    assert_formats_safely("messy", messy);
}

#[test]
fn examples_and_docs_format_safely() {
    let root = env!("CARGO_MANIFEST_DIR");
    for entry in std::fs::read_dir(format!("{root}/examples")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "hypra") {
            let src = std::fs::read_to_string(&path).unwrap();
            assert_formats_safely(&path.display().to_string(), &src);
        }
    }
    for file in ["README.md", "docs/spec.md"] {
        let text = std::fs::read_to_string(format!("{root}/{file}")).unwrap();
        for (i, block) in text.split("```hypra\n").skip(1).enumerate() {
            let code = block.split("```").next().unwrap_or_default();
            if lex(code, 0).is_ok() {
                assert_formats_safely(&format!("{file} block {i}"), code);
            }
        }
    }
}

#[test]
fn files_that_dont_lex_are_reported() {
    assert!(format("p \"never closed").is_err());
}
