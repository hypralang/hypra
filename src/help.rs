//! Help for editors, taken from the spec so it says the same as the docs: what an element, statement or field type
//! does, and where its rule codes are explained.

const SPEC: &str = include_str!("../docs/spec.md");
const DOCS: &str = "https://hypralang.dev/docs/";

/// The page that explains a rule code, like `A11Y-001`: why it's checked, and a page that breaks it and one that
/// doesn't.
pub fn rule_url(code: &str) -> String {
    format!("https://hypralang.dev/rules/{}/", code.to_ascii_lowercase())
}

/// Markdown about an element or statement, like `field` or `for`: what its section in the spec says about it, with
/// the section's first example. Plain HTML elements get their Hypra rule, if any, and a link to MDN.
pub fn element(name: &str) -> Option<String> {
    let heading = match name {
        "page" => "The page",
        "props" => "Data: `props`",
        "head" => "`head`",
        "form" | "field" => "Forms",
        "action" => "Actions",
        "raw" | "allow" => "Escape hatches",
        "use" => "`use`",
        "component" | "slot" => "`component`",
        "layout" => "`layout`",
        "for" => "`for`",
        "if" => "`if`",
        _ if crate::html::TAGS.contains(&name) => {
            let mdn = format!("[MDN reference](https://developer.mozilla.org/docs/Web/HTML/Element/{name})");
            return Some(match rule_row(name) {
                Some((element, rule)) => format!("**{element}** {rule}\n\n{mdn}"),
                None => format!("The HTML element `<{name}>`.\n\n{mdn}"),
            });
        }
        _ => return None,
    };
    let blocks = section(heading);
    let own = |b: &&Block| matches!(b, Block::Prose(t) if t.starts_with(&format!("`{name}(")));
    let prose = blocks.iter().find(own).or_else(|| blocks.iter().find(|b| matches!(b, Block::Prose(_))));
    let example = blocks.iter().find(|b| matches!(b, Block::Code(_)));
    let mut out = format!("**`{name}`**");
    for block in [prose, example].into_iter().flatten() {
        match block {
            Block::Prose(text) => out.push_str(&format!("\n\n{text}")),
            Block::Code(code) => out.push_str(&format!("\n\n```hypra\n{code}\n```")),
        }
    }
    Some(format!("{out}\n\n[Docs]({DOCS}#{})", slug(heading)))
}

/// The row of the spec's table of elements with Hypra rules, like `img(src, alt: "...")` and what it needs.
fn rule_row(name: &str) -> Option<(&'static str, &'static str)> {
    let table = SPEC.lines().skip_while(|l| *l != "Elements with Hypra rules:").skip(2);
    table.take_while(|l| l.starts_with('|')).find_map(|line| {
        let (element, rule) = line.strip_prefix("| ")?.strip_suffix(" |")?.split_once(" | ")?;
        let written = element.trim_matches('`').split(['(', ' ']).next()?;
        (written == name).then_some((element, rule))
    })
}

/// Markdown about a field type, like `Email`: its row of the spec's table of field types.
pub fn field_type(name: &str) -> Option<String> {
    let row = SPEC.lines().find(|line| {
        line.starts_with("| `")
            && line.split('|').nth(1).is_some_and(|cell| {
                cell.split(", ").any(|t| t.trim().trim_matches('`').split('(').next() == Some(name))
            })
    })?;
    let cells: Vec<&str> = row.split('|').map(str::trim).filter(|c| !c.is_empty()).collect();
    Some(format!("**Field type** {}: {}\n\n[Docs]({DOCS}#forms)", cells.first()?, cells.get(1)?))
}

enum Block {
    /// A paragraph, or one item of a list without its dash.
    Prose(String),
    Code(String),
}

/// The paragraphs, list items and code of a section of the spec, without its tables.
fn section(heading: &str) -> Vec<Block> {
    let mut lines = SPEC.lines().skip_while(|l| !(l.starts_with('#') && l.trim_start_matches('#').trim() == heading));
    lines.next();
    let mut blocks = vec![];
    let mut prose: Vec<&str> = vec![];
    let mut code: Option<Vec<&str>> = None;
    let flush = |prose: &mut Vec<&str>, blocks: &mut Vec<Block>| {
        if !prose.is_empty() {
            blocks.push(Block::Prose(std::mem::take(prose).join("\n")));
        }
    };
    for line in lines.take_while(|l| !l.starts_with('#')) {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            match code.take() {
                Some(lines) => blocks.push(Block::Code(lines.join("\n"))),
                None => {
                    flush(&mut prose, &mut blocks);
                    code = Some(vec![]);
                }
            }
        } else if let Some(lines) = &mut code {
            lines.push(line);
        } else if trimmed.is_empty() || line.starts_with('|') {
            flush(&mut prose, &mut blocks);
        } else if let Some(item) = line.strip_prefix("- ") {
            flush(&mut prose, &mut blocks);
            prose.push(item);
        } else {
            prose.push(trimmed);
        }
    }
    flush(&mut prose, &mut blocks);
    blocks
}

/// The id the docs site gives a heading.
fn slug(heading: &str) -> String {
    let words: Vec<String> = heading
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect();
    words.join("-")
}
