//! Lowers a checked page to an IR [`Program`]: resolves names, inlines components and layouts,
//! and turns every element into HTML and Markdown output operations.
//!
//! HTML elements all go through one path ([`Lowerer::element`]). Only elements with Hypra
//! meaning (`img`, `video`, `form`, `field`, `action`, `raw`) are handled specially.

use crate::defs::{Defs, chain_setting, forward_root, given_content, layout_chain, param_name};
use crate::diag::{self, Diagnostic, suggest};
use crate::elements::{self, FieldArgs};
use crate::html;
use crate::ir::{ActionDef, Channel, Escape, Expr, Meta, Op, ParamDef, Program, Prop, apply, display, esc};
use crate::lexer::Pos;
use crate::parser::{
    Arg, Braces, Def, Element, If, Item, Piece, Value, ValueKind, any_element, elements_named, is_name, piece_pos,
    pieces,
};
use serde_json::{Value as Json, json};
use std::collections::HashMap;

/// Pages that expand to more elements than this (components inside components) are rejected.
const MAX_ELEMENTS: usize = 200_000;

/// The components these items call, directly or through other components, each once.
fn components_used<'a>(items: &'a [Item], components: &HashMap<String, &'a Def>, used: &mut Vec<&'a Def>) {
    for item in items {
        match item {
            Item::Element(e) => {
                if let Some(def) = components.get(&e.name)
                    && !used.iter().any(|u| std::ptr::eq(*u, *def))
                {
                    used.push(def);
                    components_used(&def.body, components, used);
                }
                components_used(&e.children, components, used);
            }
            Item::For(f) => components_used(&f.body, components, used),
            Item::If(c) => {
                components_used(&c.then, components, used);
                components_used(&c.otherwise, components, used);
            }
            _ => {}
        }
    }
}

/// The tags in a head that replace one Hypra writes: `meta(name: "viewport", ...)` replaces the viewport tag.
fn head_keys(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Element(m) if m.name == "meta" && m.arg("charset").is_some() => Some("charset".to_string()),
            Item::Element(m) if m.name == "meta" => {
                m.arg("name").or(m.arg("property")).and_then(Value::as_str).map(String::from)
            }
            Item::Element(l) if l.name == "link" && l.arg("rel").and_then(Value::as_str) == Some("canonical") => {
                Some("canonical".to_string())
            }
            _ => None,
        })
        .collect()
}

/// The rules `allow`ed right before the page or at its start, which apply to the whole page.
fn allowed_before_page(items: &[Item]) -> Vec<String> {
    let page = items.iter().position(|i| matches!(i, Item::Element(e) if e.name == "page")).unwrap_or(0);
    let before = items[..page].iter().rev().map_while(Item::allowed_rule).filter_map(|r| r.as_str().map(String::from));
    let inside = match items.get(page) {
        Some(Item::Element(p)) => Item::page_allows(&p.children).collect(),
        _ => vec![],
    };
    before.chain(inside).collect()
}

pub fn lower<'a>(items: &'a [Item], defs: &Defs<'a>, stem: &str) -> Result<Program, Vec<Diagnostic>> {
    let page = elements_named([items], "page")[0];
    let props: Vec<Prop> = elements_named([&page.children[..]], "props")
        .into_iter()
        .flat_map(|e| &e.args)
        .filter_map(|a| a.key.as_ref().map(|k| Prop { name: k.name.clone(), pos: k.pos }))
        .collect();
    let mut l = Lowerer::new(defs, props.iter().map(|p| p.name.clone()).collect());

    let chain = layout_chain(defs, page.arg("layout"));
    let settings = Settings::new(&mut l, page, &chain);
    // the outermost layout renders first; each slot shows the layout inside it, and the innermost one the page
    let body = match chain.split_last() {
        Some((outermost, inner)) => {
            l.slots.push(SlotFrame { children: &page.children, scopes: vec![], root_visible: true });
            for layout in inner {
                l.slots.push(SlotFrame { children: &layout.body, scopes: vec![], root_visible: true });
            }
            let body = l.block(&outermost.body, 1);
            l.slots.clear();
            body
        }
        None => l.block(&page.children, 1),
    };

    l.out.push(vec![]);
    // the outermost layout's head and body come first, the page's last
    let sources: Vec<&[Item]> = chain.iter().rev().map(|layout| &layout.body[..]).chain([&page.children[..]]).collect();
    l.document_start(page, &sources, &settings, defs, stem);
    l.front_matter(&settings);
    let mut ops = l.out.pop().expect("document frame");
    ops.extend(body);
    ops.push(Op::Html("</body>\n</html>\n".into()));

    // HTML and Markdown read the same values, so a bad one is reported once
    let diags = diag::unique(l.diags);
    if diags.iter().any(Diagnostic::is_error) {
        return Err(diags);
    }
    let at = |v: Option<&Value>| v.map(|v| (v.pos, v.len));
    let setting = |key: &str| page.arg(key).or_else(|| chain_setting(&chain, key));
    Ok(Program {
        ops,
        vars: l.vars,
        props,
        meta: Meta {
            title: settings.title,
            full_title: settings.full_title,
            title_at: at(page.arg("title")),
            description: settings.description,
            description_at: at(setting("description")),
            lang: settings.lang,
            lang_at: at(setting("lang")),
            canonical: settings.canonical,
            canonical_at: at(page.arg("canonical")),
            allowed: allowed_before_page(items),
        },
    })
}

/// The page's settings, with the layout's as defaults.
struct Settings {
    title: Expr,
    /// The title with the layout's `title_suffix`, as shown in `<title>`.
    full_title: Expr,
    description: Option<Expr>,
    image: Option<Expr>,
    canonical: Option<Expr>,
    robots: Option<Expr>,
    site_name: Option<Expr>,
    lang: Expr,
}

impl Settings {
    fn new(l: &mut Lowerer, page: &Element, chain: &[&Def]) -> Settings {
        let setting = |key: &str| page.arg(key).or_else(|| chain_setting(chain, key));
        let title = l.expr_or_empty(page.arg("title"));
        let full_title = match chain_setting(chain, "title_suffix") {
            Some(suffix) => Expr::concat(vec![title.clone(), l.expr(suffix)]),
            None => title.clone(),
        };
        Settings {
            title,
            full_title,
            description: setting("description").map(|v| l.expr(v)),
            image: setting("image").map(|v| l.expr(v)),
            canonical: page.arg("canonical").map(|v| l.expr(v)),
            robots: setting("robots").map(|v| l.expr(v)),
            site_name: chain_setting(chain, "site_name").map(|v| l.expr(v)),
            lang: setting("lang").map_or(Expr::Lit(json!("en")), |v| l.expr(v)),
        }
    }

    /// The head tags Hypra writes, each with the name of the tag that replaces it, e.g. `og:title`.
    fn head_tags(&self) -> Vec<(String, Vec<Part>)> {
        let meta = |kind: &str, key: &str, value: &Expr| {
            (key.to_string(), vec![fixed(&format!("<meta {kind}=\"{key}\" content=\"")), escaped(value), fixed("\">")])
        };
        let mut tags = vec![
            ("charset".to_string(), vec![fixed("<meta charset=\"utf-8\">")]),
            meta("name", "viewport", &Expr::Lit(json!("width=device-width, initial-scale=1"))),
            ("title".to_string(), vec![fixed("<title>"), escaped(&self.full_title), fixed("</title>")]),
        ];
        tags.extend(self.description.as_ref().map(|v| meta("name", "description", v)));
        tags.extend(self.robots.as_ref().map(|v| meta("name", "robots", v)));
        if let Some(c) = &self.canonical {
            tags.push((
                "canonical".to_string(),
                vec![fixed("<link rel=\"canonical\" href=\""), escaped(c), fixed("\">")],
            ));
        }
        tags.extend(self.site_name.as_ref().map(|v| meta("property", "og:site_name", v)));
        tags.push(meta("property", "og:title", &self.title));
        for (value, property) in
            [(&self.description, "og:description"), (&self.image, "og:image"), (&self.canonical, "og:url")]
        {
            tags.extend(value.as_ref().map(|v| meta("property", property, v)));
        }
        let card = if self.image.is_some() { "summary_large_image" } else { "summary" };
        tags.push(meta("name", "twitter:card", &Expr::Lit(json!(card))));
        tags
    }
}

/// HTML attributes a component call adds to the component's root element.
struct Forward<'a> {
    root: &'a Element,
    /// Joined with the root's own classes.
    class: Option<Expr>,
    attrs: Vec<Part>,
}

/// The parameter of a component with this name, if any.
fn param_name_of<'d>(def: &'d Def, name: &str) -> Option<&'d str> {
    def.params.iter().filter_map(param_name).find(|p| *p == name)
}

/// What a `slot` renders: the children a component or layout was given, in the caller's scope.
struct SlotFrame<'a> {
    children: &'a [Item],
    scopes: Vec<(String, usize)>,
    root_visible: bool,
}

struct Lowerer<'a> {
    components: HashMap<String, &'a Def>,
    props: Vec<String>,
    /// Op lists being built; loops and conditions push a new list for their body.
    out: Vec<Vec<Op>>,
    /// Names in scope (loop variables, component parameters) and their slots, innermost last.
    scopes: Vec<(String, usize)>,
    /// False inside components: they only see their parameters, not the page data.
    root_visible: bool,
    slots: Vec<SlotFrame<'a>>,
    /// Enclosing lists, innermost last; `true` for numbered (`ol`) lists.
    lists: Vec<bool>,
    /// Enclosing tables: whether the Markdown header separator is written, and the loop depth at the table.
    tables: Vec<(bool, usize)>,
    /// How many `for` loops are being lowered; a row inside a loop repeats, so it can't carry the header.
    loops: usize,
    /// Components being expanded, innermost last, to stop a component that calls itself.
    calling: Vec<String>,
    /// Elements lowered so far, to stop pages that expand without bound.
    elements: usize,
    vars: usize,
    /// Above zero where content is left out of the Markdown: site chrome, head, forms, scripts.
    md_muted: usize,
    has_h1: bool,
    /// Resolving a `{name}` inside quoted text, where the fix for a mistake differs.
    in_text: bool,
    /// Attributes waiting for the root element of the components being expanded, innermost last.
    forwards: Vec<Forward<'a>>,
    /// How many HTML `template` elements enclose the current one; inside them `slot` is the HTML element.
    html_templates: usize,
    /// How many table cells enclose the current element.
    cells: usize,
    /// How many `article`, `section` and `aside` elements enclose the current one.
    sections: usize,
    /// The fields of the enclosing form, which become its parameters.
    fields: Vec<Vec<ParamDef>>,
    diags: Vec<Diagnostic>,
}

/// A piece of output: static text, a value (escaped, as JSON, or raw), or text written only when a value is truthy.
enum Part {
    /// Text known now, written as it is.
    Fixed(String),
    /// A value, escaped for the output: HTML-escaped, or Markdown-escaped.
    Text(Expr),
    /// A value as a JSON string.
    Json(Expr),
    /// A value written unescaped, from `raw`.
    Raw(Expr),
    /// A URL: one that would run code is replaced at render time.
    Url(Expr),
    /// Text inside `<script>` or `<style>`.
    Code(Expr),
    /// Data inside a JSON script, written as a JSON string.
    ScriptJson(Expr),
    /// Output already lowered for this channel: an `if` or `for` inside a line of text.
    Ops(Vec<Op>),
    /// Text inside a Markdown code span, or a fenced block when true.
    MdCode(Expr, bool),
    When(Expr, String),
}

fn fixed(text: &str) -> Part {
    Part::Fixed(text.to_string())
}

fn escaped(expr: &Expr) -> Part {
    Part::Text(expr.clone())
}

/// A Markdown code span around inline parts. Text from data could hold backticks, so it gets a wider span.
fn code_span(inner: Vec<Part>) -> Vec<Part> {
    let plain = inner.iter().all(|p| match p {
        Part::Fixed(text) | Part::Text(Expr::Lit(Json::String(text))) => !text.contains('`'),
        Part::Text(Expr::Lit(_)) => true,
        _ => false,
    });
    let (open, close) = if plain { ("`", "`") } else { ("`` ", " ``") };
    let mut parts = vec![fixed(open)];
    parts.extend(inner.into_iter().map(|p| if let Part::Text(x) = p { Part::MdCode(x, false) } else { p }));
    parts.push(fixed(close));
    parts
}

fn as_json(expr: &Expr) -> Part {
    Part::Json(expr.clone())
}

impl<'a> Lowerer<'a> {
    fn new(defs: &Defs<'a>, props: Vec<String>) -> Self {
        Lowerer {
            components: defs.components.clone(),
            props,
            out: vec![vec![]],
            scopes: vec![],
            root_visible: true,
            slots: vec![],
            lists: vec![],
            tables: vec![],
            calling: vec![],
            elements: 0,
            loops: 0,
            vars: 0,
            md_muted: 0,
            has_h1: false,
            in_text: false,
            sections: 0,
            cells: 0,
            html_templates: 0,
            forwards: vec![],
            fields: vec![],
            diags: vec![],
        }
    }

    /// `<!doctype html>` up to `<body>`: `<html>` with the page's HTML attributes, the head, and `<body>` with
    /// the layout's and the page's `body(...)` attributes.
    fn document_start(
        &mut self,
        page: &'a Element,
        sources: &[&'a [Item]],
        settings: &Settings,
        defs: &Defs<'a>,
        stem: &str,
    ) {
        let mut html_open = vec![fixed("<!doctype html>\n<html lang=\""), escaped(&settings.lang), fixed("\"")];
        html_open.extend(self.attributes(page, &elements::page_settings()));
        html_open.push(fixed(">\n<head>\n"));
        self.html(html_open);

        // the layouts' heads, the page's, then those of the components they use, once each
        let mut used = vec![];
        for items in sources {
            components_used(items, &defs.components, &mut used);
        }
        let mut heads = elements_named(sources.iter().copied(), "head");
        for def in used {
            heads.extend(elements_named([&def.body[..]], "head"));
        }
        // a tag of the same name in a head replaces the one Hypra writes
        let written: Vec<String> = heads.iter().flat_map(|h| head_keys(&h.children)).collect();
        for (key, line) in settings.head_tags() {
            if !written.contains(&key) {
                self.html_line(1, line);
            }
        }
        self.emit(Op::AlternateLink { indent: "  ".into(), stem: stem.to_string() });
        self.md_muted += 1;
        for h in heads {
            self.items(&h.children, 1);
        }
        self.md_muted -= 1;
        self.html(vec![fixed("</head>\n<body")]);

        let bodies = elements_named(sources.iter().copied(), "body");
        for (i, b) in bodies.iter().enumerate() {
            // a later body(...) wins, so earlier ones leave out what it sets
            let later: Vec<&str> = bodies[i + 1..].iter().flat_map(|l| l.args.iter().filter_map(Arg::name)).collect();
            let attrs = self.attributes(b, &later);
            self.html(attrs);
        }
        self.html(vec![fixed(">\n")]);
    }

    fn front_matter(&mut self, settings: &Settings) {
        self.md(vec![fixed("---\ntitle: "), as_json(&settings.title), fixed("\n")]);
        if let Some(d) = &settings.description {
            self.md(vec![fixed("description: "), as_json(d), fixed("\n")]);
        }
        if let Some(c) = &settings.canonical {
            self.md(vec![fixed("canonical: "), as_json(c), fixed("\n")]);
        }
        self.md(vec![fixed("---\n\n")]);
        // a page without an h1 still starts with its title
        if !self.has_h1 {
            self.md(vec![fixed("# "), escaped(&settings.title), fixed("\n\n")]);
        }
    }

    fn emit(&mut self, op: Op) {
        // A constant value is known now, so it becomes static output.
        let op = match op {
            Op::Value { to: Channel::Html, expr: Expr::Lit(v), escape } => Op::Html(apply(escape, display(&v))),
            Op::Value { to: Channel::Md, expr: Expr::Lit(v), escape } => Op::Md(apply(escape, display(&v))),
            op => op,
        };
        let ops = self.out.last_mut().expect("an open frame");
        match (ops.last_mut(), op) {
            (Some(Op::Html(prev)), Op::Html(next)) | (Some(Op::Md(prev)), Op::Md(next)) => prev.push_str(&next),
            (_, op) => ops.push(op),
        }
    }

    fn write(&mut self, to: Channel, parts: Vec<Part>) {
        let html = to == Channel::Html;
        let text_escape = if html { Escape::Html } else { Escape::Markdown };
        let json_escape = if html { Escape::JsonHtml } else { Escape::Json };
        let url_escape = if html { Escape::Url } else { Escape::MarkdownUrl };
        for part in parts {
            let op = match part {
                Part::Fixed(text) if to == Channel::Html => Op::Html(text),
                Part::Fixed(text) => Op::Md(text),
                Part::Text(expr) => Op::Value { to, expr, escape: text_escape },
                Part::Json(expr) => Op::Value { to, expr, escape: json_escape },
                Part::Raw(expr) => Op::Value { to, expr, escape: Escape::Raw },
                Part::Url(expr) => Op::Value { to, expr, escape: url_escape },
                Part::Code(expr) => Op::Value { to, expr, escape: Escape::Script },
                Part::ScriptJson(expr) => Op::Value { to, expr, escape: Escape::ScriptJson },
                Part::Ops(ops) => {
                    for op in ops {
                        self.emit(op);
                    }
                    continue;
                }
                Part::MdCode(expr, fence) => {
                    Op::Value { to, expr, escape: if fence { Escape::MarkdownFence } else { Escape::MarkdownCode } }
                }
                Part::When(cond, text) => {
                    let text = if to == Channel::Html { Op::Html(text) } else { Op::Md(text) };
                    Op::If { cond, then: vec![text], otherwise: vec![] }
                }
            };
            self.emit(op);
        }
    }

    fn html(&mut self, parts: Vec<Part>) {
        self.write(Channel::Html, parts);
    }

    fn html_line(&mut self, depth: usize, mut parts: Vec<Part>) {
        parts.insert(0, Part::Fixed("  ".repeat(depth)));
        parts.push(fixed("\n"));
        self.html(parts);
    }

    fn md(&mut self, mut parts: Vec<Part>) {
        if self.md_muted > 0 {
            return;
        }
        // a Markdown table row is one line, so blocks inside a cell are separated by spaces
        if self.cells > 0 {
            for part in &mut parts {
                if let Part::Fixed(text) = part {
                    *text = text.replace('\n', " ");
                }
            }
        }
        self.write(Channel::Md, parts);
    }

    fn md_block(&mut self, mut parts: Vec<Part>) {
        parts.push(fixed("\n\n"));
        self.md(parts);
    }

    fn new_var(&mut self) -> usize {
        self.vars += 1;
        self.vars - 1
    }

    /// Lowers `items` into a fresh op list and returns it.
    fn block(&mut self, items: &'a [Item], depth: usize) -> Vec<Op> {
        self.out.push(vec![]);
        self.items(items, depth);
        self.out.pop().expect("pushed above")
    }

    fn items(&mut self, items: &'a [Item], depth: usize) {
        for item in items {
            match item {
                Item::Element(el) => self.element(el, depth),
                Item::Text(v) => {
                    let text = self.expr(v);
                    self.html_line(depth, vec![escaped(&text)]);
                    self.md_block(vec![escaped(&text)]);
                }
                Item::For(_) | Item::If(_) => {
                    if let Some(op) = self.control(item, &|l, body| l.block(body, depth)) {
                        self.emit(op);
                    }
                }
                // definitions and imports render where they're used
                Item::Use(_) | Item::Component(_) | Item::Layout(_) => {}
            }
        }
    }

    fn element(&mut self, el: &'a Element, depth: usize) {
        self.elements += 1;
        if self.elements > MAX_ELEMENTS {
            if self.elements == MAX_ELEMENTS + 1 {
                let message = format!("the page expands to more than {MAX_ELEMENTS} elements");
                self.diags.push(
                    Diagnostic::error("COMP-009", message, el.pos, el.name.len())
                        .help("components that use other components several times multiply; simplify the nesting"),
                );
            }
            return;
        }
        match el.name.as_str() {
            name if is_name(name) => self.call(el, depth),
            "slot" if self.html_templates == 0 => self.slot(el, depth),
            // compile-time only, or written into <head> by `lower`
            "page" | "props" | "allow" | "head" | "body" => {}
            "raw" => {
                if let Some(v) = &el.value {
                    let html = self.expr(v);
                    self.html_line(depth, vec![Part::Raw(html)]);
                }
            }
            "img" => {
                let tag = self.img(el);
                self.html_line(depth, tag);
                if !el.flag("decorative") {
                    let md = self.img_md(el);
                    self.md_block(md);
                }
            }
            "video" => self.video(el, depth),
            "action" => self.action(el, depth),
            "form" => self.form(el, depth),
            "field" => {
                if let Some(param) = self.field(el, depth)
                    && let Some(fields) = self.fields.last_mut()
                {
                    fields.push(param);
                }
            }
            _ => self.html_element(el, depth),
        }
    }

    /// Any HTML element or custom element.
    /// The opening tag, with the attributes a component call forwards when this is the component's root.
    fn open_tag(&mut self, el: &'a Element) -> Vec<Part> {
        let mut open = vec![fixed(&format!("<{}", el.name))];
        let forward = match self.forwards.last() {
            Some(f) if std::ptr::eq(f.root, el) => self.forwards.pop(),
            _ => None,
        };
        let merge_class = forward.as_ref().and_then(|f| f.class.clone());
        let attrs = self.attributes(el, if merge_class.is_some() { &["class"] } else { &[] });
        open.extend(attrs);
        if let Some(class) = merge_class {
            open.push(fixed(" class=\""));
            if let Some(own) = el.arg("class") {
                let own = self.expr(own);
                open.extend([escaped(&own), fixed(" ")]);
            }
            open.extend([escaped(&class), fixed("\"")]);
        }
        open.extend(forward.into_iter().flat_map(|f| f.attrs));
        open.push(fixed(">"));
        open
    }

    /// An element with children on lines of their own, and in Markdown a list item, table cell or row.
    fn block_element(&mut self, el: &'a Element, open: Vec<Part>, depth: usize) {
        let tag = el.name.as_str();
        self.html_line(depth, open);
        if tag == "li" {
            let bullet = if self.lists.last() == Some(&true) { "1. " } else { "- " };
            self.md(vec![fixed(bullet)]);
        }
        let cell = matches!(tag, "td" | "th");
        if cell {
            self.md(vec![fixed("| ")]);
            self.cells += 1;
        }
        if let Some(v) = &el.value {
            let text = self.expr(v);
            self.html_line(depth + 1, vec![escaped(&text)]);
            self.md_block(vec![escaped(&text)]);
        }
        let list = matches!(tag, "ul" | "ol");
        if list {
            self.lists.push(tag == "ol");
        }
        if tag == "table" {
            self.tables.push((false, self.loops));
        }
        self.items(&el.children, depth + 1);
        if list {
            self.lists.pop();
            self.md(vec![fixed("\n")]);
        }
        if tag == "table" {
            self.tables.pop();
            self.md(vec![fixed("\n")]);
        }
        if cell {
            self.md(vec![fixed(" ")]);
            self.cells -= 1;
        }
        if tag == "tr" {
            self.md(vec![fixed("|\n")]);
            self.table_header(el);
        }
        self.html_line(depth, vec![fixed(&format!("</{tag}>"))]);
    }

    fn html_element(&mut self, el: &'a Element, depth: usize) {
        let tag = el.name.as_str();
        let mut open = self.open_tag(el);
        let close = format!("</{tag}>");
        // scripts and styles are code, not text: no escaping, no Markdown
        let code = matches!(tag, "script" | "style");
        // the site's header and footer are chrome, but an article's header holds its title and byline
        let site_chrome = matches!(tag, "header" | "footer") && self.sections == 0;
        let chrome = site_chrome
            || matches!(tag, "nav" | "script" | "style" | "template" | "noscript" | "svg" | "math")
            || matches!(tag, "button" | "label" | "select" | "option" | "textarea" | "input" | "dialog");
        let section = matches!(tag, "article" | "section" | "aside");
        self.md_muted += usize::from(chrome);
        self.sections += usize::from(section);
        self.html_templates += usize::from(tag == "template");

        if html::is_void(tag) {
            self.html_line(depth, open);
            if tag == "hr" {
                self.md_block(vec![fixed("---")]);
            }
        } else if el.value.is_none() && el.children.is_empty() {
            open.push(fixed(&close));
            self.html_line(depth, open);
        } else if code {
            // code is written as it is: braces in JavaScript and CSS are never data, except in JSON scripts
            match (&el.value, el.value.as_ref().and_then(Value::as_str)) {
                (Some(v), Some(text)) if elements::is_json_script(el) => open.extend(self.json_script(text, v.pos)),
                (_, Some(text)) => open.push(Part::Code(Expr::Lit(json!(text)))),
                (Some(v), None) => open.push(Part::Code(self.expr(v))),
                (None, None) => {}
            }
            open.push(fixed(&close));
            self.html_line(depth, open);
        } else if is_inline_content(el) {
            let html = self.inline_html(el);
            open.extend(html);
            open.push(fixed(&close));
            self.html_line(depth, open);
            let mut md = if tag == "pre" { self.plain_text(el) } else { self.inline_md(el) };
            if tag == "pre" {
                md.insert(0, Part::Fixed(code_language(el)));
            }
            if tag == "a" {
                let href = self.expr_or_empty(el.main_or("href"));
                md.insert(0, fixed("["));
                md.extend([fixed("]("), Part::Url(href), fixed(")")]);
            }
            self.md_text_block(tag, md);
        } else {
            self.block_element(el, open, depth);
        }
        self.md_muted -= usize::from(chrome);
        self.sections -= usize::from(section);
        self.html_templates -= usize::from(tag == "template");
    }

    /// Markdown tables need `| --- |` after the first row. Only a row written once (not in a loop) can carry it.
    fn table_header(&mut self, row: &Element) {
        let loops = self.loops;
        let Some((written, table_loops)) = self.tables.last_mut() else { return };
        if *written {
            return;
        }
        *written = true;
        let cells = row
            .children
            .iter()
            .filter(|c| matches!(c, Item::Element(e) if matches!(e.name.as_str(), "th" | "td")))
            .count();
        if loops == *table_loops && cells > 0 {
            self.md(vec![Part::Fixed(format!("|{}\n", " --- |".repeat(cells)))]);
        }
    }

    fn inline_html(&mut self, el: &'a Element) -> Vec<Part> {
        let mut parts: Vec<Part> = el.value.iter().map(|v| Part::Text(self.expr(v))).collect();
        parts.extend(self.inline_html_items(&el.children));
        parts
    }

    fn inline_html_items(&mut self, items: &'a [Item]) -> Vec<Part> {
        let mut parts = vec![];
        for child in items {
            match child {
                Item::Text(v) => parts.push(Part::Text(self.expr(v))),
                Item::Element(c) if c.name == "img" => parts.extend(self.img(c)),
                Item::Element(c) => {
                    parts.push(fixed(&format!("<{}", c.name)));
                    parts.extend(self.attributes(c, &[]));
                    parts.push(fixed(">"));
                    if !html::is_void(&c.name) {
                        parts.extend(self.inline_html(c));
                        parts.push(fixed(&format!("</{}>", c.name)));
                    }
                }
                control => parts.extend(self.inline_control(control, Channel::Html, Self::inline_html_items)),
            }
        }
        parts
    }

    /// An `if` or `for` inside a line of text, lowered by `items` for one output.
    fn inline_control(
        &mut self,
        item: &'a Item,
        to: Channel,
        items: fn(&mut Self, &'a [Item]) -> Vec<Part>,
    ) -> Option<Part> {
        let lower = |l: &mut Self, body: &'a [Item]| {
            l.out.push(vec![]);
            let parts = items(l, body);
            l.write(to, parts);
            l.out.pop().expect("pushed above")
        };
        self.control(item, &lower).map(|op| Part::Ops(vec![op]))
    }

    /// An `if` or `for`, with its bodies lowered by `body`; `None` for any other item.
    fn control(&mut self, item: &'a Item, body: &dyn Fn(&mut Self, &'a [Item]) -> Vec<Op>) -> Option<Op> {
        match item {
            Item::If(c) => {
                let cond = self.condition(c);
                let then = body(self, &c.then);
                let otherwise = body(self, &c.otherwise);
                let (then, otherwise) = if c.negate { (otherwise, then) } else { (then, otherwise) };
                Some(Op::If { cond, then, otherwise })
            }
            Item::For(f) => {
                let list = self.expr(&f.list);
                let var = self.new_var();
                self.scopes.push((f.var.clone(), var));
                self.loops += 1;
                let lowered = body(self, &f.body);
                self.loops -= 1;
                self.scopes.pop();
                let source = f.list.as_path().unwrap_or_default().to_string();
                Some(Op::For { var, list, source, pos: f.list.pos, len: f.list.len, body: lowered })
            }
            _ => None,
        }
    }

    /// A JSON script like `"{\"name\": {product.name}}"`: each `{name}` becomes the value as a JSON string. Only
    /// a valid name in braces is data, so JSON's own braces, `{{` and `}}` included, stay as they are.
    fn json_script(&mut self, text: &str, pos: Pos) -> Vec<Part> {
        pieces(text, Braces::AsWritten)
            .into_iter()
            .map(|piece| match piece {
                Piece::Text(literal) => Part::Code(Expr::Lit(json!(literal))),
                Piece::Data { name, .. } => Part::ScriptJson(self.path(name, pos, 1)),
            })
            .collect()
    }

    /// The text of an element and everything inside it, without markup: the content of a code block.
    fn plain_text(&mut self, el: &'a Element) -> Vec<Part> {
        let mut parts: Vec<Part> = el.value.iter().map(|v| Part::MdCode(self.expr(v), true)).collect();
        parts.extend(self.plain_text_items(&el.children));
        parts
    }

    fn plain_text_items(&mut self, items: &'a [Item]) -> Vec<Part> {
        let mut parts = vec![];
        for child in items {
            match child {
                Item::Text(v) => parts.push(Part::MdCode(self.expr(v), true)),
                Item::Element(c) => parts.extend(self.plain_text(c)),
                control => parts.extend(self.inline_control(control, Channel::Md, Self::plain_text_items)),
            }
        }
        parts
    }

    fn inline_md(&mut self, el: &'a Element) -> Vec<Part> {
        let mut parts: Vec<Part> = el.value.iter().map(|v| Part::Text(self.expr(v))).collect();
        parts.extend(self.inline_md_items(&el.children));
        parts
    }

    fn inline_md_items(&mut self, items: &'a [Item]) -> Vec<Part> {
        let mut parts = vec![];
        for child in items {
            match child {
                Item::Text(v) => parts.push(Part::Text(self.expr(v))),
                Item::Element(c) => {
                    let inner = self.inline_md(c);
                    let (before, after) = match c.name.as_str() {
                        "b" | "strong" => ("**", "**".to_string()),
                        "i" | "em" => ("*", "*".to_string()),
                        "code" | "kbd" | "samp" => {
                            parts.extend(code_span(inner));
                            continue;
                        }
                        "a" => {
                            let href = self.expr_or_empty(c.main_or("href"));
                            parts.push(fixed("["));
                            parts.extend(inner);
                            parts.extend([fixed("]("), Part::Url(href), fixed(")")]);
                            continue;
                        }
                        "img" if !c.flag("decorative") => {
                            parts.extend(self.img_md(c));
                            continue;
                        }
                        "br" => (" ", String::new()),
                        "button" | "input" | "select" | "textarea" | "img" => continue,
                        _ => ("", String::new()),
                    };
                    parts.push(fixed(before));
                    parts.extend(inner);
                    parts.push(Part::Fixed(after));
                }
                control => parts.extend(self.inline_control(control, Channel::Md, Self::inline_md_items)),
            }
        }
        parts
    }

    /// Markdown for an element whose content is one line of text.
    fn md_text_block(&mut self, tag: &str, mut text: Vec<Part>) {
        let (prefix, suffix) = match tag {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level: usize = tag[1..].parse().unwrap_or(2);
                if level == 1 && self.md_muted == 0 {
                    self.has_h1 = true;
                }
                (format!("{} ", "#".repeat(level)), "\n\n")
            }
            "li" => (if self.lists.last() == Some(&true) { "1. " } else { "- " }.to_string(), "\n"),
            "blockquote" => ("> ".to_string(), "\n\n"),
            "pre" => {
                text =
                    text.into_iter().map(|p| if let Part::Text(x) = p { Part::MdCode(x, true) } else { p }).collect();
                // the language, if any, comes first and belongs on the fence line
                ("```".to_string(), "\n```\n\n")
            }
            "summary" => ("**".to_string(), "**\n\n"),
            "td" | "th" => ("| ".to_string(), " "),
            _ => (String::new(), "\n\n"),
        };
        text.insert(0, Part::Fixed(prefix));
        text.push(fixed(suffix));
        self.md(text);
    }

    /// The attributes of an HTML element, in the order written. `skip` lists Hypra-only attributes.
    fn attributes(&mut self, el: &Element, skip: &[&str]) -> Vec<Part> {
        let mut parts = vec![];
        for (i, a) in el.args.iter().enumerate() {
            let name = match &a.key {
                Some(key) => key.name.as_str(),
                None if i == 0 => match html::main_attribute(&el.name) {
                    Some(main) => main,
                    None => continue,
                },
                None => continue,
            };
            if skip.contains(&name) {
                continue;
            }
            parts.extend(self.attribute(el, name, &a.value));
        }
        parts
    }

    /// `hidden: true` is written `hidden`; `aria-hidden: true` is written `aria-hidden="true"`.
    fn attribute(&mut self, el: &Element, name: &str, value: &Value) -> Vec<Part> {
        let presence = html::is_presence_attribute(&el.name, name);
        // a written true or false says it plainly; data is only known when rendering, so its kind isn't
        let bare = !html::takes_true_false_text(name);
        match &value.kind {
            ValueKind::Bool(true) if bare => vec![fixed(&format!(" {name}"))],
            ValueKind::Bool(false) if bare => vec![],
            ValueKind::Path(_) if presence => {
                let cond = self.expr(value);
                vec![Part::When(cond, format!(" {name}"))]
            }
            _ => {
                let v = self.expr(value);
                let v = if elements::is_url(el, name) { Part::Url(v) } else { Part::Text(v) };
                vec![fixed(&format!(" {name}=\"")), v, fixed("\"")]
            }
        }
    }

    fn img(&mut self, el: &Element) -> Vec<Part> {
        let src = self.expr_or_empty(el.main_or("src"));
        let alt = if el.flag("decorative") { Expr::Lit(json!("")) } else { self.expr_or_empty(el.arg("alt")) };
        let mut tag = vec![fixed("<img src=\""), Part::Url(src), fixed("\" alt=\""), escaped(&alt), fixed("\"")];
        tag.extend(self.attributes(el, &["src", "alt", "decorative", "priority"]));
        if el.flag("priority") {
            tag.push(fixed(" fetchpriority=\"high\""));
        } else if el.arg("loading").is_none() {
            tag.push(fixed(" loading=\"lazy\""));
        }
        tag.push(fixed(">"));
        tag
    }

    fn img_md(&mut self, el: &Element) -> Vec<Part> {
        let src = self.expr_or_empty(el.main_or("src"));
        let alt = self.expr_or_empty(el.arg("alt"));
        vec![fixed("!["), escaped(&alt), fixed("]("), Part::Url(src), fixed(")")]
    }

    fn video(&mut self, el: &'a Element, depth: usize) {
        // controls are on unless the page turns them off, e.g. for a muted background video
        let controls = !matches!(el.arg("controls").map(|v| &v.kind), Some(ValueKind::Bool(false)));
        let mut open = vec![fixed(if controls { "<video controls" } else { "<video" })];
        let src = el.main_or("src").map(|v| self.expr(v));
        if let Some(src) = &src {
            open.extend([fixed(" src=\""), Part::Url(src.clone()), fixed("\"")]);
        }
        open.extend(self.attributes(el, &["src", "captions", "controls"]));
        open.push(fixed(">"));
        self.html_line(depth, open);
        if let Some(captions) = el.arg("captions") {
            let captions = self.expr(captions);
            self.html_line(
                depth + 1,
                vec![fixed("<track kind=\"captions\" src=\""), Part::Url(captions), fixed("\" default>")],
            );
        }
        // shown by browsers that can't play the video
        if let Some(v) = &el.value {
            let fallback = self.expr(v);
            self.html_line(depth + 1, vec![escaped(&fallback)]);
        }
        self.items(&el.children, depth + 1);
        self.html_line(depth, vec![fixed("</video>")]);
        let first_source = el.children.iter().find_map(|c| match c {
            Item::Element(source) if source.name == "source" => source.main_or("src"),
            _ => None,
        });
        if let Some(src) = src.or_else(|| first_source.map(|v| self.expr(v))) {
            self.md_block(vec![fixed("[Video]("), Part::Url(src), fixed(")")]);
        }
    }

    fn action(&mut self, el: &Element, depth: usize) {
        let name = el.main_or("name").and_then(Value::as_str).unwrap_or("action").to_string();
        let label = self.expr_or_empty(el.value.as_ref());
        let endpoint = format!("/_a/{name}");
        let mut open = vec![fixed(&format!(
            "<form method=\"post\" action=\"{}\" toolname=\"{}\" tooldescription=\"",
            esc(&endpoint),
            esc(&name)
        ))];
        open.push(escaped(&label));
        open.push(fixed("\""));
        if let Some(confirm) = el.arg("confirm") {
            let confirm = self.expr(confirm);
            open.extend([fixed(" onsubmit=\"return confirm("), Part::Json(confirm), fixed(")\"")]);
        }
        open.push(fixed(">"));
        self.html_line(depth, open);
        self.emit(Op::FormFields { indent: "  ".repeat(depth + 1), action: None });

        let mut params = vec![];
        let mut styling = vec![];
        for a in &el.args {
            // the unnamed first argument and `name` name the action; `confirm` is the question
            let Some(key) = &a.key else { continue };
            if matches!(key.name.as_str(), "name" | "confirm") {
                continue;
            }
            if is_button_attribute(&key.name) {
                styling.push(key.name.as_str());
                continue;
            }
            let value = self.expr(&a.value);
            let input = format!("<input type=\"hidden\" name=\"{}\" value=\"", esc(&key.name));
            self.html_line(depth + 1, vec![fixed(&input), escaped(&value), fixed("\">")]);
            params.push(ParamDef { name: key.name.clone(), schema: Json::Null, value: Some(value), required: true });
        }
        let sent: Vec<&str> = el.args.iter().filter_map(|a| a.name()).filter(|n| !styling.contains(n)).collect();
        let mut button = vec![fixed("<button type=\"submit\"")];
        button.extend(self.attributes(el, &sent));
        button.extend([fixed(">"), escaped(&label), fixed("</button>")]);
        self.html_line(depth + 1, button);
        self.html_line(depth, vec![fixed("</form>")]);
        self.md_block(vec![fixed("[Action: "), escaped(&label), fixed("]")]);
        self.emit(Op::Action(ActionDef {
            name,
            description: label,
            method: "POST".into(),
            endpoint: Expr::Lit(json!(endpoint)),
            params,
        }));
    }

    fn form(&mut self, el: &'a Element, depth: usize) {
        let method = el.arg("method").and_then(Value::as_str).unwrap_or("post").to_ascii_lowercase();
        let skip = ["action", "name", "description", "method"];
        if method == "dialog" {
            // closes the dialog it's in; nothing is sent, so it's plain HTML
            let mut open = vec![fixed("<form method=\"dialog\"")];
            open.extend(self.attributes(el, &skip));
            open.push(fixed(">"));
            self.html_line(depth, open);
            self.md_muted += 1;
            self.items(&el.children, depth + 1);
            self.md_muted -= 1;
            self.html_line(depth, vec![fixed("</form>")]);
            return;
        }
        let name = el.arg("name").and_then(Value::as_str).map(str::to_string);
        let description = match (el.arg("description"), &name) {
            (Some(v), _) => Some(self.expr(v)),
            (None, Some(name)) => Some(Expr::Lit(json!(name))),
            (None, None) => None,
        };
        let endpoint = self.expr_or_empty(el.main_or("action"));
        let html_method = if method == "get" { "get" } else { "post" };
        let mut open =
            vec![fixed(&format!("<form method=\"{html_method}\" action=\"")), Part::Url(endpoint.clone()), fixed("\"")];
        if has_file_field(&el.children) {
            open.push(fixed(" enctype=\"multipart/form-data\""));
        }
        // a named form is a tool agents can call
        if let (Some(name), Some(description)) = (&name, &description) {
            open.extend([
                fixed(&format!(" toolname=\"{}\" tooldescription=\"", esc(name))),
                escaped(description),
                fixed("\""),
            ]);
        }
        open.extend(self.attributes(el, &skip));
        open.push(fixed(">"));
        self.html_line(depth, open);
        // hidden fields like a CSRF token only go to this site's own URLs
        if html_method == "post" {
            let indent = "  ".repeat(depth + 1);
            match &endpoint {
                Expr::Lit(url) if !html::is_same_site(&display(url)) => {}
                Expr::Lit(_) => self.emit(Op::FormFields { indent, action: None }),
                dynamic => self.emit(Op::FormFields { indent, action: Some(dynamic.clone()) }),
            }
        }
        if !matches!(method.as_str(), "get" | "post") {
            let input = format!("<input type=\"hidden\" name=\"_method\" value=\"{}\">", esc(&method.to_uppercase()));
            self.html_line(depth + 1, vec![fixed(&input)]);
        }

        // the form is listed as an action; its controls aren't Markdown content
        self.md_muted += 1;
        self.fields.push(vec![]);
        self.items(&el.children, depth + 1);
        let params = self.fields.pop().expect("pushed above");
        self.md_muted -= 1;
        self.html_line(depth, vec![fixed("</form>")]);
        if let Some(description) = &description {
            self.md_block(vec![fixed("[Form: "), escaped(description), fixed("]")]);
        }
        if let (Some(name), Some(description)) = (name, description) {
            let method = method.to_uppercase();
            self.emit(Op::Action(ActionDef { name, description, method, endpoint, params }));
        }
    }

    fn field(&mut self, el: &Element, depth: usize) -> Option<ParamDef> {
        let key = el.main_or("name").and_then(Value::as_str)?.to_string();
        let label = self.expr_or_empty(el.arg("label"));
        let default = el.arg("default").map(|v| self.expr(v));
        let (ty, args): (&str, &[Arg]) = match el.arg("type").map(|v| &v.kind) {
            Some(ValueKind::Call(t, a)) => (t, a),
            Some(ValueKind::Name(t)) => (t, &[]),
            _ => ("String", &[]),
        };
        let kind = elements::field_type(ty).unwrap_or(&elements::FIELD_TYPES[0]);
        let required = !el.flag("optional") || kind.input == "checkbox";
        let req = if required { " required" } else { "" };
        let name = esc(&key);
        let extra = self.attributes(el, &["name", "type", "label", "optional", "default"]);

        let mut schema = json!({ "type": kind.schema.0 });
        if let Some(format) = kind.schema.1 {
            schema["format"] = json!(format);
        }
        let (lo, hi) = field_bounds(args);
        let (html_names, schema_names) = match kind.args {
            FieldArgs::Length => (("minlength", "maxlength"), Some(("minLength", "maxLength"))),
            FieldArgs::Number => (("min", "max"), Some(("minimum", "maximum"))),
            _ => (("min", "max"), None),
        };
        if let Some((min, max)) = schema_names {
            for (bound, key) in [(&lo, min), (&hi, max)] {
                if let Some(n) = bound.as_deref().and_then(schema_number) {
                    schema[key] = n;
                }
            }
        }
        let bounds = format!("{}{}", attr(html_names.0, &lo), attr(html_names.1, &hi));

        let mut control = match kind.input {
            "textarea" => {
                let mut c = vec![fixed(&format!("<textarea name=\"{name}\"{bounds}"))];
                c.extend(extra);
                c.push(fixed(&format!("{req}>")));
                c.extend(default.as_ref().map(escaped));
                c.push(fixed("</textarea>"));
                c
            }
            "select" => {
                let options = choice_options(args);
                schema["enum"] = json!(options.iter().map(|(value, _)| value).collect::<Vec<_>>());
                let mut c = vec![fixed(&format!("<select name=\"{name}\""))];
                c.extend(extra);
                c.push(fixed(&format!("{req}>")));
                // a required select starts on an empty choice named by the label, so the visitor has to choose
                if required {
                    c.extend([fixed("<option value=\"\">"), escaped(&label), fixed("</option>")]);
                }
                for (value, text) in &options {
                    c.push(fixed(&format!("<option value=\"{}\"", esc(value))));
                    match &default {
                        Some(Expr::Lit(d)) if display(d) == *value => c.push(fixed(" selected")),
                        Some(Expr::Lit(_)) | None => {}
                        Some(dynamic) => {
                            let chosen = Expr::Equals(Box::new(dynamic.clone()), value.clone());
                            c.push(Part::When(chosen, " selected".into()));
                        }
                    }
                    c.push(fixed(&format!(">{}</option>", esc(text))));
                }
                c.push(fixed("</select>"));
                c
            }
            "checkbox" => {
                schema["const"] = json!(true);
                let mut c = vec![fixed(&format!("<input type=\"checkbox\" name=\"{name}\" value=\"1\""))];
                c.extend(extra);
                c.push(fixed(" required>"));
                c
            }
            input => {
                let step = if kind.name == "Number" { " step=\"any\"" } else { "" };
                let mut c = vec![fixed(&format!("<input type=\"{input}\" name=\"{name}\"{bounds}{step}"))];
                c.extend(extra);
                if let Some(d) = &default {
                    c.extend([fixed(" value=\""), escaped(d), fixed("\"")]);
                }
                c.push(fixed(&format!("{req}>")));
                c
            }
        };
        let mut line = vec![fixed("<label>")];
        if kind.input == "checkbox" {
            line.append(&mut control);
            line.extend([fixed(" "), escaped(&label)]);
        } else {
            line.extend([escaped(&label), fixed(" ")]);
            line.append(&mut control);
        }
        line.push(fixed("</label>"));
        self.html_line(depth, line);
        Some(ParamDef { name: key, schema, value: None, required })
    }

    fn call(&mut self, el: &'a Element, depth: usize) {
        let Some(def) = self.components.get(&el.name).copied() else { return };
        if self.calling.contains(&el.name) {
            let message = format!("`{}` calls itself", el.name);
            self.diags.push(
                Diagnostic::error("COMP-007", message, el.pos, el.name.len())
                    .help("components are expanded into the page, so they can't be recursive; use a for loop"),
            );
            return;
        }
        self.calling.push(el.name.clone());
        let mut params = vec![];
        for p in &def.params {
            let Some(name) = param_name(p) else { continue };
            let value = match (el.arg(name), &p.default) {
                (Some(v), _) => self.expr(v),
                // a default belongs to the component, which never sees the caller's data
                (None, Some(v)) => {
                    let caller_scopes = std::mem::take(&mut self.scopes);
                    let caller_root = std::mem::replace(&mut self.root_visible, false);
                    let value = self.expr(v);
                    self.scopes = caller_scopes;
                    self.root_visible = caller_root;
                    value
                }
                (None, None) => Expr::Lit(Json::Null),
            };
            let var = self.new_var();
            self.emit(Op::Set { var, value });
            params.push((name.to_string(), var));
        }
        // HTML attributes of the call go to the component's root element, read in the caller's scope
        let forwards = self.forwards.len();
        if let Some(root) = forward_root(def) {
            let mut forward = Forward { root, class: None, attrs: vec![] };
            for a in &el.args {
                let Some(key) = a.key.as_ref().filter(|k| param_name_of(def, &k.name).is_none()) else { continue };
                if key.name == "class" {
                    forward.class = Some(self.expr(&a.value));
                } else {
                    let attr = self.attribute(root, &key.name, &a.value);
                    forward.attrs.extend(attr);
                }
            }
            self.forwards.push(forward);
        }
        let caller_scopes = std::mem::replace(&mut self.scopes, params);
        let caller_root = std::mem::replace(&mut self.root_visible, false);
        self.slots.push(SlotFrame { children: &el.children, scopes: caller_scopes, root_visible: caller_root });
        self.items(&def.body, depth);
        self.forwards.truncate(forwards);
        self.calling.pop();
        let frame = self.slots.pop().expect("pushed above");
        self.scopes = frame.scopes;
        self.root_visible = frame.root_visible;
    }

    /// Renders what the call gave a slot, in the caller's scope; a named slot the call left empty shows its own
    /// content instead.
    fn slot(&mut self, el: &'a Element, depth: usize) {
        let Some(frame) = self.slots.pop() else { return };
        match given_content(frame.children, el) {
            Some(items) => {
                let inner_scopes = std::mem::replace(&mut self.scopes, frame.scopes.clone());
                let inner_root = std::mem::replace(&mut self.root_visible, frame.root_visible);
                for item in items {
                    self.items(std::slice::from_ref(item), depth);
                }
                self.scopes = inner_scopes;
                self.root_visible = inner_root;
            }
            None => self.items(&el.children, depth),
        }
        self.slots.push(frame);
    }

    /// A value, or empty text when it's left out.
    fn expr_or_empty(&mut self, value: Option<&Value>) -> Expr {
        value.map_or(Expr::Lit(json!("")), |v| self.expr(v))
    }

    fn expr(&mut self, v: &Value) -> Expr {
        match &v.kind {
            ValueKind::Str(text) => self.interpolate(text, v),
            ValueKind::Num(n) => Expr::Lit(num(n)),
            ValueKind::Bool(b) => Expr::Lit(json!(b)),
            ValueKind::Path(p) => self.path(p, v.pos, v.len),
            ValueKind::Name(n) => Expr::Lit(json!(n)),
            // checked to hold only fixed values, so the whole list is known now
            ValueKind::List(items) => Expr::Lit(Json::Array(
                items
                    .iter()
                    .map(|item| match self.expr(item) {
                        Expr::Lit(value) => value,
                        _ => Json::Null,
                    })
                    .collect(),
            )),
            ValueKind::Call(..) | ValueKind::Range(..) => Expr::Lit(Json::Null),
        }
    }

    /// `"Hi {user.name}"` reads data. Braces around anything that isn't a name, like the
    /// JavaScript in `"{ open: false }"`, are text. `{{` and `}}` are literal braces.
    fn interpolate(&mut self, text: &str, value: &Value) -> Expr {
        let mut parts = vec![];
        for piece in pieces(text, Braces::Doubled) {
            match piece {
                Piece::Text(literal) => parts.push(Expr::Lit(json!(literal))),
                Piece::Data { name, at } => {
                    // where the piece is written, or the whole string where that isn't known
                    let at = piece_pos(value, at).unwrap_or(value.pos);
                    self.in_text = true;
                    parts.push(self.path(name, at, name.chars().count() + 2));
                    self.in_text = false;
                }
            }
        }
        Expr::concat(parts)
    }

    /// What an `if` tests: the value, or whether it equals a fixed one. `negate` swaps the branches instead.
    fn condition(&mut self, c: &If) -> Expr {
        let value = self.expr(&c.cond);
        match c.equals.as_ref().map(|v| &v.kind) {
            Some(ValueKind::Str(text) | ValueKind::Num(text)) => Expr::Equals(Box::new(value), text.clone()),
            Some(ValueKind::Bool(b)) => Expr::Equals(Box::new(value), b.to_string()),
            _ => value,
        }
    }

    /// Resolves a dotted name to a variable slot or page data, at compile time.
    fn path(&mut self, path: &str, pos: Pos, len: usize) -> Expr {
        match path {
            "true" => return Expr::Lit(json!(true)),
            "false" => return Expr::Lit(json!(false)),
            _ => {}
        }
        let mut segments = path.split('.').map(String::from);
        let first = segments.next().unwrap_or_default();
        let rest: Vec<String> = segments.collect();

        if let Some((_, id)) = self.scopes.iter().rev().find(|(name, _)| *name == first) {
            return Expr::Var { id: *id, name: first, path: rest, pos, len };
        }
        if self.root_visible && self.props.contains(&first) {
            let mut full = vec![first];
            full.extend(rest);
            return Expr::Data { path: full, pos, len };
        }

        let mut names: Vec<&str> = self.scopes.iter().map(|(n, _)| n.as_str()).collect();
        if self.root_visible {
            names.extend(self.props.iter().map(String::as_str));
        }
        let mut d = Diagnostic::error("TYPE-002", format!("unknown name `{first}`"), pos, len);
        d = match suggest(&first, names) {
            Some(close) => d.help(format!("did you mean `{close}`?")),
            None if is_name(&first) => {
                d.help("names (capitalized) are layouts, components and types; data is lowercase")
            }
            None if !self.root_visible => d.help("components only see their parameters; pass it as an argument"),
            None if self.in_text => d.help(format!(
                "declare it with props({first}: ...), or double the braces to write them as text: {{{{{first}}}}}"
            )),
            None => d.help(format!("declare it with props({first}: ...), or quote it if it's text: \"{first}\"")),
        };
        self.diags.push(d);
        Expr::Lit(Json::Null)
    }
}

/// The fence line's language for `pre { code(class: "language-rust") "..." }`, then the line break.
fn code_language(pre: &Element) -> String {
    let classes = pre.children.iter().find_map(|c| match c {
        Item::Element(code) if code.name == "code" => code.arg("class").and_then(Value::as_str),
        _ => None,
    });
    let language = classes.and_then(|c| c.split_whitespace().find_map(|class| class.strip_prefix("language-")));
    let language = language.filter(|l| l.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '+' | '#')));
    format!("{}\n", language.unwrap_or_default())
}

/// Text with inline elements, written on one line: `p { "Weighs " b "220 g" }`. It fits on one line when it's
/// text and inline elements like `b` and `a`, with at least one of them.
fn is_inline_content(el: &Element) -> bool {
    (el.value.is_some() || !el.children.is_empty()) && el.children.iter().all(is_inline_item)
}

/// Inline content: text and inline HTML elements, all the way down.
fn is_inline_item(item: &Item) -> bool {
    match item {
        Item::Text(_) => true,
        Item::Element(el) => html::is_inline(&el.name) && el.children.iter().all(is_inline_item),
        Item::If(c) => c.then.iter().chain(&c.otherwise).all(is_inline_item),
        Item::For(f) => f.body.iter().all(is_inline_item),
        _ => false,
    }
}

/// Attributes of an action that style its button or hook up a script, instead of being sent with it.
fn is_button_attribute(name: &str) -> bool {
    matches!(name, "class" | "style" | "title" | "disabled" | "form")
        || ["aria-", "data-", "hx-", "x-", "@", ":"].iter().any(|prefix| name.starts_with(prefix))
}

/// Files can only be sent as multipart form data.
fn has_file_field(items: &[Item]) -> bool {
    any_element(items, &|e| {
        e.name == "field"
            && matches!(e.arg("type").map(|v| &v.kind), Some(ValueKind::Name(t) | ValueKind::Call(t, _)) if t == "File")
    })
}

/// A field type's lower and upper bound as written: `1..5`, or `min: 1, max: 5`.
fn field_bounds(args: &[Arg]) -> (Option<String>, Option<String>) {
    let range = args.iter().find_map(|a| match &a.value.kind {
        ValueKind::Range(lo, hi) if a.key.is_none() => Some((lo.clone(), hi.clone())),
        _ => None,
    });
    let named = |key: &str| {
        args.iter().find(|a| a.name() == Some(key)).and_then(|a| match &a.value.kind {
            ValueKind::Num(x) | ValueKind::Str(x) => Some(x.clone()),
            _ => None,
        })
    };
    range.unwrap_or_else(|| (named("min"), named("max")))
}

/// A bound in a JSON Schema, which takes numbers.
fn schema_number(text: &str) -> Option<Json> {
    text.parse::<i64>().map(Json::from).ok().or_else(|| text.parse::<f64>().ok().map(Json::from))
}

/// A Choice's options as (value, text): `"s"` is both, `de: "Germany"` is a value with its own text.
fn choice_options(args: &[Arg]) -> Vec<(String, String)> {
    args.iter()
        .filter_map(|a| {
            let text = match &a.value.kind {
                ValueKind::Str(t) | ValueKind::Num(t) => t.clone(),
                _ => return None,
            };
            Some(match &a.key {
                Some(key) => (key.name.clone(), text),
                None => (text.clone(), text),
            })
        })
        .collect()
}

fn attr(name: &str, value: &Option<String>) -> String {
    value.as_ref().map(|v| format!(" {name}=\"{}\"", esc(v))).unwrap_or_default()
}

/// A number as written. When the value would print differently (`13.40`, `01067`), it stays text,
/// so the output shows exactly what the source says.
fn num(s: &str) -> Json {
    let number = s.parse::<i64>().map(Json::from).or_else(|_| s.parse::<f64>().map(Json::from));
    match number {
        Ok(n) => {
            let printed = n.to_string();
            if printed == s { n } else { Json::String(s.to_string()) }
        }
        Err(_) => Json::String(s.to_string()),
    }
}
