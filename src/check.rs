//! Structural checks before lowering: every element and attribute against the element table,
//! plus accessibility, forms, SEO basics, components and layouts.

use crate::defs::{
    Defs, chain_setting, contains_slot, forward_root, given_content, is_html_attribute, layout_chain, named_content,
    param_name, slot_name,
};
use crate::diag::{Diagnostic, RULES, suggest};
use crate::elements::{self, Attr, FIELD_TYPES, FieldArgs, Kind, LAYOUT_SETTINGS, Spec, TextRule};
use crate::html;
use crate::ir::runs_code;
use crate::lexer::Pos;
use crate::parser::{
    Arg, Braces, Def, Element, Item, Piece, Value, ValueKind, any_element, elements_named, is_name, pieces, reads_data,
};
use crate::rules;
use crate::words::{self, Words};

/// The language a page is written in: its `lang`, its layout's, or English, Hypra's default. `None` when it comes
/// from data, or for a file without a page, so checks that depend on the language don't guess.
pub fn page_language(items: &[Item], defs: &Defs) -> Option<String> {
    let page = *elements_named([items], "page").first()?;
    let chain = layout_chain(defs, page.arg("layout"));
    match page.arg("lang").or_else(|| chain_setting(&chain, "lang")) {
        None => Some("en".to_string()),
        Some(v) => static_text(v).map(String::from),
    }
}

/// Checks one file. Only the entry file (`is_entry`) has a page; imported files hold definitions. `lang` is the
/// page's language, for the checks that depend on it.
pub fn check<'a>(items: &'a [Item], defs: &'a Defs<'a>, is_entry: bool, lang: Option<&str>) -> Vec<Diagnostic> {
    let mut c = Checker {
        defs,
        words: lang.and_then(words::for_language),
        loops: 0,
        links: vec![],
        diags: vec![],
        allowed: vec![],
        in_template: false,
        names: vec![],
        in_foreign: false,
        in_html_template: false,
        in_control: false,
        form: InForm::No,
    };
    let pages = elements_named([items], "page");
    match (pages.first(), is_entry) {
        (None, true) if !is_library(items) => c.diags.push(
            Diagnostic::error("SEM-002", "a .hypra file needs a `page` element", Default::default(), 1)
                .help("page(title: \"...\", description: \"...\") { ... }"),
        ),
        (Some(p), false) => c.diags.push(
            Diagnostic::error("SEM-007", "imported files can't contain a page", p.pos, 4)
                .help("imported files hold components and layouts"),
        ),
        _ => {}
    }
    for extra in pages.iter().skip(1) {
        c.diags.push(
            Diagnostic::error("SEM-003", "a file has one page", extra.pos, 4)
                .help("put each page in its own file; share parts with components and layouts"),
        );
    }
    c.items(items, "");
    c.diags
}

struct Checker<'a> {
    defs: &'a Defs<'a>,
    /// Words that say nothing in the page's language, if the checks know that language.
    words: Option<&'static Words>,
    /// How many `for` loops the current item is in.
    loops: usize,
    /// Links with fixed text and a fixed target seen so far, for links with the same text going elsewhere.
    links: Vec<(String, String)>,
    diags: Vec<Diagnostic>,
    /// Rule codes switched off by `allow` for the item currently being checked.
    allowed: Vec<String>,
    /// Inside a component or layout, where `slot` is allowed and headings don't count toward the page.
    in_template: bool,
    /// Inside `svg` or `math`.
    in_foreign: bool,
    /// Inside an HTML `template`, where `slot` is the web components element, not Hypra's.
    in_html_template: bool,
    /// Data names in scope: the page's props, a component's parameters, loop variables.
    names: Vec<String>,
    /// Directly inside a `for` or `if`, with no element in between.
    in_control: bool,
    form: InForm,
}

/// Where a `field` can go: anywhere inside a form, but not in a `for` or `if` there, because fields are the
/// form's fixed schema.
#[derive(Clone, Copy, PartialEq)]
enum InForm {
    No,
    Yes,
    InControl,
}

impl<'a> Checker<'a> {
    fn report(&mut self, d: Diagnostic) {
        if !self.allowed.contains(&d.code) {
            self.diags.push(d);
        }
    }

    /// Reports a problem found after the walk, under the rules that were allowed where it is.
    fn report_under(&mut self, d: Diagnostic, allowed: &[String]) {
        if !allowed.contains(&d.code) {
            self.diags.push(d);
        }
    }

    /// Inside a form, a `for` or `if` closes it to fields. Returns the state to restore.
    fn enter_control(&mut self) -> InForm {
        let was = self.form;
        if was == InForm::Yes {
            self.form = InForm::InControl;
        }
        was
    }

    /// `parent` is the enclosing element's name, or "" at the top of a file.
    fn items(&mut self, items: &'a [Item], parent: &str) {
        let mut pending: Vec<String> = vec![];
        for item in items {
            if let Item::Element(e) = item
                && e.name == "allow"
            {
                self.element(e, parent);
                if let Some(rule) = item.allowed_rule() {
                    self.known_rule(rule);
                    pending.extend(rule.as_str().map(String::from));
                }
                continue;
            }
            let before = self.allowed.len();
            self.allowed.append(&mut pending);
            self.item(item, parent);
            self.allowed.truncate(before);
        }
    }

    /// `allow("SEO-03")` with a typo would silently allow nothing.
    fn known_rule(&mut self, rule: &Value) {
        let Some(code) = rule.as_str() else { return };
        if RULES.iter().any(|(known, _)| *known == code) {
            return;
        }
        let d = Diagnostic::warning("ALLOW-003", format!("there's no rule `{code}`"), rule.pos, rule.len);
        let d = match suggest(code, RULES.iter().map(|(known, _)| *known)) {
            Some(close) => d.help(format!("did you mean `{close}`?")),
            None => d.help("rule codes look like SEO-003; the compiler prints them with every message"),
        };
        self.diags.push(d);
    }

    fn item(&mut self, item: &'a Item, parent: &str) {
        let top = parent.is_empty();
        let misplaced = |what: &str, pos, len| {
            Diagnostic::error("SEM-003", format!("{what} must be at the top of the file"), pos, len)
        };
        match item {
            // allows at the start of a page cover all of it, including what's checked on the page itself
            Item::Element(e) if e.name == "page" => {
                let before = self.allowed.len();
                self.allowed.extend(Item::page_allows(&e.children));
                self.element(e, parent);
                self.allowed.truncate(before);
            }
            Item::Element(e) => self.element(e, parent),
            Item::Text(v) if top => {
                self.report(Diagnostic::error("SEM-003", "text must be inside the page", v.pos, v.len));
            }
            Item::Text(v) if parent == "head" => {
                self.report(Diagnostic::error("SEM-011", "text doesn't belong in head", v.pos, v.len));
            }
            Item::Text(_) => {}
            Item::Use(v) if !top => self.report(misplaced("`use`", v.pos, v.len)),
            Item::Use(_) => {}
            Item::Component(d) if !top => self.report(misplaced("a component", d.pos, 9)),
            Item::Component(d) => self.component_def(d),
            Item::Layout(d) if !top => self.report(misplaced("a layout", d.pos, 6)),
            Item::Layout(d) => self.layout_def(d),
            Item::For(f) if top => {
                self.report(Diagnostic::error("SEM-003", "`for` must be inside the page", f.pos, 3));
            }
            Item::For(f) => {
                if matches!(f.list.kind, ValueKind::List(_)) && !f.list.is_fixed() {
                    self.report(
                        Diagnostic::error("ATTR-003", "a list holds fixed values", f.list.pos, f.list.len)
                            .help("write the list in the controller when it holds data"),
                    );
                }
                let was = std::mem::replace(&mut self.in_control, true);
                let was_form = self.enter_control();
                self.names.push(f.var.clone());
                self.loops += 1;
                self.items(&f.body, parent);
                self.loops -= 1;
                self.names.pop();
                self.in_control = was;
                self.form = was_form;
            }
            Item::If(c) if top => {
                self.report(Diagnostic::error("SEM-003", "`if` must be inside the page", c.pos, 2));
            }
            Item::If(c) => {
                let was = std::mem::replace(&mut self.in_control, true);
                let was_form = self.enter_control();
                self.items(&c.then, parent);
                self.items(&c.otherwise, parent);
                self.in_control = was;
                self.form = was_form;
            }
        }
    }

    fn element(&mut self, e: &'a Element, parent: &str) {
        let len = e.name.len();
        if is_name(&e.name) {
            self.call(e);
            self.items(&e.children, &e.name);
            return;
        }
        // inside svg and math, elements and attributes follow SVG and MathML; only code in them is checked
        if self.in_foreign {
            self.common(e);
            return self.items(&e.children, &e.name);
        }
        // inside an HTML template, slot is the web components element
        let spec = if e.name == "slot" && self.in_html_template {
            Some(elements::html_spec("slot"))
        } else {
            elements::spec(&e.name)
        };
        let Some(spec) = spec else {
            let d = Diagnostic::error("SEM-001", format!("unknown element `{}`", e.name), e.pos, len);
            let component = capitalize(&e.name);
            let d = if e.name == "else" {
                d.help("`else` belongs right after the `}` of an if: if x { ... } else { ... }")
            } else if e.name.contains('.') {
                d.help(format!("data in text goes inside quotes: \"{{{}}}\"", e.name))
            } else if self.defs.components.contains_key(&component) {
                d.help(format!("components start with a capital letter: {component}"))
            } else if let Some(s) = suggest(&e.name, elements::names()) {
                d.help(format!("did you mean `{s}`?"))
            } else {
                d
            };
            return self.report(d);
        };

        self.placement(e, parent);
        self.attributes(e, &spec);
        self.common(e);

        match spec.text {
            TextRule::None => {
                if let Some(v) = &e.value {
                    self.report(Diagnostic::error("ATTR-006", format!("`{}` takes no text", e.name), v.pos, v.len));
                }
            }
            TextRule::Required(code, message, help) if e.value.is_none() && e.children.is_empty() => {
                self.report(Diagnostic::error(code, message, e.pos, len).help(help));
            }
            _ => {}
        }
        if !spec.children && !e.children.is_empty() {
            self.report(Diagnostic::error("SEM-008", format!("`{}` can't have children", e.name), e.pos, len));
        }

        match e.name.as_str() {
            "page" => self.page(e),
            "script" | "style" => self.code_text(e),
            "props" => self.names.extend(e.args.iter().filter_map(Arg::name).map(String::from)),
            "action" => self.action_name(e),
            "img" => self.image(e),
            "a" => self.link_text(e),
            "iframe" if e.arg("title").is_none() => {
                self.report(
                    Diagnostic::warning("A11Y-009", "iframe has no title", e.pos, len)
                        .help("describe it for screen readers, e.g. title: \"Map of our store\""),
                );
            }
            "video" => self.video(e),
            "raw" if e.value.as_ref().and_then(Value::as_str).is_some_and(reads_data) => {
                let v = e.value.as_ref().expect("checked above");
                self.report(
                    Diagnostic::warning("SEC-002", "raw HTML from data isn't escaped", v.pos, v.len)
                        .help("only use trusted HTML here; p \"{...}\" escapes it"),
                );
            }
            "button"
                if e.value.is_none()
                    && e.children.is_empty()
                    && ["aria-label", "aria-labelledby", "title"].iter().all(|a| e.arg(a).is_none()) =>
            {
                self.report(
                    Diagnostic::error("A11Y-005", "button has no text", e.pos, len)
                        .help("button \"Send\", or aria-label: \"...\" for an icon button"),
                );
            }
            "form" => self.form(e),
            "field" => self.field_type(e),
            _ => {}
        }

        let was_control = std::mem::replace(&mut self.in_control, false);
        let was_form = if e.name == "form" { std::mem::replace(&mut self.form, InForm::Yes) } else { self.form };
        let was_foreign = std::mem::replace(&mut self.in_foreign, html::is_foreign(&e.name));
        let was_html_template = self.in_html_template;
        self.in_html_template |= e.name == "template";
        self.items(&e.children, &e.name);
        self.in_control = was_control;
        self.form = was_form;
        self.in_foreign = was_foreign;
        self.in_html_template = was_html_template;
    }

    fn form(&mut self, e: &Element) {
        let len = e.name.len();
        let method = e.arg("method").and_then(Value::as_str).map(str::to_ascii_lowercase);
        if let (Some(method), Some(v)) = (&method, e.arg("method"))
            && !FORM_METHODS.contains(&method.as_str())
        {
            self.report(
                Diagnostic::error("FORM-006", format!("`{method}` isn't a form method"), v.pos, v.len)
                    .help(format!("use one of {}", FORM_METHODS.join(", "))),
            );
        }
        if !contains_submit(&e.children, self.defs, 0) {
            self.report(
                Diagnostic::warning("FORM-003", "form has no button to submit it", e.pos, len)
                    .help("add button \"Send\""),
            );
        }
        if e.arg("name").is_none() && method.as_deref() != Some("dialog") && contains_element(&e.children, "field") {
            self.report(
                Diagnostic::warning("FORM-004", "form has fields but no name, so agents can't call it", e.pos, len)
                    .help("add name: \"signup\""),
            );
        }
    }

    fn video(&mut self, e: &Element) {
        let len = e.name.len();
        if e.main_or("src").is_none() && !contains_element(&e.children, "source") {
            self.report(
                Diagnostic::error("SEM-004", "video has no source", e.pos, len)
                    .help("video(\"/clip.mp4\", captions: \"/clip.vtt\"), or source elements inside it"),
            );
        }
        let captioned = e.arg("captions").is_some() || contains_element(&e.children, "track");
        if !captioned && !e.flag("muted") {
            self.report(
                Diagnostic::error("A11Y-003", "video has no captions", e.pos, len)
                    .help("add captions: \"/clip.vtt\"; a video without sound takes muted: true instead"),
            );
        }
        if e.flag("autoplay") && !e.flag("muted") {
            self.report(
                Diagnostic::warning("A11Y-011", "video plays sound on its own", e.pos, len)
                    .help("add muted: true, or remove autoplay"),
            );
        }
    }

    /// Scripts and styles are written as they are, so `{user.name}` in one stays that text. When it names data the
    /// page has, the author most likely expected the value.
    fn code_text(&mut self, e: &Element) {
        let Some(v) = e.value.as_ref().filter(|_| !elements::is_json_script(e)) else { return };
        let Some(text) = v.as_str() else { return };
        let data = pieces(text, Braces::AsWritten).into_iter().find_map(|piece| match piece {
            Piece::Data { name, .. } if self.names.iter().any(|n| name.split('.').next() == Some(n.as_str())) => {
                Some(name)
            }
            _ => None,
        });
        if let Some(name) = data {
            self.report(
                Diagnostic::warning("SEC-005", format!("`{{{name}}}` in {} is written as-is", e.name), v.pos, v.len)
                    .help("scripts and styles don't read data; pass it in a data-* attribute or a script(type: \"application/json\")"),
            );
        }
    }

    /// Action names become URLs (`/_a/<name>`), so they're limited to safe characters.
    fn action_name(&mut self, e: &Element) {
        let Some(v) = e.main_or("name") else { return };
        let Some(name) = v.as_str() else { return };
        let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
        if !valid {
            self.report(
                Diagnostic::error("SEM-014", format!("`{name}` isn't a valid action name"), v.pos, v.len)
                    .help("use letters, digits, dots, dashes and underscores, e.g. \"cart.add\""),
            );
        }
    }

    /// Elements that are valid but in the wrong place.
    fn placement(&mut self, e: &Element, parent: &str) {
        let (code, message, help): (&str, String, Option<&str>) = match (parent, e.name.as_str()) {
            ("", "page" | "allow") => return,
            ("", name) => ("SEM-003", format!("`{name}` must be inside the page"), None),
            (_, "page") => ("SEM-003", "`page` must be at the top of the file".into(), None),
            // `body(class: "dark")` sets the attributes of the <body> Hypra writes
            (p, "body")
                if matches!(p, "page" | "layout") && !self.in_control && e.children.is_empty() && e.value.is_none() =>
            {
                return;
            }
            (_, "html" | "body") => (
                "SEM-009",
                format!("Hypra writes <{}> itself", e.name),
                Some(
                    "set <html> attributes on page(...), and <body> attributes with body(...) inside the page or layout",
                ),
            ),
            (_, "title") => ("SEM-009", "Hypra writes <title> itself".into(), Some("page(title: \"...\")")),
            (p, "props") if p != "page" || self.in_control => (
                "SEM-003",
                "`props` belongs directly inside the page".into(),
                Some("components get their data as parameters: component Card(title: Text)"),
            ),
            (p, "head") if !matches!(p, "page" | "layout" | "component") || self.in_control => {
                ("SEM-010", "`head` belongs directly inside a page, layout or component".into(), None)
            }
            // inside an HTML template, slot is the web components element
            (_, "slot") if self.in_html_template => return,
            // content for a named slot, given directly inside a component call; `call` checks the name
            (p, "slot") if is_name(p) && slot_name(e).is_some() => return,
            (p, "slot") if is_name(p) => (
                "SEM-006",
                "a slot given to a component needs a name".into(),
                Some("slot(\"actions\") { ... }; content without a slot goes to the default slot"),
            ),
            (_, "slot") if !self.in_template => (
                "SEM-006",
                "`slot` is only allowed inside a component or layout".into(),
                Some("slot marks where a component or layout renders the content it was given"),
            ),
            (_, "field") if self.form == InForm::No => ("FORM-005", "`field` must be inside a form".into(), None),
            (_, "field") if self.form == InForm::InControl => (
                "FORM-005",
                "a form's fields can't be inside for or if".into(),
                Some("fields are the form's fixed inputs; use a Choice field for options that come from data"),
            ),
            (_, name @ ("form" | "action")) if self.form != InForm::No => (
                "SEM-003",
                format!("`{name}` can't be inside a form"),
                Some("an action is a form of its own, and forms can't be nested; put it next to the form"),
            ),
            (p, name @ ("form" | "action")) if p == "p" || html::is_inline(p) || is_heading(p) => (
                "SEM-003",
                format!("`{name}` can't be inside {p}"),
                Some("an action is a form, which is a block; put it next to the text"),
            ),
            ("head", name) if !html::HEAD.contains(&name) && !matches!(name, "raw" | "allow") => (
                "SEM-011",
                format!("`{name}` doesn't belong in head"),
                Some("head takes meta, link, script, style, base, noscript and template"),
            ),
            _ => return,
        };
        let mut d = Diagnostic::error(code, message, e.pos, e.name.len());
        if let Some(help) = help {
            d = d.help(help);
        }
        self.report(d);
    }

    fn attributes(&mut self, e: &Element, spec: &Spec) {
        let mut seen: Vec<&str> = vec![];
        for (i, a) in e.args.iter().enumerate() {
            let (name, known): (&str, Option<&Attr>) = match &a.key {
                None => match (&spec.main, i) {
                    (Some(main), 0) => (main.name, Some(main)),
                    (Some(_), _) => {
                        self.report(
                            Diagnostic::error(
                                "ATTR-004",
                                "only the first argument can be unnamed",
                                a.value.pos,
                                a.value.len,
                            )
                            .help(match &a.value.kind {
                                ValueKind::Str(_) => {
                                    format!("text goes after the parentheses: {}(...) \"...\"", e.name)
                                }
                                _ => "name it, e.g. title: value".to_string(),
                            }),
                        );
                        continue;
                    }
                    (None, _) => {
                        let message = format!("`{}` takes no unnamed argument", e.name);
                        self.report(Diagnostic::error("ATTR-004", message, a.value.pos, a.value.len));
                        continue;
                    }
                },
                Some(key) => {
                    if key.quoted && !is_attribute_name(&key.name) {
                        let message = format!("`{}` isn't a valid attribute name", key.name);
                        self.report(
                            Diagnostic::error("ATTR-008", message, key.pos, key.name.len() + 2)
                                .help("attribute names can't contain spaces, quotes, =, > or /"),
                        );
                        continue;
                    }
                    let known = spec.main.iter().chain(spec.attrs).find(|x| x.name == key.name);
                    // event handlers pass too, with a warning below
                    let passes = spec.html
                        && (key.quoted
                            || key.name.contains('-')
                            || key.name.starts_with("on")
                            || html::ATTRIBUTES.contains(&key.name.as_str()));
                    if known.is_none() && !passes && spec.open.is_none() {
                        self.unknown_attribute(e, spec, &key.name, key.pos);
                        continue;
                    }
                    (key.name.as_str(), known)
                }
            };
            self.set_once(&mut seen, name, a.key.as_ref().map_or(a.value.pos, |k| k.pos));
            if let Some(default) = &a.default {
                self.report(
                    Diagnostic::error(
                        "ATTR-007",
                        "defaults are only for component parameters",
                        default.pos,
                        default.len,
                    )
                    .help("component Card(featured: Bool = false) { ... }"),
                );
            }
            if spec.html && known.is_none() && name.starts_with("on") && name.len() > 2 {
                self.report(
                    Diagnostic::warning("SEC-001", format!("inline event handler `{name}`"), a.value.pos, a.value.len)
                        .help("put scripts in head { script(\"/app.js\") }"),
                );
            }
            let kind = known.map(|x| x.kind).or(spec.open).unwrap_or(Kind::Html);
            self.value_kind(&a.value, kind);
        }

        for attr in spec.main.iter().chain(spec.attrs) {
            let Some((code, help)) = attr.required else { continue };
            let is_main = spec.main.as_ref().is_some_and(|m| m.name == attr.name);
            if e.arg(attr.name).is_none() && !(is_main && e.main().is_some()) {
                let message = format!("`{}` needs `{}`", e.name, attr.name);
                self.report(Diagnostic::error(code, message, e.pos, e.name.len()).help(help));
            }
        }
    }

    /// An attribute Hypra doesn't know. On an HTML element it's written as-is with a warning, since HTML keeps
    /// gaining attributes; a near miss of one of Hypra's own settings is an error, because it's most likely a typo.
    fn unknown_attribute(&mut self, e: &Element, spec: &Spec, name: &str, pos: Pos) {
        let own: Vec<&str> = spec.main.iter().chain(spec.attrs).map(|x| x.name).collect();
        let message = format!("`{}` has no attribute `{name}`", e.name);
        let d = match (suggest(name, own.iter().copied()), spec.html) {
            (Some(close), _) => {
                Diagnostic::error("ATTR-001", message, pos, name.len()).help(format!("did you mean `{close}`?"))
            }
            (None, true) => {
                let d = Diagnostic::warning("ATTR-001", format!("{message}, so it's written as-is"), pos, name.len());
                match suggest(name, html::ATTRIBUTES.iter().copied()) {
                    Some(close) => d.help(format!("did you mean `{close}`? If not, quote the name: \"{name}\": ...")),
                    None => d.help(format!("quote the name if it's meant: \"{name}\": ...")),
                }
            }
            (None, false) => Diagnostic::error("ATTR-001", message, pos, name.len()).help(format!(
                "`{}` takes: {}",
                e.name,
                own.join(", ")
            )),
        };
        self.report(d);
    }

    fn value_kind(&mut self, v: &Value, kind: Kind) {
        let ok = match (kind, &v.kind) {
            (_, ValueKind::Range(..)) => false,
            // a list is data for a component or a data-* attribute, e.g. data-sizes: ["S", "M"]
            (Kind::Html, ValueKind::List(_)) => v.is_fixed(),
            (Kind::Text, ValueKind::Str(_) | ValueKind::Num(_) | ValueKind::Path(_)) => true,
            (Kind::Static, ValueKind::Str(text)) => !reads_data(text),
            (Kind::Bool, ValueKind::Bool(_)) => true,
            (Kind::Name, ValueKind::Name(_)) => true,
            (Kind::Type, ValueKind::Name(_)) => true,
            (Kind::Type, ValueKind::Call(name, _)) => is_name(name),
            (Kind::Html, ValueKind::Str(_) | ValueKind::Num(_) | ValueKind::Bool(_) | ValueKind::Path(_)) => true,
            _ => false,
        };
        if ok {
            return;
        }
        let expected = match kind {
            Kind::Text => "text or data",
            Kind::Static => "fixed text in quotes, without data",
            Kind::Bool => "true or false, without data",
            Kind::Name => "a name, e.g. Shop",
            Kind::Type => "a type, e.g. Email or Int(1..5)",
            Kind::Html => "text, a number, true, false or data",
        };
        let mut d = Diagnostic::error("ATTR-003", format!("expected {expected}"), v.pos, v.len);
        let field_type = v.as_path().and_then(|p| FIELD_TYPES.iter().find(|t| t.name.eq_ignore_ascii_case(p)));
        if let (Kind::Type, Some(t)) = (kind, field_type) {
            d = d.help(format!("types start with a capital letter: {}", t.name));
        } else if let ValueKind::Name(name) = &v.kind {
            d = d.help(format!("`{name}` is a name; data is lowercase, text is quoted"));
        }
        self.report(d);
    }

    fn page(&mut self, e: &'a Element) {
        let defs = self.defs;
        if let Some(v) = e.arg("layout") {
            self.layout_name(None, v);
        }
        let chain = layout_chain(defs, e.arg("layout"));
        let setting = |name: &str| e.arg(name).or_else(|| chain_setting(&chain, name));
        // search results show the description; a page kept out of them doesn't need one
        let noindex = setting("robots").and_then(static_text).is_some_and(|r| r.contains("noindex"));
        if setting("description").is_none() && !noindex {
            self.report(
                Diagnostic::error("SEO-001", "`page` needs `description`", e.pos, 4)
                    .help("add description: \"...\" (search results and link previews use it), here or in the layout"),
            );
        }

        // fixed metadata; metadata from data is checked when the page is built
        let suffix = chain_setting(&chain, "title_suffix");
        let suffix = match suffix {
            Some(v) => static_text(v),
            None => Some(""),
        };
        if let (Some(title), Some(suffix)) = (e.arg("title"), suffix)
            && let Some(text) = static_text(title)
            && let Some(d) = rules::title(&format!("{text}{suffix}"), (title.pos, title.len))
        {
            self.report(d);
        }
        if let Some(v) = e.arg("description")
            && let Some(d) = static_text(v).and_then(|text| rules::description(text, (v.pos, v.len)))
        {
            self.report(d);
        }
        if let Some(v) = e.arg("canonical")
            && let Some(d) = static_text(v).and_then(|text| rules::canonical(text, (v.pos, v.len)))
        {
            self.report(d);
        }

        // the page as it renders: layout, page content in its slot, components expanded
        let mut outline = Outline::default();
        let allowed = self.allowed.clone();
        let mut walk = Walk { slots: vec![], components: vec![], allowed, in_loop: false, branches: vec![] };
        match chain.split_last() {
            Some((outermost, inner)) => {
                walk.slots.push(&e.children);
                walk.slots.extend(inner.iter().map(|layout| &layout.body[..]));
                self.outline(&outermost.body, &mut walk, &mut outline);
            }
            None => self.outline(&e.children, &mut walk, &mut outline),
        }
        self.headings(e, &outline.headings);
        self.unique_ids(&outline.ids);
    }

    /// Collects headings and fixed ids in render order, following layouts, components and slots.
    fn outline(&self, items: &'a [Item], walk: &mut Walk<'a>, out: &mut Outline) {
        let mut pending: Vec<String> = vec![];
        for item in items {
            out.visited += 1;
            if out.visited > MAX_OUTLINE {
                return;
            }
            if let Item::Element(e) = item
                && e.name == "allow"
            {
                pending.extend(item.allowed_rule().and_then(Value::as_str).map(String::from));
                continue;
            }
            let outer = walk.allowed.len();
            walk.allowed.append(&mut pending);
            match item {
                Item::Element(e) if is_name(&e.name) => {
                    // recursion is reported when lowering; here it just stops
                    if let Some(def) = self.defs.components.get(&e.name)
                        && !walk.components.contains(&e.name.as_str())
                    {
                        walk.components.push(&e.name);
                        walk.slots.push(&e.children);
                        self.outline(&def.body, walk, out);
                        walk.slots.pop();
                        walk.components.pop();
                    }
                }
                Item::Element(e) if e.name == "slot" => {
                    if let Some(content) = walk.slots.pop() {
                        // a named slot shows its content from the call, or its own as a fallback
                        match given_content(content, e) {
                            Some(items) => {
                                for item in items {
                                    self.outline(std::slice::from_ref(item), walk, out);
                                }
                            }
                            None => self.outline(&e.children, walk, out),
                        }
                        walk.slots.push(content);
                    }
                }
                Item::Element(e) => {
                    let level =
                        e.name.strip_prefix('h').and_then(|n| n.parse::<usize>().ok()).filter(|l| (1..=6).contains(l));
                    let place = Place { in_loop: walk.in_loop, branches: walk.branches.clone() };
                    if let Some(level) = level {
                        let allowed = walk.allowed.clone();
                        out.headings.push(Heading { level, pos: e.pos, allowed, place: place.clone() });
                    }
                    if let Some(id) = e.arg("id")
                        && let Some(text) = static_text(id)
                    {
                        let allowed = walk.allowed.clone();
                        out.ids.push(IdUse { id: text.to_string(), pos: id.pos, len: id.len, allowed, place });
                    }
                    self.outline(&e.children, walk, out);
                }
                Item::For(f) => {
                    let was = std::mem::replace(&mut walk.in_loop, true);
                    self.outline(&f.body, walk, out);
                    walk.in_loop = was;
                }
                Item::If(c) => {
                    out.ifs += 1;
                    let id = out.ifs;
                    for (branch, items) in [(true, &c.then), (false, &c.otherwise)] {
                        walk.branches.push((id, branch));
                        self.outline(items, walk, out);
                        walk.branches.pop();
                    }
                }
                _ => {}
            }
            walk.allowed.truncate(outer);
        }
    }

    fn headings(&mut self, page: &Element, headings: &[Heading]) {
        if !headings.iter().any(|h| h.level == 1) {
            self.report(
                Diagnostic::warning("SEO-004", "page has no h1", page.pos, 4)
                    .help("add h1 \"...\" with the page's main topic"),
            );
        }
        let h1s: Vec<&Heading> = headings.iter().filter(|h| h.level == 1).collect();
        for (i, h) in h1s.iter().enumerate() {
            let message = if h.place.in_loop {
                "h1 inside a loop repeats on the page"
            } else if h1s[..i].iter().any(|earlier| !earlier.place.in_loop && earlier.place.renders_with(&h.place)) {
                "more than one h1 on this page"
            } else {
                continue;
            };
            let d = Diagnostic::error("SEO-003", message, h.pos, 2)
                .help("a page has one h1 with its main topic; use h2 for sections");
            self.report_under(d, &h.allowed);
        }
        for (i, h) in headings.iter().enumerate() {
            // the heading before it on the page, leaving out other branches of the same `if`
            let previous = headings[..i].iter().rev().find(|p| p.place.renders_with(&h.place)).map_or(0, |p| p.level);
            if previous > 0 && h.level > previous + 1 {
                let message = format!("h{} follows h{previous}, skipping a level", h.level);
                let d = Diagnostic::warning("A11Y-006", message, h.pos, 2)
                    .help(format!("use h{}; screen readers navigate by heading levels", previous + 1));
                self.report_under(d, &h.allowed);
            }
        }
    }

    fn unique_ids(&mut self, ids: &[IdUse]) {
        for (i, id) in ids.iter().enumerate() {
            let message = if id.place.in_loop {
                format!("id `{}` is inside a loop, so it repeats", id.id)
            } else if ids[..i].iter().any(|earlier| earlier.id == id.id && earlier.place.renders_with(&id.place)) {
                format!("id `{}` is used more than once", id.id)
            } else {
                continue;
            };
            let d = Diagnostic::warning("SEM-012", message, id.pos, id.len)
                .help("ids must be unique on a page; build it from data in a loop, e.g. id: \"item-{item.id}\"");
            self.report_under(d, &id.allowed);
        }
    }

    /// Checks every element gets.
    fn common(&mut self, e: &Element) {
        self.aria_role(e);
        if let Some(v) = e.arg("lang")
            && let Some(d) = static_text(v).and_then(|tag| rules::lang(tag, (v.pos, v.len)))
        {
            self.report(d);
        }
        if let Some(v) = e.arg("tabindex")
            && let ValueKind::Num(n) | ValueKind::Str(n) = &v.kind
            && n.trim().parse::<i64>().is_ok_and(|n| n > 0)
        {
            self.report(
                Diagnostic::warning("A11Y-010", "a positive tabindex changes the keyboard order", v.pos, v.len)
                    .help("use tabindex: 0 to make it focusable, and order the page instead"),
            );
        }
        for (i, a) in e.args.iter().enumerate() {
            let name = match &a.key {
                Some(key) => Some(key.name.as_str()),
                None if i == 0 => html::main_attribute(&e.name),
                None => None,
            };
            if let Some(name) = name {
                self.attribute_security(e, name, &a.value);
            }
        }
    }

    /// A div or span is ignored by screen readers, so naming it or giving it a state needs a role.
    fn aria_role(&mut self, e: &Element) {
        const NEEDS_ROLE: &[&str] = &[
            "aria-label",
            "aria-labelledby",
            "aria-expanded",
            "aria-checked",
            "aria-selected",
            "aria-pressed",
            "aria-level",
            "aria-valuenow",
            "aria-valuemin",
            "aria-valuemax",
            "aria-required",
            "aria-readonly",
        ];
        if !matches!(e.name.as_str(), "div" | "span") || e.arg("role").is_some() {
            return;
        }
        let Some(a) = e.args.iter().find(|a| a.name().is_some_and(|n| NEEDS_ROLE.contains(&n))) else { return };
        let name = a.name().unwrap_or_default();
        let pos = a.key.as_ref().map_or(a.value.pos, |k| k.pos);
        self.report(
            Diagnostic::warning(
                "A11Y-012",
                format!("`{name}` on a {} without a role does nothing", e.name),
                pos,
                name.len(),
            )
            .help("add a role, e.g. role: \"group\", or use an element like nav, section or button"),
        );
    }

    /// Reports an attribute or argument that `seen` already has, then adds it.
    fn set_once<'n>(&mut self, seen: &mut Vec<&'n str>, name: &'n str, pos: Pos) {
        if seen.contains(&name) {
            self.report(Diagnostic::error("ATTR-005", format!("`{name}` is set twice"), pos, name.len()));
        }
        seen.push(name);
    }

    /// URLs that run code, forms sent elsewhere, and data inside attributes that run or load code.
    fn attribute_security(&mut self, e: &Element, name: &str, v: &Value) {
        // the fixed text before any data already decides the scheme: "javascript:{x}"
        let fixed = v.as_str().map(|text| text.split('{').next().unwrap_or_default());
        let animated = elements::animates_link(e) && name == "values";
        let written_code = match fixed {
            Some(text) if animated => text.split(';').any(runs_code),
            Some(text) => elements::is_url(e, name) && runs_code(text),
            None => false,
        };
        if written_code {
            self.report(
                Diagnostic::error("SEC-003", "this URL runs code instead of opening a page", v.pos, v.len)
                    .help("javascript:, vbscript: and data:text/html URLs aren't allowed; use a script or an action"),
            );
        }
        // a list of URLs can't be checked as one, so data in it isn't allowed
        if animated && !v.is_fixed() {
            self.report(
                Diagnostic::error("SEC-007", "data in the `values` of an animated link could run code", v.pos, v.len)
                    .help(
                        "animate the link with `to`, which blocks URLs that run code, or write the values in the page",
                    ),
            );
        }
        // both decide where a form goes, and forms carry hidden fields such as the CSRF token
        let sends_form = match name {
            "formaction" => !static_text(v).is_some_and(html::is_same_site),
            "href" if e.name == "base" => static_text(v).is_none(),
            _ => false,
        };
        if sends_form {
            let message = format!("`{name}` could send forms and their hidden fields to another site");
            self.report(
                Diagnostic::error("SEC-006", message, v.pos, v.len)
                    .help("use a fixed path on this site, or give the other address its own form"),
            );
        }
        let from_data = match &v.kind {
            ValueKind::Path(_) => true,
            ValueKind::Str(text) => reads_data(text),
            _ => false,
        };
        if !from_data {
            return;
        }
        let runs =
            ["on", "x-", "hx-on", "v-on:", "ng-", "@", ":"].iter().any(|p| name.starts_with(p)) || name == "srcdoc";
        // these decide which script, page, stylesheet or plugin the browser loads, or where it redirects
        let loads_code = matches!(
            (e.name.as_str(), name),
            ("script" | "iframe" | "embed", "src") | ("object", "data") | ("link", "href")
        ) || (e.name == "meta" && name == "content" && e.arg("http-equiv").is_some());
        let message = match (runs, loads_code) {
            (true, _) => format!("data inside `{name}`, which runs as code"),
            (false, true) => format!("data decides the `{name}` of {}, which loads code or redirects", e.name),
            (false, false) => return,
        };
        self.report(
            Diagnostic::warning("SEC-004", message, v.pos, v.len)
                .help("only use trusted data here; pass values in data-* attributes instead"),
        );
    }

    fn image(&mut self, e: &Element) {
        if e.flag("decorative") {
            return;
        }
        let Some(alt) = e.arg("alt") else {
            let (pos, len) = e.main().map_or((e.pos, 3), |v| (v.pos, v.len));
            return self.report(
                Diagnostic::error("A11Y-001", "image has no alt text", pos, len)
                    .help("add alt: \"...\" or decorative: true"),
            );
        };
        let Some(text) = static_text(alt) else { return };
        let text = text.trim().to_lowercase();
        let generic = self.words.is_some_and(|w| w.generic_alt.contains(&text.as_str()));
        const FILES: &[&str] = &[".jpg", ".jpeg", ".png", ".gif", ".webp", ".avif", ".svg"];
        let help = if text.is_empty() {
            "for an image without meaning, use decorative: true instead of an empty alt"
        } else if generic {
            "say what the image shows, e.g. alt: \"Blue running shoe, side view\""
        } else if FILES.iter().any(|ext| text.ends_with(ext)) || text.starts_with("img_") || text.starts_with("dsc") {
            "alt is a file name; say what the image shows instead"
        } else {
            return;
        };
        self.report(
            Diagnostic::warning("A11Y-008", "alt text doesn't describe the image", alt.pos, alt.len).help(help),
        );
    }

    /// Link text that says where the link goes, in the page's language, and that means one place.
    fn link_text(&mut self, e: &Element) {
        // an aria-label names the link instead of its text
        let (v, label) = match (e.arg("aria-label"), &e.value, e.children.as_slice()) {
            (Some(v), ..) => (v, true),
            _ if e.arg("aria-labelledby").is_some() => return,
            (None, Some(v), _) | (None, None, [Item::Text(v)]) => (v, false),
            _ => return,
        };
        let Some(text) = static_text(v) else { return };
        let text = text.trim().trim_end_matches(['.', '!', '…', ':']).to_lowercase();
        if !label && self.words.is_some_and(|w| w.vague_links.contains(&text.as_str())) {
            return self.report(
                Diagnostic::warning("A11Y-007", format!("link text `{text}` doesn't say where it goes"), v.pos, v.len)
                    .help("describe the target, e.g. a(\"/guide\") \"Read the setup guide\""),
            );
        }
        // people using screen readers often go through a list of a page's links, where only the text tells them
        // apart; this needs no knowledge of the language
        let Some(href) = e.main_or("href") else { return };
        let message = format!("links with the text `{text}` go to different places");
        match static_text(href) {
            None if self.loops > 0 => {
                self.report(Diagnostic::warning("A11Y-013", message, v.pos, v.len).help(
                    "put what's different into the text, e.g. \"Read more about {post.title}\", or set aria-label",
                ))
            }
            None => {}
            Some(target) => {
                let elsewhere = self.links.iter().any(|(t, h)| *t == text && h != target);
                if elsewhere {
                    self.report(
                        Diagnostic::warning("A11Y-013", message, v.pos, v.len)
                            .help("give each link text that says where it goes, or set aria-label"),
                    );
                }
                self.links.push((text, target.to_string()));
            }
        }
    }

    fn field_type(&mut self, e: &Element) {
        let Some(ty) = e.arg("type") else { return };
        let (name, args): (&str, &[Arg]) = match &ty.kind {
            ValueKind::Name(n) => (n, &[]),
            ValueKind::Call(n, a) => (n, a),
            _ => return,
        };
        let Some(kind) = elements::field_type(name) else {
            let names = FIELD_TYPES.iter().map(|t| t.name);
            let d = Diagnostic::error("TYPE-004", format!("unknown field type `{name}`"), ty.pos, ty.len);
            let d = match suggest(name, names.clone()) {
                Some(close) => d.help(format!("did you mean `{close}`?")),
                None => d.help(format!("field types: {}", names.collect::<Vec<_>>().join(", "))),
            };
            return self.report(d);
        };
        let example = match kind.args {
            FieldArgs::None => format!("{name} takes nothing in parentheses"),
            FieldArgs::Length => format!("{name}(1..80), or {name}(min: 1, max: 80)"),
            FieldArgs::Number => format!("{name}(1..5), or {name}(min: 0, max: 100)"),
            FieldArgs::Text => format!("{name}(min: \"...\", max: \"...\")"),
            FieldArgs::Options => "Choice(\"s\", \"m\"), or Choice(de: \"Germany\", at: \"Austria\")".to_string(),
        };
        let whole = |n: &str| n.parse::<u64>().is_ok();
        let fits = |n: &str| match kind.args {
            FieldArgs::Length => whole(n),
            _ if name == "Int" => n.parse::<i64>().is_ok(),
            _ => n.parse::<f64>().is_ok(),
        };
        let numbers = matches!(kind.args, FieldArgs::Length | FieldArgs::Number);
        for a in args {
            let ok = match (kind.args, a.name(), &a.value.kind) {
                (FieldArgs::Options, None, ValueKind::Num(_)) => true,
                (FieldArgs::Options, _, ValueKind::Str(text)) => !reads_data(text.as_str()),
                (_, None, ValueKind::Range(lo, hi)) if numbers => lo.iter().chain(hi.iter()).all(|n| fits(n.as_str())),
                (_, Some("min" | "max"), ValueKind::Num(n)) if numbers => fits(n.as_str()),
                (FieldArgs::Text, Some("min" | "max"), ValueKind::Str(text)) => !reads_data(text.as_str()),
                _ => false,
            };
            if !ok {
                let message = match kind.args {
                    FieldArgs::None => format!("`{name}` takes no arguments"),
                    FieldArgs::Options => "an option is fixed text or a number".to_string(),
                    _ => format!("`{name}` doesn't take this"),
                };
                self.report(Diagnostic::error("TYPE-005", message, a.value.pos, a.value.len).help(example.clone()));
            }
        }
        if kind.args == FieldArgs::Options && args.is_empty() {
            self.report(Diagnostic::error("TYPE-005", "`Choice` has no options", ty.pos, ty.len).help(example));
        }
        if let Some(v) = e.arg("default")
            && matches!(kind.input, "password" | "file" | "checkbox")
        {
            self.report(Diagnostic::error("TYPE-005", format!("a {name} field has no default"), v.pos, v.len));
        }
    }

    fn component_def(&mut self, d: &'a Def) {
        if !is_name(&d.name) {
            self.report(
                Diagnostic::error("COMP-005", "component names start with a capital letter", d.name_pos, d.name.len())
                    .help(format!("component {}(...)", capitalize(&d.name))),
            );
        }
        let mut seen = vec![];
        for p in &d.params {
            if let Some(name) = param_name(p) {
                if seen.contains(&name) {
                    let message = format!("parameter `{name}` is defined twice");
                    self.report(Diagnostic::error("COMP-008", message, p.value.pos, p.value.len));
                }
                seen.push(name);
            }
            let typed = p.key.is_some() && matches!(&p.value.kind, ValueKind::Name(_) | ValueKind::Call(..));
            let untyped = p.key.is_none() && p.value.as_path().is_some();
            if !typed && !untyped {
                self.report(
                    Diagnostic::error("COMP-008", "a parameter is `name` or `name: Type`", p.value.pos, p.value.len)
                        .help("component Card(title: Text, featured: Bool = false)"),
                );
            }
        }
        let was_template = std::mem::replace(&mut self.in_template, true);
        let was_form = std::mem::replace(&mut self.form, InForm::No);
        let params = d.params.iter().filter_map(param_name).map(String::from).collect();
        let page_names = std::mem::replace(&mut self.names, params);
        self.items(&d.body, "component");
        self.names = page_names;
        self.in_template = was_template;
        self.form = was_form;
    }

    /// The layout a page or layout sits in must exist, and layouts can't sit in each other in a loop. `owner` is
    /// the layout naming it, if it's a layout.
    fn layout_name(&mut self, owner: Option<&Def>, v: &Value) {
        let Some(name) = v.as_name() else { return };
        if !self.defs.layouts.contains_key(name) {
            let message = format!("unknown layout `{name}`");
            let d = Diagnostic::error("LAYOUT-001", message, v.pos, v.len);
            let d = match suggest(name, self.defs.layouts.keys().map(String::as_str)) {
                Some(close) => d.help(format!("did you mean `{close}`?")),
                None => d.help("define it with layout Name { ... } or import it with use \"layouts/name.hypra\""),
            };
            return self.report(d);
        }
        let chain = layout_chain(self.defs, Some(v));
        if let Some(d) = owner
            && chain.iter().any(|l| l.name == d.name)
        {
            self.report(
                Diagnostic::error("LAYOUT-005", format!("layout `{}` ends up inside itself", d.name), v.pos, v.len)
                    .help("a layout can sit inside another one, but never inside itself"),
            );
        }
    }

    fn layout_def(&mut self, d: &'a Def) {
        if !is_name(&d.name) {
            self.report(
                Diagnostic::error("LAYOUT-004", "layout names start with a capital letter", d.name_pos, d.name.len())
                    .help(format!("layout {} {{ ... }}", capitalize(&d.name))),
            );
        }
        for p in &d.params {
            let Some(key) = &p.key else {
                self.report(
                    Diagnostic::error("ATTR-004", "layout settings are named", p.value.pos, p.value.len)
                        .help("layout Shop(lang: \"en\", title_suffix: \" | Shop\")"),
                );
                continue;
            };
            if !LAYOUT_SETTINGS.contains(&key.name.as_str()) {
                let message = format!("layouts have no setting `{}`", key.name);
                let d = Diagnostic::error("ATTR-001", message, key.pos, key.name.len());
                let d = match suggest(&key.name, LAYOUT_SETTINGS.iter().copied()) {
                    Some(close) => d.help(format!("did you mean `{close}`?")),
                    None => d.help(format!("layout settings: {}", LAYOUT_SETTINGS.join(", "))),
                };
                self.report(d);
            }
            if key.name == "layout" {
                self.value_kind(&p.value, Kind::Name);
                self.layout_name(Some(d), &p.value);
            } else {
                self.value_kind(&p.value, Kind::Text);
            }
        }
        let rules: [(&str, rules::Rule); 2] = [("description", rules::description), ("lang", rules::lang)];
        for (setting, rule) in rules {
            if let Some(p) = d.params.iter().find(|p| p.name() == Some(setting))
                && let Some(diagnostic) = static_text(&p.value).and_then(|text| rule(text, (p.value.pos, p.value.len)))
            {
                self.report(diagnostic);
            }
        }
        if !contains_slot(&d.body, None) {
            self.report(
                Diagnostic::error("LAYOUT-002", "layout has no `slot`, so page content would disappear", d.pos, 6)
                    .help("put `slot` where the page content goes, e.g. main { slot }"),
            );
        }
        let was_template = std::mem::replace(&mut self.in_template, true);
        let was_form = std::mem::replace(&mut self.form, InForm::No);
        self.items(&d.body, "layout");
        self.in_template = was_template;
        self.form = was_form;
    }

    fn call(&mut self, e: &Element) {
        let defs = self.defs;
        let Some(def) = defs.components.get(&e.name) else {
            let mut d = Diagnostic::error("COMP-001", format!("unknown component `{}`", e.name), e.pos, e.name.len());
            if let Some(s) = suggest(&e.name, defs.components.keys().map(String::as_str)) {
                d = d.help(format!("did you mean `{s}`?"));
            }
            return self.report(d);
        };
        for (name, given) in e.children.iter().filter_map(named_content) {
            if !contains_slot(&def.body, Some(name)) {
                let v = given.main_or("name").expect("named content has a name");
                self.report(Diagnostic::error("COMP-010", format!("`{}` has no slot `{name}`", e.name), v.pos, v.len));
            }
        }
        let default_content = e.children.iter().any(|item| named_content(item).is_none());
        if default_content && !contains_slot(&def.body, None) {
            let message = format!("`{}` has no `slot`, so the content given to it would disappear", e.name);
            self.report(
                Diagnostic::error("COMP-010", message, e.pos, e.name.len())
                    .help("put the content next to the call instead of inside it, or add `slot` to the component"),
            );
        }
        let names: Vec<&str> = def.params.iter().filter_map(param_name).collect();
        let mut seen = vec![];
        for a in &e.args {
            if let Some(k) = &a.key {
                self.set_once(&mut seen, &k.name, k.pos);
            }
            match &a.key {
                None => self.report(
                    Diagnostic::error("COMP-006", "component arguments are named", a.value.pos, a.value.len)
                        .help(format!("{}(name: value)", e.name)),
                ),
                // HTML attributes go to the component's root element
                Some(k)
                    if !names.contains(&k.name.as_str())
                        && is_html_attribute(&k.name)
                        && forward_root(def).is_some() =>
                {
                    self.value_kind(&a.value, Kind::Html);
                }
                Some(k) if !names.contains(&k.name.as_str()) => {
                    let message = format!("`{}` has no parameter `{}`", e.name, k.name);
                    let d = Diagnostic::error("COMP-003", message, k.pos, k.name.len());
                    let d = match suggest(&k.name, names.iter().copied()) {
                        Some(close) => d.help(format!("did you mean `{close}`?")),
                        None if is_html_attribute(&k.name) => d.help(format!(
                            "`{}` renders more than one element, so it can't take HTML attributes; add a parameter",
                            e.name
                        )),
                        None => d.help(format!("parameters: {}", names.join(", "))),
                    };
                    self.report(d);
                }
                Some(_) => self.value_kind(&a.value, Kind::Html),
            }
        }
        for p in def.params.iter().filter(|p| p.default.is_none()) {
            let Some(name) = param_name(p) else { continue };
            if e.arg(name).is_none() {
                let message = format!("missing argument `{name}` for `{}`", e.name);
                self.report(
                    Diagnostic::error("COMP-002", message, e.pos, e.name.len())
                        .help(format!("{}({name}: ...)", e.name)),
                );
            }
        }
    }
}

/// The outline stops after this many items; lowering reports pages that expand further.
const MAX_OUTLINE: usize = 200_000;

#[derive(Default)]
struct Outline {
    headings: Vec<Heading>,
    ids: Vec<IdUse>,
    visited: usize,
    /// `if`s walked so far, to tell them apart.
    ifs: usize,
}

/// Where in the page something is: inside a loop, and in which branch of each enclosing `if`.
#[derive(Clone)]
struct Place {
    in_loop: bool,
    branches: Vec<(usize, bool)>,
}

impl Place {
    /// False when the two are in different branches of the same `if`, so only one of them renders.
    fn renders_with(&self, other: &Place) -> bool {
        !self.branches.iter().any(|(i, a)| other.branches.iter().any(|(j, b)| i == j && a != b))
    }
}

/// Where the outline walk is: the content each open slot renders, the components being expanded,
/// the rules switched off by `allow`, and whether it's inside a loop.
struct Walk<'a> {
    slots: Vec<&'a [Item]>,
    components: Vec<&'a str>,
    allowed: Vec<String>,
    in_loop: bool,
    /// The enclosing `if`s and which branch the walk is in.
    branches: Vec<(usize, bool)>,
}

struct Heading {
    level: usize,
    pos: Pos,
    /// Rules switched off by `allow` where the heading is.
    allowed: Vec<String>,
    place: Place,
}

struct IdUse {
    id: String,
    pos: Pos,
    len: usize,
    /// Rules switched off by `allow` where the id is.
    allowed: Vec<String>,
    place: Place,
}

/// What HTML allows as an attribute name.
fn is_attribute_name(name: &str) -> bool {
    !name.is_empty()
        && !name.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '>' | '/' | '=' | '<'))
}

/// The text of a string that doesn't read data, so it's known at compile time.
fn static_text(v: &Value) -> Option<&str> {
    v.as_str().filter(|text| !reads_data(text))
}

const FORM_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "dialog"];

fn is_heading(name: &str) -> bool {
    matches!(name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

/// Whether the items hold a submit button, also inside components they call.
fn contains_submit(items: &[Item], defs: &Defs, depth: usize) -> bool {
    any_element(items, &|e| {
        let kind = e.arg("type").and_then(Value::as_str).unwrap_or_default();
        let submits = match e.name.as_str() {
            "button" => !matches!(kind, "button" | "reset"),
            "input" => matches!(kind, "submit" | "image"),
            _ => false,
        };
        submits
            || (depth < 16 && defs.components.get(&e.name).is_some_and(|d| contains_submit(&d.body, defs, depth + 1)))
    })
}

fn contains_element(items: &[Item], name: &str) -> bool {
    any_element(items, &|e| e.name == name)
}

fn capitalize(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

/// A file of components and layouts for pages to import, with nothing else in it. It can be checked on its own,
/// but there's no page to build.
pub fn is_library(items: &[Item]) -> bool {
    items.iter().all(|i| matches!(i, Item::Use(_) | Item::Component(_) | Item::Layout(_)))
        && items.iter().any(|i| matches!(i, Item::Component(_) | Item::Layout(_)))
}
