//! Words that don't say anything, per language: link texts that don't say where a link goes, and alt texts that
//! don't say what an image shows. A page is checked against the words of its own `lang`; a page in a language
//! that isn't listed here isn't checked for them, rather than against another language's words.

pub struct Words {
    /// Link texts that don't say where a link goes, in lowercase.
    pub vague_links: &'static [&'static str],
    /// Alt texts that only say there's an image, in lowercase.
    pub generic_alt: &'static [&'static str],
}

const ENGLISH: Words = Words {
    vague_links: &["click here", "here", "click", "read more", "more", "learn more", "link", "this link", "this"],
    generic_alt: &["image", "img", "photo", "picture", "pic", "graphic"],
};

const GERMAN: Words = Words {
    vague_links: &[
        "hier klicken",
        "hier",
        "klicken",
        "klick hier",
        "mehr",
        "weiter",
        "weiterlesen",
        "mehr lesen",
        "mehr erfahren",
        "mehr infos",
        "link",
        "dieser link",
    ],
    generic_alt: &["bild", "foto", "grafik", "abbildung", "img"],
};

/// The words for a language tag like `de` or `de-AT`, if they're known.
pub fn for_language(lang: &str) -> Option<&'static Words> {
    let primary = lang.split(['-', '_']).next().unwrap_or_default().to_ascii_lowercase();
    match primary.as_str() {
        "en" => Some(&ENGLISH),
        "de" => Some(&GERMAN),
        _ => None,
    }
}
