//! What each element accepts: its unnamed main argument, its attributes, text and children.
//! The checker validates every element against this table; elements that aren't in it are unknown.

use crate::html;
use crate::parser::{Element, Value};

/// What kind of value an attribute takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A string, number or data path.
    Text,
    /// Text in quotes without data, because it's needed at compile time: a form's name, an action's name.
    Static,
    /// `true` or `false`, written in the page: it changes what Hypra writes, so it's needed at compile time.
    Bool,
    /// A layout, component or type name, e.g. `Shop`.
    Name,
    /// A field type, e.g. `Email` or `Int(1..5)`.
    Type,
    /// Any plain HTML attribute: a string, number, boolean or data path.
    Html,
}

pub struct Attr {
    pub name: &'static str,
    pub kind: Kind,
    /// When set, the attribute is required and a missing one is reported with this code and help.
    pub required: Option<(&'static str, &'static str)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextRule {
    None,
    Optional,
    /// Text (or children) is required; reported with this code, message and help.
    Required(&'static str, &'static str, &'static str),
}

pub struct Spec {
    /// The attribute an unnamed first argument sets, e.g. `href` for `a("/shoes")`.
    pub main: Option<Attr>,
    pub attrs: &'static [Attr],
    /// Accepts HTML attributes besides `attrs`.
    pub html: bool,
    /// Accepts attributes of any name with this kind, e.g. the parameters of an action.
    pub open: Option<Kind>,
    pub text: TextRule,
    pub children: bool,
}

const fn attr(name: &'static str, kind: Kind) -> Attr {
    Attr { name, kind, required: None }
}

const fn req(name: &'static str, kind: Kind, code: &'static str, help: &'static str) -> Attr {
    Attr { name, kind, required: Some((code, help)) }
}

const PAGE: &[Attr] = &[
    req("title", Kind::Text, "SEO-001", "add title: \"...\""),
    attr("description", Kind::Text),
    attr("layout", Kind::Name),
    attr("canonical", Kind::Text),
    attr("image", Kind::Text),
    attr("robots", Kind::Text),
    attr("lang", Kind::Text),
];

/// The page settings Hypra uses itself; every other attribute of a page goes on `<html>`.
pub fn page_settings() -> Vec<&'static str> {
    PAGE.iter().map(|a| a.name).collect()
}

/// Settings a `layout Name(...)` takes; pages inherit them.
pub const LAYOUT_SETTINGS: &[&str] = &["layout", "lang", "title_suffix", "site_name", "description", "image", "robots"];

const IMG: &[Attr] = &[attr("alt", Kind::Text), attr("decorative", Kind::Bool), attr("priority", Kind::Bool)];

const VIDEO: &[Attr] = &[attr("captions", Kind::Text)];

const FORM: &[Attr] = &[attr("name", Kind::Static), attr("description", Kind::Text), attr("method", Kind::Static)];

const ACTION: &[Attr] = &[attr("confirm", Kind::Text)];

const FIELD: &[Attr] = &[
    req("type", Kind::Type, "FORM-001", "add type: Email, Int(1..5), Text(max: 2000), ..."),
    req("label", Kind::Text, "FORM-002", "add label: \"...\""),
    attr("optional", Kind::Bool),
    attr("default", Kind::Text),
];

/// A field type: the control it becomes and how agents see it.
pub struct FieldType {
    pub name: &'static str,
    /// The input's `type`, or `textarea`, `select` or `checkbox`.
    pub input: &'static str,
    /// The JSON Schema type and format.
    pub schema: (&'static str, Option<&'static str>),
    pub args: FieldArgs,
}

/// What a field type takes in parentheses.
#[derive(Clone, Copy, PartialEq)]
pub enum FieldArgs {
    None,
    /// A length: `String(1..80)`, `Text(max: 2000)`.
    Length,
    /// A number: `Int(1..5)`, `Number(min: 0)`.
    Number,
    /// Fixed text: `Date(min: "2025-01-01")`.
    Text,
    /// The options: `Choice("s", "m")`, `Choice(de: "Germany", at: "Austria")`.
    Options,
}

const fn field(
    name: &'static str,
    input: &'static str,
    schema: (&'static str, Option<&'static str>),
    args: FieldArgs,
) -> FieldType {
    FieldType { name, input, schema, args }
}

/// The field types `field(..., type: ...)` accepts.
pub const FIELD_TYPES: &[FieldType] = &[
    field("String", "text", ("string", None), FieldArgs::Length),
    field("Text", "textarea", ("string", None), FieldArgs::Length),
    field("Password", "password", ("string", None), FieldArgs::Length),
    field("Email", "email", ("string", Some("email")), FieldArgs::Length),
    field("Url", "url", ("string", Some("uri")), FieldArgs::Length),
    field("Tel", "tel", ("string", None), FieldArgs::Length),
    field("Int", "number", ("integer", None), FieldArgs::Number),
    field("Number", "number", ("number", None), FieldArgs::Number),
    field("Date", "date", ("string", Some("date")), FieldArgs::Text),
    field("Time", "time", ("string", Some("time")), FieldArgs::Text),
    field("Choice", "select", ("string", None), FieldArgs::Options),
    field("Accept", "checkbox", ("boolean", None), FieldArgs::None),
    field("File", "file", ("string", Some("binary")), FieldArgs::None),
];

pub fn field_type(name: &str) -> Option<&'static FieldType> {
    FIELD_TYPES.iter().find(|t| t.name == name)
}

pub fn spec(name: &str) -> Option<Spec> {
    let none = Spec { main: None, attrs: &[], html: false, open: None, text: TextRule::None, children: false };
    let spec = match name {
        // any other HTML attribute of a page, like `dir` or `class`, goes on <html>
        "page" => Spec { attrs: PAGE, children: true, html: true, ..none },
        "props" => Spec { open: Some(Kind::Type), ..none },
        "head" => Spec { children: true, ..none },
        "slot" => Spec { main: Some(attr("name", Kind::Static)), children: true, ..none },
        "raw" => Spec { text: TextRule::Required("ATTR-002", "`raw` needs the HTML", "raw \"<b>html</b>\""), ..none },
        "allow" => Spec {
            main: Some(req("rule", Kind::Static, "ALLOW-002", "allow(\"SEO-003\") \"reason\"")),
            text: TextRule::Required(
                "ALLOW-001",
                "`allow` needs a written reason",
                "allow(\"SEO-003\") \"why this rule doesn't apply here\"",
            ),
            ..none
        },
        "action" => Spec {
            main: Some(req("name", Kind::Static, "SEM-005", "action(\"cart.add\", id: product.id) \"Add to cart\"")),
            attrs: ACTION,
            open: Some(Kind::Html),
            text: TextRule::Required(
                "A11Y-004",
                "action has no label",
                "add the button text: action(\"cart.add\") \"Add to cart\"",
            ),
            ..none
        },
        "field" => Spec {
            main: Some(req("name", Kind::Static, "FORM-001", "field(\"email\", type: Email, label: \"Email\")")),
            attrs: FIELD,
            html: true,
            ..none
        },
        tag if html::is_tag(tag) || html::is_custom_element(tag) => html_spec(tag),
        _ => return None,
    };
    Some(spec)
}

/// A script holding JSON, like structured data for search engines, rather than code: `{name}` in it reads data.
pub fn is_json_script(el: &Element) -> bool {
    let kind = el.arg("type").and_then(Value::as_str).unwrap_or_default().to_ascii_lowercase();
    el.name == "script" && (kind == "application/json" || kind.ends_with("+json") || kind == "importmap")
}

/// The rules for an HTML element or custom element.
pub fn html_spec(tag: &str) -> Spec {
    let main = html::main_attribute(tag).map(|name| match tag {
        "img" => req(name, Kind::Text, "SEM-004", "img(\"/photo.jpg\", alt: \"...\")"),
        _ => attr(name, Kind::Text),
    });
    let (attrs, text) = match tag {
        "img" => (IMG, TextRule::None),
        "video" => (VIDEO, TextRule::Optional),
        "form" => (FORM, TextRule::None),
        "a" => (&[][..], TextRule::Required("A11Y-002", "link has no text", "a(\"/about\") \"About us\"")),
        _ if html::is_void(tag) => (&[][..], TextRule::None),
        _ => (&[][..], TextRule::Optional),
    };
    // custom elements define their own attributes, and so do SVG and MathML
    let open = (html::is_custom_element(tag) || html::is_foreign(tag)).then_some(Kind::Html);
    // scripts and styles hold code as text, never elements
    let children = !html::is_void(tag) && !matches!(tag, "script" | "style");
    Spec { main, attrs, html: true, open, text, children }
}

/// Every element name, for "did you mean" hints.
pub fn names() -> impl Iterator<Item = &'static str> {
    ["page", "props", "head", "slot", "raw", "allow", "action", "field"].into_iter().chain(html::TAGS.iter().copied())
}

/// Whether an attribute's value is a URL, so data in it that would run code is blocked: the usual URL attributes,
/// and what SVG's `set` and `animate` write into a link.
pub fn is_url(e: &Element, name: &str) -> bool {
    html::URL_ATTRIBUTES.contains(&name) || (animates_link(e) && matches!(name, "to" | "from" | "by"))
}

/// `set` or `animate` changing a link: whatever it sets becomes the link's URL. An attribute name that comes from
/// data could be a link, so it counts.
pub fn animates_link(e: &Element) -> bool {
    matches!(e.name.as_str(), "set" | "animate")
        && e.arg("attributeName").is_some_and(|v| {
            v.as_str().is_none_or(|name| {
                crate::parser::reads_data(name) || matches!(name.to_ascii_lowercase().as_str(), "href" | "xlink:href")
            })
        })
}
