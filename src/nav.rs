//! What the names in a file refer to, for editors: go to definition, find references, rename and hover. Components
//! and layouts are looked up across the page's imports; data names in the scopes a page gives them: `props`,
//! component parameters and loop variables. A definition refers to itself, so it can be renamed and its references
//! found from where it's written.

use crate::defs::{Defs, param_name};
use crate::lexer::Pos;
use crate::parser::{
    Arg, Braces, Def, Element, Item, Piece, Value, ValueKind, elements_named, is_name, piece_pos, pieces,
};

/// Where a name is defined.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// A name in one of the project's files, and its length.
    At(Pos, usize),
    /// A file, as a `use` writes it: relative to the importing file.
    File(String),
}

/// What's written somewhere in a file.
#[derive(Debug, Clone, PartialEq)]
pub enum Hit {
    /// A name with a definition in the project.
    Name(Target),
    /// The name of an element that isn't a component, like `img` or `field`.
    Element(String),
    /// The keyword of a statement: `component`, `layout`, `for` or `if`.
    Statement(&'static str),
    /// A field type, like `Email` in `field("email", type: Email)`.
    FieldType(String),
}

/// A hit and where it's written: `len` characters from `pos`.
#[derive(Debug, Clone, PartialEq)]
pub struct Mention {
    pub pos: Pos,
    pub len: usize,
    pub hit: Hit,
}

impl Mention {
    pub fn covers(&self, at: Pos) -> bool {
        self.pos.file == at.file
            && self.pos.line == at.line
            && (self.pos.col..self.pos.col + self.len).contains(&at.col)
    }
}

/// Whatever is written at `at`, in the file `at.file` of `modules`.
pub fn find(modules: &[Vec<Item>], defs: &Defs, at: Pos) -> Option<Mention> {
    mentions(modules.get(at.file)?, defs).into_iter().find(|m| m.covers(at))
}

/// Everything in a file that refers to something, in the order it's written.
pub fn mentions(items: &[Item], defs: &Defs) -> Vec<Mention> {
    let mut finder = Finder { defs, scopes: vec![], found: vec![] };
    finder.items(items);
    finder.found
}

struct Finder<'a, 'd> {
    defs: &'d Defs<'d>,
    /// Data names in scope and where they're defined, innermost last.
    scopes: Vec<(&'a str, Pos)>,
    found: Vec<Mention>,
}

impl<'a> Finder<'a, '_> {
    fn add(&mut self, pos: Pos, len: usize, hit: Hit) {
        self.found.push(Mention { pos, len, hit });
    }

    /// A name that defines itself, like a prop or a loop variable.
    fn definition(&mut self, name: &str, pos: Pos) {
        let len = name.chars().count();
        self.add(pos, len, Hit::Name(Target::At(pos, len)));
    }

    /// A data name like `product.name` written at `pos`: its first part refers to where `product` is defined.
    fn data(&mut self, path: &str, pos: Pos) {
        let first = path.split('.').next().unwrap_or_default();
        if let Some(&(name, defined)) = self.scopes.iter().rev().find(|(name, _)| *name == first) {
            let len = name.chars().count();
            self.add(pos, len, Hit::Name(Target::At(defined, len)));
        }
    }

    fn component_or_layout(&mut self, def: &Def) {
        self.add(def.name_pos, def.name.chars().count(), Hit::Name(Target::At(def.name_pos, def.name.chars().count())));
    }

    fn items(&mut self, items: &'a [Item]) {
        for item in items {
            match item {
                Item::Use(path) => {
                    if let Some(file) = path.as_str() {
                        self.add(path.pos, path.len, Hit::Name(Target::File(file.to_string())));
                    }
                }
                Item::Component(def) | Item::Layout(def) => {
                    let component = matches!(item, Item::Component(_));
                    let keyword = if component { "component" } else { "layout" };
                    self.add(def.pos, keyword.len(), Hit::Statement(keyword));
                    self.component_or_layout(def);
                    // a component sees only its parameters; a layout's parameters are settings, not data
                    let outer = std::mem::take(&mut self.scopes);
                    if component {
                        for param in &def.params {
                            let pos = param.key.as_ref().map_or(param.value.pos, |k| k.pos);
                            if let Some(name) = param_name(param) {
                                self.definition(name, pos);
                                self.scopes.push((name, pos));
                            }
                        }
                    } else {
                        self.settings(&def.params);
                    }
                    self.items(&def.body);
                    self.scopes = outer;
                }
                Item::Element(e) => self.element(e),
                Item::Text(text) => self.value(text),
                Item::For(f) => {
                    self.add(f.pos, 3, Hit::Statement("for"));
                    self.value(&f.list);
                    self.definition(&f.var, f.var_pos);
                    self.scopes.push((&f.var, f.var_pos));
                    self.items(&f.body);
                    self.scopes.pop();
                }
                Item::If(c) => {
                    self.add(c.pos, 2, Hit::Statement("if"));
                    self.value(&c.cond);
                    self.items(&c.then);
                    self.items(&c.otherwise);
                }
            }
        }
    }

    fn element(&mut self, e: &'a Element) {
        let len = e.name.chars().count();
        let component = if is_name(&e.name) { self.defs.components.get(&e.name).copied() } else { None };
        match component {
            Some(def) => {
                self.add(e.pos, len, Hit::Name(Target::At(def.name_pos, def.name.chars().count())));
                // an argument names one of the component's parameters
                for arg in &e.args {
                    let Some(key) = &arg.key else { continue };
                    let param = def.params.iter().find(|p| param_name(p) == Some(key.name.as_str()));
                    if let Some(param_pos) = param.and_then(|p| p.key.as_ref()).map(|k| k.pos) {
                        let len = key.name.chars().count();
                        self.add(key.pos, len, Hit::Name(Target::At(param_pos, len)));
                    }
                }
            }
            None if !is_name(&e.name) => self.add(e.pos, len, Hit::Element(e.name.clone())),
            None => {}
        }
        // the props of a page are in scope everywhere in it, also before they're declared
        let declared = self.scopes.len();
        if e.name == "page" {
            let props = elements_named([&e.children[..]], "props");
            for key in props.into_iter().flat_map(|p| &p.args).filter_map(|a| a.key.as_ref()) {
                self.scopes.push((&key.name, key.pos));
            }
        }
        if e.name == "props" {
            // `props(product: Product)` declares names; its types are labels for the data, defined nowhere
            for key in e.args.iter().filter_map(|a| a.key.as_ref()) {
                self.definition(&key.name, key.pos);
            }
        } else {
            self.settings(&e.args);
        }
        if let Some(text) = &e.value {
            self.value(text);
        }
        self.items(&e.children);
        self.scopes.truncate(declared);
    }

    /// Arguments, where `layout: Shop` names a layout and `type: Email` a field type.
    fn settings(&mut self, args: &'a [Arg]) {
        for arg in args {
            match (arg.name(), &arg.value.kind) {
                (Some("layout"), ValueKind::Name(name)) => {
                    if let Some(def) = self.defs.layouts.get(name) {
                        let target = Target::At(def.name_pos, def.name.chars().count());
                        self.add(arg.value.pos, arg.value.len, Hit::Name(target));
                    }
                }
                (Some("type"), ValueKind::Name(name) | ValueKind::Call(name, _)) => {
                    self.add(arg.value.pos, name.chars().count(), Hit::FieldType(name.clone()));
                }
                _ => self.value(&arg.value),
            }
        }
    }

    fn value(&mut self, value: &'a Value) {
        match &value.kind {
            ValueKind::Path(path) => self.data(path, value.pos),
            ValueKind::Str(text) => {
                for piece in pieces(text, Braces::Doubled) {
                    if let Piece::Data { name, at } = piece
                        && let Some(pos) = piece_pos(value, at)
                    {
                        self.data(name, Pos { col: pos.col + 1, ..pos });
                    }
                }
            }
            ValueKind::List(items) => items.iter().for_each(|item| self.value(item)),
            _ => {}
        }
    }
}
