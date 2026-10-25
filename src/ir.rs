//! The intermediate representation between the checked syntax tree and an output backend.
//!
//! A [`Program`] is a flat list of operations that write to two channels (HTML and Markdown) and
//! record actions. Names are already resolved: page data is [`Expr::Data`], loop variables and
//! component parameters are numbered [`Expr::Var`] slots. Components and layouts are inlined.
//! Backends either run the program with data ([`crate::interp`]) or translate it to code.

use crate::lexer::Pos;
use serde_json::Value as Json;

pub struct Program {
    pub ops: Vec<Op>,
    /// Number of variable slots the program uses.
    pub vars: usize,
    /// Page data the program expects, declared with `props { ... }`.
    pub props: Vec<Prop>,
    pub meta: Meta,
}

pub struct Prop {
    pub name: String,
    pub pos: Pos,
}

/// Where a value is written: its position and length in the source.
pub type At = (Pos, usize);

/// Page metadata that backends also need outside the document, e.g. for the JSON output.
pub struct Meta {
    pub title: Expr,
    /// The title with the layout's `title_suffix`, as shown in `<title>`.
    pub full_title: Expr,
    pub title_at: Option<At>,
    pub description: Option<Expr>,
    pub description_at: Option<At>,
    pub lang: Expr,
    pub lang_at: Option<At>,
    pub canonical: Option<Expr>,
    pub canonical_at: Option<At>,
    /// Rule codes switched off by `allow` right before the page; they apply to the checks run on data too.
    pub allowed: Vec<String>,
}

pub enum Op {
    /// Static HTML, already escaped.
    Html(String),
    /// Static Markdown.
    Md(String),
    /// A computed value written to one channel.
    Value {
        to: Channel,
        expr: Expr,
        escape: Escape,
    },
    For {
        var: usize,
        list: Expr,
        source: String,
        pos: Pos,
        len: usize,
        body: Vec<Op>,
    },
    If {
        cond: Expr,
        then: Vec<Op>,
        otherwise: Vec<Op>,
    },
    Set {
        var: usize,
        value: Expr,
    },
    /// Records something a reader can do on the page.
    Action(ActionDef),
    /// Where a server adds hidden fields to a POST form, such as a CSRF token. `indent` is the line indent.
    /// With `action`, only when it's a local URL, so tokens never go to other sites.
    FormFields {
        indent: String,
        action: Option<Expr>,
    },
    /// The `<link rel="alternate">` to the page's Markdown. A static build links `<stem>.md`; a server links
    /// the page's own URL, which answers with Markdown when asked for it.
    AlternateLink {
        indent: String,
        stem: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Html,
    Md,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escape {
    /// HTML-escaped text.
    Html,
    /// Unchanged text.
    Raw,
    /// A JSON string literal.
    Json,
    /// A JSON string literal, then HTML-escaped (for inline event attributes).
    JsonHtml,
    /// A URL in an attribute: a `javascript:`, `vbscript:` or `data:text/html` URL is replaced, then HTML-escaped.
    Url,
    /// Text inside `<script>` or `<style>`: `</` is written as `<\/` so it can't close the element.
    Script,
    /// Data inside a JSON script: a JSON string with `<`, `>` and `&` escaped, so it can't close the element.
    ScriptJson,
    /// Markdown text: it can't start a heading, list or quote, or open a link, HTML tag, code span or table cell.
    Markdown,
    /// A Markdown link or image target: a safe URL that can't close the parentheses around it.
    MarkdownUrl,
    /// Text inside a Markdown code span: on one line, and no backtick run that could close the span.
    MarkdownCode,
    /// Text inside a fenced Markdown code block: no backtick run that could close the fence.
    MarkdownFence,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Json),
    /// A path into the page data, e.g. `product.name` → `["product", "name"]`.
    Data {
        path: Vec<String>,
        pos: Pos,
        len: usize,
    },
    /// A field path inside a variable slot, e.g. `review.author` → slot of `review`, `["author"]`.
    Var {
        id: usize,
        name: String,
        path: Vec<String>,
        pos: Pos,
        len: usize,
    },
    /// String interpolation: the parts' display texts joined.
    Concat(Vec<Expr>),
    /// True when the value's display text is this text, e.g. the option a select starts on.
    Equals(Box<Expr>, String),
}

pub struct ActionDef {
    pub name: String,
    pub description: Expr,
    pub method: String,
    pub endpoint: Expr,
    pub params: Vec<ParamDef>,
}

pub struct ParamDef {
    pub name: String,
    /// JSON Schema of an input field. Unused when `value` is set.
    pub schema: Json,
    /// A fixed value, e.g. `id: product.id`; its schema is `{ "type": ..., "const": value }`.
    pub value: Option<Expr>,
    /// What the field is for, for agents: its `description`, or else its label. Goes into the schema.
    pub description: Option<Expr>,
    pub required: bool,
}

impl Expr {
    /// Joins parts into one interpolation, flattening nested ones and merging adjacent literals.
    pub fn concat(parts: Vec<Expr>) -> Expr {
        let mut out: Vec<Expr> = vec![];
        for part in parts {
            let flat = match part {
                Expr::Concat(inner) => inner,
                other => vec![other],
            };
            for part in flat {
                match (out.last_mut(), part) {
                    (Some(Expr::Lit(prev)), Expr::Lit(next)) => *prev = Json::String(display(prev) + &display(&next)),
                    (_, part) => out.push(part),
                }
            }
        }
        match out.len() {
            0 => Expr::Lit(Json::String(String::new())),
            1 if matches!(out[0], Expr::Lit(_)) => out.pop().expect("one part"),
            _ => Expr::Concat(out),
        }
    }
}

/// The text a value shows as: strings as-is, null as nothing, everything else as JSON.
pub fn display(value: &Json) -> String {
    match value {
        Json::String(s) => s.clone(),
        Json::Null => String::new(),
        other => other.to_string(),
    }
}

pub fn apply(escape: Escape, text: String) -> String {
    match escape {
        Escape::Html => esc(&text),
        Escape::Raw => text,
        Escape::Json => Json::String(text).to_string(),
        Escape::JsonHtml => esc(&Json::String(text).to_string()),
        Escape::Url => esc(&safe_url(&text)),
        Escape::Script => text.replace("</", "<\\/"),
        Escape::ScriptJson => {
            Json::String(text).to_string().replace('<', "\\u003C").replace('>', "\\u003E").replace('&', "\\u0026")
        }
        Escape::Markdown => md_text(&text),
        Escape::MarkdownUrl => safe_url(&text).replace('(', "%28").replace(')', "%29"),
        Escape::MarkdownCode => md_code(&text.replace(['\r', '\n'], " ")),
        Escape::MarkdownFence => md_code(&text),
    }
}

/// Data shown as Markdown text stays text: it can't add a heading such as a fake `## Actions`, a list, a link
/// or HTML. Line breaks become spaces, so it can only start a line where the page starts one.
pub fn md_text(text: &str) -> String {
    let text = text.replace(['\r', '\n'], " ");
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        if matches!(c, '\\' | '`' | '[' | ']' | '<' | '|') {
            out.push('\\');
        }
        out.push(c);
    }
    // what the text starts with could make the line a heading, list item, rule or quote: `# `, `- `, `---`,
    // `1. `, `>`
    let lead = out.len() - out.trim_start_matches([' ', '\t']).len();
    let rest = &out[lead..];
    let ends_marker = |i: usize| rest[i..].chars().next().is_none_or(|c| c == ' ' || c == '\t');
    let run = |c: char| rest.len() - rest.trim_start_matches(c).len();
    let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let mark = match rest.chars().next() {
        Some('>') => Some(lead),
        Some(c @ ('-' | '+' | '*' | '_')) if ends_marker(1) || run(c) > 1 => Some(lead),
        Some('#') if ends_marker(run('#')) => Some(lead),
        Some(_) if digits > 0 && rest[digits..].starts_with(['.', ')']) && ends_marker(digits + 1) => {
            Some(lead + digits)
        }
        _ => None,
    };
    if let Some(i) = mark {
        out.insert(i, '\\');
    }
    out
}

/// A whole Markdown code span, wider when the text holds a backtick.
pub fn md_span(text: &str) -> String {
    let code = md_code(&text.replace(['\r', '\n'], " "));
    if code.contains('`') { format!("`` {code} ``") } else { format!("`{code}`") }
}

/// Breaks every run of backticks with a zero-width space, so the text can't close the code span or fence around
/// it. Replacing doesn't overlap, so one pass leaves two backticks of a run of three; the second breaks them.
fn md_code(text: &str) -> String {
    text.replace("``", "`\u{200B}`").replace("``", "`\u{200B}`")
}

/// URLs that run code when followed are replaced, so data can't turn a link into a script. Characters URLs
/// can't contain (spaces, quotes, braces, …) are percent-encoded, as a browser would when following the link.
pub fn safe_url(url: &str) -> String {
    if runs_code(url) {
        return "about:blank#blocked".to_string();
    }
    let mut out = String::with_capacity(url.len());
    for c in url.chars() {
        if c.is_ascii_control() || matches!(c, ' ' | '"' | '<' | '>' | '\\' | '^' | '`' | '{' | '|' | '}') {
            out.push_str(&format!("%{:02X}", c as u8));
        } else {
            out.push(c);
        }
    }
    out
}

/// Browsers ignore whitespace and control characters in a URL scheme, so they're ignored here too.
pub fn runs_code(url: &str) -> bool {
    let scheme: String = url
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && !c.is_ascii_control())
        .flat_map(char::to_lowercase)
        .collect();
    ["javascript:", "vbscript:", "data:text/html"].iter().any(|bad| scheme.starts_with(bad))
}

/// Escapes text for HTML content and double-quoted attributes.
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
