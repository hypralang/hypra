//! Rules on page metadata. The checker applies them to fixed text at compile time; the interpreter applies
//! them to text that comes from data, once the data is known.

use crate::diag::Diagnostic;

use crate::ir::At;

/// A rule on one piece of metadata: the text and where it's written.
pub type Rule = fn(&str, At) -> Option<Diagnostic>;

/// Search results show about this many characters of a title, counted by `width`.
const TITLE_MAX: usize = 60;
/// Search results show descriptions of roughly this length, counted by `width`.
const DESCRIPTION_MIN: usize = 50;
const DESCRIPTION_MAX: usize = 160;

/// How wide text is, in the columns of a Latin letter. Search results cut titles and descriptions by width, not by
/// how many characters they have: Chinese, Japanese and Korean characters are about twice as wide, following
/// Unicode's East Asian Width, and combining marks take no room of their own.
pub fn width(text: &str) -> usize {
    text.chars()
        .map(|c| match u32::from(c) {
            0x0300..=0x036F | 0x200B..=0x200F | 0xFE00..=0xFE0F => 0,
            0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xA960..=0xA97F
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1F64F
            | 0x1F900..=0x1F9FF
            | 0x20000..=0x3FFFD => 2,
            _ => 1,
        })
        .sum()
}

pub fn title(title: &str, (pos, len): At) -> Option<Diagnostic> {
    let length = width(title);
    (length > TITLE_MAX).then(|| {
        Diagnostic::warning(
            "SEO-002",
            format!("title is {length} characters wide (search results show about {TITLE_MAX})"),
            pos,
            len,
        )
        .help("shorten the title")
    })
}

pub fn description(description: &str, (pos, len): At) -> Option<Diagnostic> {
    let length = width(description);
    let problem = match length {
        n if n < DESCRIPTION_MIN => "short",
        n if n > DESCRIPTION_MAX => "long",
        _ => return None,
    };
    Some(
        Diagnostic::warning("SEO-006", format!("description is {length} characters wide, too {problem}"), pos, len)
            .help(format!(
                "aim for {DESCRIPTION_MIN} to {DESCRIPTION_MAX} characters that summarize the page; Chinese, Japanese \
                 and Korean characters count twice"
            )),
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
