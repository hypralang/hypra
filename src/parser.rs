//! Parses tokens into items. Hypra has one shape for elements and five statements:
//!
//! ```text
//! name(main, key: value, ...) "text" { children }      an element; every part is optional
//! use "file.hypra"
//! component Name(param: Type, ...) { body }
//! layout Name(setting: value, ...) { body }
//! for item in list { body }
//! if [not] value { body } [else { body }]
//! ```

use crate::diag::Diagnostic;
use crate::html;
use crate::lexer::{Pos, Tok, Token};

#[derive(Debug, Clone)]
pub enum ValueKind {
    Str(String),
    Num(String),
    Bool(bool),
    /// Data, e.g. `product.name`. Starts with a lowercase letter.
    Path(String),
    /// A layout, component or type, e.g. `Shop`. Starts with an uppercase letter.
    Name(String),
    /// A type with arguments, e.g. `Int(1..5)`.
    Call(String, Vec<Arg>),
    Range(Option<String>, Option<String>),
    /// A list of fixed values, e.g. `["S", "M", "L"]`.
    List(Vec<Value>),
}

#[derive(Debug, Clone)]
pub struct Value {
    pub kind: ValueKind,
    pub pos: Pos,
    pub len: usize,
}

impl Value {
    /// True for a value known when compiling: text without data, a number, true or false, or a list of them.
    pub fn is_fixed(&self) -> bool {
        match &self.kind {
            ValueKind::Str(text) => !reads_data(text),
            ValueKind::Num(_) | ValueKind::Bool(_) => true,
            ValueKind::List(items) => items.iter().all(Value::is_fixed),
            _ => false,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match &self.kind {
            ValueKind::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_path(&self) -> Option<&str> {
        match &self.kind {
            ValueKind::Path(p) => Some(p),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&str> {
        match &self.kind {
            ValueKind::Name(n) => Some(n),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Key {
    pub name: String,
    /// Written as a string, e.g. `"x-on:click"`: passed through as-is.
    pub quoted: bool,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct Arg {
    /// `None` for the unnamed main argument.
    pub key: Option<Key>,
    pub value: Value,
    /// Only in component parameters: `featured: Bool = false`.
    pub default: Option<Value>,
}

impl Arg {
    pub fn name(&self) -> Option<&str> {
        self.key.as_ref().map(|k| k.name.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct Element {
    pub name: String,
    pub pos: Pos,
    pub args: Vec<Arg>,
    pub value: Option<Value>,
    pub children: Vec<Item>,
}

impl Element {
    /// The unnamed first argument.
    pub fn main(&self) -> Option<&Value> {
        self.args.first().filter(|a| a.key.is_none()).map(|a| &a.value)
    }

    /// The unnamed first argument, or else the one named `key`: `a("/shoes")` or `a(href: "/shoes")`.
    pub fn main_or(&self, key: &str) -> Option<&Value> {
        self.main().or_else(|| self.arg(key))
    }

    pub fn arg(&self, key: &str) -> Option<&Value> {
        self.args.iter().find(|a| a.name() == Some(key)).map(|a| &a.value)
    }

    /// True when `key: true` is set.
    pub fn flag(&self, key: &str) -> bool {
        matches!(self.arg(key).map(|v| &v.kind), Some(ValueKind::Bool(true)))
    }
}

/// `component Name(...) { ... }` or `layout Name(...) { ... }`.
#[derive(Debug, Clone)]
pub struct Def {
    pub name: String,
    pub pos: Pos,
    pub name_pos: Pos,
    pub params: Vec<Arg>,
    pub body: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct For {
    pub var: String,
    pub var_pos: Pos,
    pub pos: Pos,
    pub list: Value,
    pub body: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct If {
    pub pos: Pos,
    pub negate: bool,
    pub cond: Value,
    /// The fixed value in `if user.role == "admin"`; `!=` sets `negate`.
    pub equals: Option<Value>,
    pub then: Vec<Item>,
    pub otherwise: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Element(Element),
    /// A string on its own: a run of text, e.g. inside `p { "Weighs " b "220 g" }`.
    Text(Value),
    Use(Value),
    Component(Def),
    Layout(Def),
    For(For),
    If(If),
}

/// The elements with this name directly in the given item lists, in order.
pub fn elements_named<'a>(lists: impl IntoIterator<Item = &'a [Item]>, name: &str) -> Vec<&'a Element> {
    lists
        .into_iter()
        .flatten()
        .filter_map(|item| match item {
            Item::Element(e) if e.name == name => Some(e),
            _ => None,
        })
        .collect()
}

/// Whether any element in these items passes `test`, also among children and inside `for` and `if`.
pub fn any_element(items: &[Item], test: &dyn Fn(&Element) -> bool) -> bool {
    items.iter().any(|item| match item {
        Item::Element(e) => test(e) || any_element(&e.children, test),
        Item::For(f) => any_element(&f.body, test),
        Item::If(c) => any_element(&c.then, test) || any_element(&c.otherwise, test),
        _ => false,
    })
}

impl Item {
    /// The rules of the `allow`s at the start of a page, which cover the whole page.
    pub fn page_allows(items: &[Item]) -> impl Iterator<Item = String> + '_ {
        items.iter().map_while(Item::allowed_rule).filter_map(|rule| rule.as_str().map(String::from))
    }

    /// The rule an `allow("SEO-003") "reason"` switches off for the next item.
    pub fn allowed_rule(&self) -> Option<&Value> {
        match self {
            Item::Element(e) if e.name == "allow" => e.main_or("rule"),
            _ => None,
        }
    }
}

/// A name that `{...}` reads inside a string: `product.name`, `review`.
pub fn is_data_name(text: &str) -> bool {
    !text.is_empty()
        && text.split('.').all(|part| {
            part.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

/// True if a string reads data with `{name}`, so its final text is only known at render time.
pub fn reads_data(text: &str) -> bool {
    pieces(text, Braces::Doubled).iter().any(|p| matches!(p, Piece::Data { .. }))
}

/// How a string treats doubled braces.
#[derive(Clone, Copy, PartialEq)]
pub enum Braces {
    /// `{{` and `}}` write one brace, as in any quoted value.
    Doubled,
    /// Every brace is text unless it encloses a name, as in JSON scripts, where `}}` ends two objects.
    AsWritten,
}

/// A piece of a quoted string: text, or a `{name}` that reads data.
#[derive(Debug, PartialEq)]
pub enum Piece<'t> {
    Text(String),
    /// `at` is the byte offset of the `{` in the string.
    Data {
        name: &'t str,
        at: usize,
    },
}

/// Splits a string into text and data: `"Hi {user.name}!"` is `Hi `, `user.name`, `!`. Only braces around a valid
/// name read data, so `"{ open: false }"` is text.
pub fn pieces(text: &str, braces: Braces) -> Vec<Piece<'_>> {
    let mut out = vec![];
    let mut literal = String::new();
    let mut i = 0;
    while let Some(offset) = text[i..].find(['{', '}']) {
        let start = i + offset;
        literal.push_str(&text[i..start]);
        let brace = &text[start..=start];
        let after = &text[start + 1..];
        if braces == Braces::Doubled && after.starts_with(brace) {
            literal.push_str(brace);
            i = start + 2;
            continue;
        }
        match after.split_once('}').map(|(name, _)| name).filter(|name| brace == "{" && is_data_name(name)) {
            Some(name) => {
                out.push(Piece::Text(std::mem::take(&mut literal)));
                out.push(Piece::Data { name, at: start });
                i = start + name.len() + 2;
            }
            None => {
                literal.push_str(brace);
                i = start + 1;
            }
        }
    }
    literal.push_str(&text[i..]);
    out.push(Piece::Text(literal));
    out
}

/// Where the piece at byte `at` of a string value is written. A one-line string is its text between two quotes, with
/// each escaped character (`\"`, `\\`, `\n`, `\t`) written as two; a `"""` string drops its indentation, so its
/// pieces can't be located and this is `None`.
pub fn piece_pos(value: &Value, at: usize) -> Option<Pos> {
    let ValueKind::Str(text) = &value.kind else { return None };
    let written =
        |s: &str| -> usize { s.chars().map(|c| if matches!(c, '"' | '\\' | '\n' | '\t') { 2 } else { 1 }).sum() };
    (value.len == written(text) + 2).then(|| Pos { col: value.pos.col + 1 + written(&text[..at]), ..value.pos })
}

/// Components and layouts are capitalized.
pub fn is_name(word: &str) -> bool {
    word.starts_with(|c: char| c.is_ascii_uppercase())
}

/// Blocks and arguments nested deeper than this are rejected instead of exhausting the stack.
const MAX_NESTING: usize = 256;

pub fn parse(toks: Vec<Token>) -> Result<Vec<Item>, Diagnostic> {
    Parser { toks, i: 0, depth: 0, misaligned: None }.items(None)
}

struct Parser {
    toks: Vec<Token>,
    i: usize,
    /// How many blocks and argument lists the parser is inside.
    depth: usize,
    /// The first block whose `}` isn't indented like the line that opened it: when a `}` is missing, it's
    /// most likely this block's.
    misaligned: Option<Pos>,
}

impl Parser {
    fn tok(&self) -> &Token {
        &self.toks[self.i]
    }

    fn peek(&self) -> &Tok {
        &self.toks[self.i].tok
    }

    fn peek_at(&self, n: usize) -> &Tok {
        &self.toks[(self.i + n).min(self.toks.len() - 1)].tok
    }

    fn next(&mut self) -> Token {
        let t = self.toks[self.i].clone();
        if self.i < self.toks.len() - 1 {
            self.i += 1;
        }
        t
    }

    fn err<T>(&self, message: impl Into<String>) -> Result<T, Diagnostic> {
        let t = self.tok();
        Err(Diagnostic::error("PARSE-001", message, t.pos, t.len))
    }

    fn skip_newlines(&mut self) {
        while *self.peek() == Tok::Newline {
            self.next();
        }
    }

    fn word(&self) -> Option<&str> {
        match self.peek() {
            Tok::Ident(w) => Some(w),
            _ => None,
        }
    }

    /// The items of a file, or of a block whose `{` is at `open`.
    fn items(&mut self, open: Option<Pos>) -> Result<Vec<Item>, Diagnostic> {
        let top = open.is_none();
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek() {
                Tok::Eof if top => break,
                Tok::Eof => {
                    let open = self.misaligned.unwrap_or(open.expect("a block has an opening brace"));
                    return Err(Diagnostic::error("PARSE-001", "this `{` is never closed", open, 1)
                        .help("add the matching `}` where the block ends"));
                }
                Tok::LBrace => {
                    return self
                        .err("expected an element, found `{`")
                        .map_err(|d| d.help("`{` goes on the same line as the element it opens: p \"Hi\" {"));
                }
                Tok::RBrace if !top => break,
                Tok::RBrace => return self.err("unexpected `}`"),
                _ => items.push(self.item()?),
            }
        }
        Ok(items)
    }

    fn item(&mut self) -> Result<Item, Diagnostic> {
        let next = self.peek_at(1).clone();
        match (self.peek().clone(), next) {
            (Tok::Str(_), _) => Ok(Item::Text(self.value()?)),
            (Tok::Ident(w), Tok::Str(_)) if w == "use" => {
                self.next();
                Ok(Item::Use(self.value()?))
            }
            (Tok::Ident(w), Tok::Ident(_)) if w == "component" => Ok(Item::Component(self.def()?)),
            (Tok::Ident(w), Tok::Ident(_)) if w == "layout" => Ok(Item::Layout(self.def()?)),
            (Tok::Ident(w), Tok::Ident(_)) if w == "for" => self.for_loop(),
            (Tok::Ident(w), _) if w == "if" => self.condition(),
            (Tok::Ident(_), _) => Ok(Item::Element(self.element()?)),
            (other, _) => self.err(format!("expected an element, found {}", describe(&other))),
        }
    }

    fn element(&mut self) -> Result<Element, Diagnostic> {
        let head = self.next();
        let Tok::Ident(name) = head.tok else { unreachable!("element() starts at an identifier") };
        let args = if *self.peek() == Tok::LParen { self.args()? } else { vec![] };
        // a void element has no text, so `br "next line"` is a br followed by text
        let mut value = match self.peek() {
            Tok::Str(_) | Tok::Num(_) if !html::is_void(&name) => Some(self.value()?),
            _ => None,
        };
        let mut children = if *self.peek() == Tok::LBrace { self.block()? } else { vec![] };
        // the text of a component call is its slot content, like the text of an element is its content
        if is_name(&name)
            && let Some(text) = value.take()
        {
            children.insert(0, Item::Text(text));
        }
        Ok(Element { name, pos: head.pos, args, value, children })
    }

    fn def(&mut self) -> Result<Def, Diagnostic> {
        let keyword = self.next();
        let name_tok = self.next();
        let Tok::Ident(name) = name_tok.tok else { unreachable!("def() is called before a name") };
        let params = if *self.peek() == Tok::LParen { self.args()? } else { vec![] };
        if *self.peek() != Tok::LBrace {
            return self.err("expected `{` to start the body");
        }
        let body = self.block()?;
        Ok(Def { name, pos: keyword.pos, name_pos: name_tok.pos, params, body })
    }

    fn for_loop(&mut self) -> Result<Item, Diagnostic> {
        let keyword = self.next();
        let var_tok = self.next();
        let Tok::Ident(var) = var_tok.tok else { unreachable!("for_loop() is called before a name") };
        if self.word() != Some("in") {
            return self.err("expected `in`, as in `for item in list { ... }`");
        }
        self.next();
        let list = self.value()?;
        if *self.peek() != Tok::LBrace {
            return self.err("expected `{` to start the loop body");
        }
        let body = self.block()?;
        Ok(Item::For(For { var, var_pos: var_tok.pos, pos: keyword.pos, list, body }))
    }

    fn condition(&mut self) -> Result<Item, Diagnostic> {
        let keyword = self.next();
        let negate = self.word() == Some("not");
        if negate {
            self.next();
        }
        let cond = self.value()?;
        let mut negate = negate;
        let equals = match self.peek() {
            Tok::EqEq | Tok::NotEq if negate => {
                return self.err("use `!=` instead of `not` with a comparison");
            }
            Tok::EqEq | Tok::NotEq => {
                negate = *self.peek() == Tok::NotEq;
                self.next();
                let value = self.value()?;
                if !value.is_fixed() || matches!(value.kind, ValueKind::List(_)) {
                    return Err(Diagnostic::error("PARSE-001", "compare with fixed text, a number, true or false", value.pos, value.len)
                        .help("prepare comparisons between two pieces of data in the controller, e.g. if user.is_owner { ... }"));
                }
                Some(value)
            }
            _ => None,
        };
        if *self.peek() != Tok::LBrace {
            return self.err("expected `{` after the condition");
        }
        let then = self.block()?;

        // `else` may follow on the same line or the next
        let before_else = self.i;
        self.skip_newlines();
        let otherwise = if self.word() == Some("else") {
            self.next();
            match self.peek() {
                Tok::LBrace => self.block()?,
                Tok::Ident(w) if w == "if" => vec![self.condition()?],
                _ => return self.err("expected `{` or `if` after `else`"),
            }
        } else {
            self.i = before_else;
            vec![]
        };
        Ok(Item::If(If { pos: keyword.pos, negate, cond, equals, then, otherwise }))
    }

    fn block(&mut self) -> Result<Vec<Item>, Diagnostic> {
        self.enter()?;
        let indent = self.indent_of_line(self.i);
        let open = self.next().pos;
        let items = self.items(Some(open))?;
        let close = self.i;
        if self.misaligned.is_none() && self.starts_line(close) && self.toks[close].pos.col != indent {
            self.misaligned = Some(open);
        }
        self.next();
        self.depth -= 1;
        Ok(items)
    }

    /// The column of the first token on the line of token `i`.
    fn indent_of_line(&self, i: usize) -> usize {
        let line = self.toks[i].pos.line;
        self.toks[..=i].iter().rev().take_while(|t| t.pos.line == line).last().map_or(1, |t| t.pos.col)
    }

    fn starts_line(&self, i: usize) -> bool {
        i == 0 || self.toks[i - 1].pos.line != self.toks[i].pos.line
    }

    fn enter(&mut self) -> Result<(), Diagnostic> {
        self.depth += 1;
        if self.depth > MAX_NESTING {
            let t = self.tok();
            let message = format!("nested more than {MAX_NESTING} levels deep");
            return Err(Diagnostic::error("PARSE-002", message, t.pos, t.len).help("split the page into components"));
        }
        Ok(())
    }

    fn args(&mut self) -> Result<Vec<Arg>, Diagnostic> {
        self.enter()?;
        self.next();
        let mut args = Vec::new();
        loop {
            if *self.peek() == Tok::RParen {
                self.next();
                break;
            }
            let key = match (self.peek().clone(), self.peek_at(1)) {
                (Tok::Ident(name), Tok::Colon) | (Tok::Str(name), Tok::Colon) => {
                    let quoted = matches!(self.peek(), Tok::Str(_));
                    let pos = self.next().pos;
                    self.next();
                    // HTML attribute names ignore case, so "HREF" is href and gets href's checks
                    let name = if quoted { name.to_ascii_lowercase() } else { name };
                    Some(Key { name, quoted, pos })
                }
                _ => None,
            };
            let value = self.value()?;
            let default = if *self.peek() == Tok::Eq {
                self.next();
                Some(self.value()?)
            } else {
                None
            };
            args.push(Arg { key, value, default });
            match self.peek() {
                Tok::Comma => {
                    self.next();
                }
                Tok::RParen => {}
                other => return self.err(format!("expected `,` or `)`, found {}", describe(other))),
            }
        }
        self.depth -= 1;
        Ok(args)
    }

    fn value(&mut self) -> Result<Value, Diagnostic> {
        let start = self.tok().clone();
        let kind = match start.tok.clone() {
            Tok::Str(s) => {
                self.next();
                ValueKind::Str(s)
            }
            Tok::Num(lo) => {
                self.next();
                if *self.peek() == Tok::DotDot {
                    self.next();
                    let hi = match self.peek().clone() {
                        Tok::Num(hi) => {
                            self.next();
                            Some(hi)
                        }
                        _ => None,
                    };
                    ValueKind::Range(Some(lo), hi)
                } else {
                    ValueKind::Num(lo)
                }
            }
            Tok::DotDot => {
                self.next();
                match self.peek().clone() {
                    Tok::Num(hi) => {
                        self.next();
                        ValueKind::Range(None, Some(hi))
                    }
                    _ => return self.err("expected a number after `..`"),
                }
            }
            Tok::Ident(word) => {
                self.next();
                match word.as_str() {
                    "true" => ValueKind::Bool(true),
                    "false" => ValueKind::Bool(false),
                    _ if *self.peek() == Tok::LParen => ValueKind::Call(word, self.args()?),
                    _ if is_name(&word) => ValueKind::Name(word),
                    _ => ValueKind::Path(word),
                }
            }
            Tok::LBracket => {
                self.enter()?;
                self.next();
                let mut items = vec![];
                while *self.peek() != Tok::RBracket {
                    items.push(self.value()?);
                    match self.peek() {
                        Tok::Comma => {
                            self.next();
                        }
                        Tok::RBracket => {}
                        other => return self.err(format!("expected `,` or `]`, found {}", describe(&other.clone()))),
                    }
                }
                self.next();
                self.depth -= 1;
                ValueKind::List(items)
            }
            other => return self.err(format!("expected a value, found {}", describe(&other))),
        };
        let end = &self.toks[self.i.saturating_sub(1)];
        let len = if end.pos.line == start.pos.line { end.pos.col + end.len - start.pos.col } else { start.len };
        Ok(Value { kind, pos: start.pos, len })
    }
}

fn describe(tok: &Tok) -> String {
    match tok {
        Tok::Ident(s) => format!("`{s}`"),
        Tok::Str(_) => "a string".into(),
        Tok::Num(n) => format!("`{n}`"),
        Tok::LBrace => "`{`".into(),
        Tok::RBrace => "`}`".into(),
        Tok::LParen => "`(`".into(),
        Tok::RParen => "`)`".into(),
        Tok::Colon => "`:`".into(),
        Tok::Comma => "`,`".into(),
        Tok::DotDot => "`..`".into(),
        Tok::Eq => "`=`".into(),
        Tok::LBracket => "`[`".into(),
        Tok::RBracket => "`]`".into(),
        Tok::EqEq => "`==`".into(),
        Tok::NotEq => "`!=`".into(),
        Tok::Newline => "end of line".into(),
        Tok::Comment(_) => "a comment".into(),
        Tok::Eof => "end of file".into(),
    }
}
