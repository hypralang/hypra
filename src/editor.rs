//! What editors ask about a page: where a name is defined, where it's used, how to rename it, and what to show on
//! hover. Positions count lines and characters from 1, like the compiler's diagnostics. Each function loads the
//! page and its imports through `load`, so an editor can pass its unsaved text.

use crate::nav::{self, Hit, Target};
use crate::{Project, Source, defs, help, lexer::Pos, line_of, load_project, normalize};
use std::path::{Path, PathBuf};

type Load<'a> = &'a dyn Fn(&str) -> Result<String, String>;

/// A name in a file: its position and length.
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub path: String,
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

/// Where the name at `line`:`col` of the file at `entry` is defined. A definition is its own definition. Works on
/// pages with errors too, as long as the files involved parse.
pub fn definition(entry: &str, line: usize, col: usize, load: Load) -> Option<Location> {
    let Project { modules, sources, .. } = load_project(entry, load);
    let (defs, _) = defs::collect(&modules);
    match nav::find(&modules, &defs, Pos { file: 0, line, col })?.hit {
        Hit::Name(target) => locate(&sources, target),
        _ => None,
    }
}

/// Every place the name at `line`:`col` of `entry` is defined and used, looking through `files`: the project's
/// `.hypra` files, since a component doesn't know which pages use it.
pub fn references(entry: &str, line: usize, col: usize, files: &[PathBuf], load: Load) -> Vec<Location> {
    let Some(target) = definition(entry, line, col, load).filter(|t| t.len > 0) else { return vec![] };
    let mut found = vec![];
    let entry = PathBuf::from(entry);
    for file in std::iter::once(&entry).chain(files.iter().filter(|f| **f != entry)) {
        let Project { modules, sources, .. } = load_project(&file.to_string_lossy(), load);
        let (defs, _) = defs::collect(&modules);
        let (Some(items), Some(source)) = (modules.first(), sources.first()) else { continue };
        for mention in nav::mentions(items, &defs) {
            let Hit::Name(to) = mention.hit else { continue };
            if locate(&sources, to).as_ref() == Some(&target) {
                let here = Location {
                    path: source.path.clone(),
                    line: mention.pos.line,
                    col: mention.pos.col,
                    len: mention.len,
                };
                if !found.contains(&here) {
                    found.push(here);
                }
            }
        }
    }
    found
}

/// The name at `line`:`col` of `entry` and where it's written, if it can be renamed.
pub fn renameable(entry: &str, line: usize, col: usize, load: Load) -> Option<(Location, String)> {
    let Project { modules, sources, .. } = load_project(entry, load);
    let (defs, _) = defs::collect(&modules);
    let mention = nav::find(&modules, &defs, Pos { file: 0, line, col })?;
    let Hit::Name(Target::At(..)) = mention.hit else { return None };
    let text = line_of(&sources.first()?.text, line);
    let name: String = text.chars().skip(mention.pos.col - 1).take(mention.len).collect();
    Some((Location { path: sources[0].path.clone(), line, col: mention.pos.col, len: mention.len }, name))
}

/// The places to change to rename the name at `line`:`col` of `entry` to `new_name`, across `files`; or why it
/// can't be renamed to that.
pub fn rename(
    entry: &str,
    line: usize,
    col: usize,
    new_name: &str,
    files: &[PathBuf],
    load: Load,
) -> Result<Vec<Location>, String> {
    let (_, old) =
        renameable(entry, line, col, load).ok_or("only components, layouts and data names can be renamed")?;
    let word = |rest: &str| rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    let valid = match old.chars().next() {
        // components and layouts are capitalized; data is lowercase
        Some(c) if c.is_ascii_uppercase() => new_name.starts_with(|c: char| c.is_ascii_uppercase()) && word(new_name),
        _ => new_name.starts_with(|c: char| c.is_ascii_lowercase() || c == '_') && word(new_name),
    };
    let reserved = ["true", "false", "in", "not", "for", "if", "else", "use", "component", "layout"];
    if !valid || reserved.contains(&new_name) {
        let kind = if crate::parser::is_name(&old) {
            "a capitalized name like `ProductCard`"
        } else {
            "a lowercase name like `product`"
        };
        return Err(format!("`{new_name}` can't replace `{old}`: use {kind}"));
    }
    Ok(references(entry, line, col, files, load))
}

/// What to show when the pointer rests at `line`:`col` of `entry`, as Markdown: a name's definition, or what an
/// element, statement or field type does.
pub fn hover(entry: &str, line: usize, col: usize, load: Load) -> Option<String> {
    let Project { modules, sources, .. } = load_project(entry, load);
    let (defs, _) = defs::collect(&modules);
    match nav::find(&modules, &defs, Pos { file: 0, line, col })?.hit {
        Hit::Name(target @ Target::At(pos, _)) => {
            let location = locate(&sources, target)?;
            let written = line_of(&sources.get(pos.file)?.text, pos.line).trim();
            let written = written.strip_suffix('{').unwrap_or(written).trim_end();
            let file = if pos.file == 0 {
                String::new()
            } else {
                let name = Path::new(&location.path).file_name()?.to_string_lossy().into_owned();
                format!("\n\nfrom `{name}`")
            };
            Some(format!("```hypra\n{written}\n```{file}"))
        }
        Hit::Name(Target::File(_)) => None,
        Hit::Element(name) => help::element(&name),
        Hit::Statement(keyword) => help::element(keyword),
        Hit::FieldType(name) => help::field_type(&name),
    }
}

/// Where a target is, as a path in the project.
fn locate(sources: &[Source], target: Target) -> Option<Location> {
    match target {
        Target::At(pos, len) => {
            Some(Location { path: sources.get(pos.file)?.path.clone(), line: pos.line, col: pos.col, len })
        }
        Target::File(relative) => {
            let dir = Path::new(&sources.first()?.path).parent().map(Path::to_path_buf).unwrap_or_default();
            let path = normalize(&dir.join(relative)).to_string_lossy().into_owned();
            Some(Location { path, line: 1, col: 1, len: 0 })
        }
    }
}
