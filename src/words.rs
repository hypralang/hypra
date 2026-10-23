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

const FRENCH: Words = Words {
    vague_links: &[
        "cliquez ici",
        "cliquer ici",
        "ici",
        "plus",
        "en savoir plus",
        "lire la suite",
        "lire plus",
        "voir plus",
        "suite",
        "lien",
        "ce lien",
    ],
    generic_alt: &["image", "photo", "img", "illustration", "graphique"],
};

const SPANISH: Words = Words {
    vague_links: &[
        "haz clic aquí",
        "haga clic aquí",
        "clic aquí",
        "pulsa aquí",
        "aquí",
        "más",
        "leer más",
        "ver más",
        "saber más",
        "más información",
        "enlace",
        "este enlace",
    ],
    generic_alt: &["imagen", "foto", "fotografía", "img", "gráfico"],
};

const PORTUGUESE: Words = Words {
    vague_links: &[
        "clique aqui",
        "aqui",
        "mais",
        "leia mais",
        "ler mais",
        "saiba mais",
        "ver mais",
        "link",
        "este link",
    ],
    generic_alt: &["imagem", "foto", "fotografia", "img", "gráfico"],
};

const ITALIAN: Words = Words {
    vague_links: &[
        "clicca qui",
        "clicca",
        "qui",
        "di più",
        "leggi di più",
        "leggi tutto",
        "scopri di più",
        "altro",
        "link",
        "questo link",
    ],
    generic_alt: &["immagine", "foto", "fotografia", "img", "grafica"],
};

const DUTCH: Words = Words {
    vague_links: &["klik hier", "hier", "klik", "meer", "lees meer", "meer lezen", "meer info", "link", "deze link"],
    generic_alt: &["afbeelding", "foto", "plaatje", "img", "beeld"],
};

/// The words for a language tag like `de` or `de-AT`, if they're known.
pub fn for_language(lang: &str) -> Option<&'static Words> {
    let primary = lang.split(['-', '_']).next().unwrap_or_default().to_ascii_lowercase();
    match primary.as_str() {
        "en" => Some(&ENGLISH),
        "de" => Some(&GERMAN),
        "fr" => Some(&FRENCH),
        "es" => Some(&SPANISH),
        "pt" => Some(&PORTUGUESE),
        "it" => Some(&ITALIAN),
        "nl" => Some(&DUTCH),
        _ => None,
    }
}
