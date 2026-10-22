//! Rules on page metadata. The checker applies them to fixed text at compile time; the interpreter applies
//! them to text that comes from data, once the data is known.

use crate::diag::Diagnostic;

use crate::ir::At;

/// A rule on one piece of metadata: the text and where it's written.
pub type Rule = fn(&str, At) -> Option<Diagnostic>;

/// Search results show about this many characters of a title.
const TITLE_MAX: usize = 60;
/// Search results show descriptions of roughly this length.
const DESCRIPTION_MIN: usize = 50;
const DESCRIPTION_MAX: usize = 160;

pub fn title(title: &str, (pos, len): At) -> Option<Diagnostic> {
    let length = title.chars().count();
    (length > TITLE_MAX).then(|| {
        Diagnostic::warning(
            "SEO-002",
            format!("title is {length} characters (search results show about {TITLE_MAX})"),
            pos,
            len,
        )
        .help("shorten the title")
    })
}

pub fn description(description: &str, (pos, len): At) -> Option<Diagnostic> {
    let length = description.chars().count();
    let problem = match length {
        n if n < DESCRIPTION_MIN => "short",
        n if n > DESCRIPTION_MAX => "long",
        _ => return None,
    };
    Some(
        Diagnostic::warning("SEO-006", format!("description is {length} characters, too {problem}"), pos, len)
            .help(format!("aim for {DESCRIPTION_MIN} to {DESCRIPTION_MAX} characters that summarize the page")),
    )
}

/// A language tag like `en`, `de-AT` or `zh-Hant`: a 2 or 3 letter language, then optional subtags.
pub fn lang(tag: &str, (pos, len): At) -> Option<Diagnostic> {
    let mut parts = tag.split('-');
    let language = parts.next().unwrap_or_default();
    let valid = (2..=3).contains(&language.len())
        && language.chars().all(|c| c.is_ascii_alphabetic())
        && parts.all(|part| (1..=8).contains(&part.len()) && part.chars().all(|c| c.is_ascii_alphanumeric()));
    (!valid).then(|| {
        Diagnostic::warning("SEM-013", format!("`{tag}` isn't a language code"), pos, len)
            .help("use a code like \"en\", \"de\" or \"pt-BR\"; screen readers pick their voice from it")
    })
}

pub fn canonical(url: &str, (pos, len): At) -> Option<Diagnostic> {
    let absolute = url.starts_with("https://") || url.starts_with("http://");
    (!absolute).then(|| {
        Diagnostic::warning("SEO-005", "canonical URL isn't absolute", pos, len)
            .help("search engines need the full URL, e.g. canonical: \"https://example.com/page\"")
    })
}
