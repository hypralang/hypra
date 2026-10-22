use crate::lexer::Pos;
use serde_json::{Value as Json, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A compiler message with a stable code (e.g. `A11Y-001`), a location and an optional fix hint.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub pos: Pos,
    pub len: usize,
    pub help: Option<String>,
}

/// Diagnostics without repeats, in order: the first of each is kept.
pub fn unique(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let mut kept: Vec<Diagnostic> = Vec::with_capacity(diagnostics.len());
    for d in diagnostics {
        if !kept.contains(&d) {
            kept.push(d);
        }
    }
    kept
}

impl Diagnostic {
    pub fn error(code: &str, message: impl Into<String>, pos: Pos, len: usize) -> Self {
        Self { severity: Severity::Error, code: code.into(), message: message.into(), pos, len: len.max(1), help: None }
    }

    pub fn warning(code: &str, message: impl Into<String>, pos: Pos, len: usize) -> Self {
        Self { severity: Severity::Warning, ..Self::error(code, message, pos, len) }
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Renders the message rustc-style, with the source line and a caret under the problem.
    pub fn render(&self, file: &str, src: &str) -> String {
        let kind = if self.is_error() { "error" } else { "warning" };
        // a problem with the whole file has no line to show
        if self.pos.line == 0 {
            let mut out = format!("{kind}[{}]: {}\n --> {file}\n", self.code, self.message);
            if let Some(help) = &self.help {
                out.push_str(&format!("  = help: {help}\n"));
            }
            return out;
        }
        let line_no = self.pos.line.to_string();
        let pad = " ".repeat(line_no.len());
        let text = crate::line_of(src, self.pos.line);
        // tabs stay tabs, so the caret lines up however wide the terminal shows them
        let caret_pad: String =
            text.chars().take(self.pos.col.saturating_sub(1)).map(|c| if c == '\t' { '\t' } else { ' ' }).collect();
        let mut out = format!(
            "{kind}[{}]: {}\n{pad}--> {file}:{}:{}\n{pad} |\n{line_no} | {text}\n{pad} | {caret_pad}{}\n",
            self.code,
            self.message,
            self.pos.line,
            self.pos.col,
            "^".repeat(self.len)
        );
        if let Some(help) = &self.help {
            out.push_str(&format!("{pad} = help: {help}\n"));
        }
        out
    }

    pub fn to_json(&self, file: &str) -> Json {
        json!({
            "severity": if self.is_error() { "error" } else { "warning" },
            "code": self.code,
            "message": self.message,
            "file": file,
            "line": self.pos.line,
            "col": self.pos.col,
            "len": self.len,
            "help": self.help,
        })
    }
}

/// Returns the closest candidate within a small edit distance, for "did you mean" hints.
pub fn suggest<'a>(word: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let max = if word.chars().count() >= 4 { 2 } else { 1 };
    candidates
        .into_iter()
        .map(|c| (distance(word, c), c))
        .filter(|(d, _)| *d > 0 && *d <= max)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c.to_string())
}

fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Every rule the compiler reports, with what it means. `allow` accepts these codes.
#[rustfmt::skip]
pub const RULES: &[(&str, &str)] = &[
    ("A11Y-001", "image has no alt text"),
    ("A11Y-002", "link has no text"),
    ("A11Y-003", "video has no captions"),
    ("A11Y-004", "action has no label"),
    ("A11Y-005", "button has no text"),
    ("A11Y-006", "heading level skipped"),
    ("A11Y-007", "link text doesn't say where it goes"),
    ("A11Y-008", "alt text doesn't describe the image"),
    ("A11Y-009", "iframe has no title"),
    ("A11Y-010", "positive tabindex"),
    ("A11Y-011", "autoplaying video isn't muted"),
    ("A11Y-012", "ARIA name or state on a div or span without a role"),
    ("A11Y-013", "links with the same text go to different places"),
    ("ALLOW-001", "allow has no reason"),
    ("ALLOW-002", "allow has no rule code"),
    ("ALLOW-003", "allow names an unknown rule"),
    ("ATTR-001", "unknown attribute"),
    ("ATTR-002", "raw has no HTML"),
    ("ATTR-003", "wrong kind of value"),
    ("ATTR-004", "unnamed argument where a name is needed"),
    ("ATTR-005", "attribute set twice"),
    ("ATTR-006", "text on an element that takes none"),
    ("ATTR-007", "default outside a component parameter"),
    ("ATTR-008", "invalid attribute name"),
    ("COMP-001", "unknown component"),
    ("COMP-002", "missing component argument"),
    ("COMP-003", "unknown component argument"),
    ("COMP-004", "component defined twice"),
    ("COMP-005", "component name isn't capitalized"),
    ("COMP-006", "unnamed component argument"),
    ("COMP-007", "component calls itself"),
    ("COMP-008", "malformed component parameter"),
    ("COMP-009", "page expands to too many elements"),
    ("COMP-010", "content given to a component without a slot"),
    ("FORM-001", "field has no name or type"),
    ("FORM-002", "field has no label"),
    ("FORM-003", "form has no button"),
    ("FORM-004", "form with fields has no name"),
    ("FORM-005", "field outside a form"),
    ("FORM-006", "unknown form method"),
    ("IMPORT-001", "imported file can't be read"),
    ("LAYOUT-001", "unknown layout"),
    ("LAYOUT-002", "layout has no slot"),
    ("LAYOUT-003", "layout defined twice"),
    ("LAYOUT-004", "layout name isn't capitalized"),
    ("LAYOUT-005", "layout inside itself"),
    ("LEX-001", "unexpected character"),
    ("LEX-002", "unterminated string"),
    ("LEX-003", "unknown escape in a string"),
    ("PARSE-001", "syntax error"),
    ("PARSE-002", "nested too deeply"),
    ("SEC-001", "inline event handler"),
    ("SEC-002", "raw HTML from data"),
    ("SEC-003", "URL that runs code"),
    ("SEC-004", "data inside an attribute that runs or loads code"),
    ("SEC-005", "data name in a script or style, which is written as-is"),
    ("SEC-006", "formaction or base sending forms elsewhere"),
    ("SEC-007", "data in the values of an animated link"),
    ("SEM-001", "unknown element"),
    ("SEM-002", "file has no page"),
    ("SEM-003", "element in the wrong place"),
    ("SEM-004", "image or video has no source"),
    ("SEM-005", "action has no name"),
    ("SEM-006", "slot outside a component or layout"),
    ("SEM-007", "imported file contains a page"),
    ("SEM-008", "void element has children"),
    ("SEM-009", "html, body or title written by hand"),
    ("SEM-010", "head in the wrong place"),
    ("SEM-011", "element doesn't belong in head"),
    ("SEM-012", "id used more than once"),
    ("SEM-013", "lang isn't a language code"),
    ("SEM-014", "invalid action name"),
    ("SEO-001", "page has no title or description"),
    ("SEO-002", "title too long"),
    ("SEO-003", "more than one h1"),
    ("SEO-004", "page has no h1"),
    ("SEO-005", "canonical URL isn't absolute"),
    ("SEO-006", "description too short or too long"),
    ("TYPE-001", "prop missing from the data"),
    ("TYPE-002", "unknown name or field"),
    ("TYPE-003", "value isn't a list"),
    ("TYPE-004", "unknown field type"),
    ("TYPE-005", "wrong argument or default for a field type"),
];
