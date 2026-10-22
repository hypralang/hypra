//! Facts about HTML that the checker and lowering share.

/// Every element of the HTML Living Standard.
#[rustfmt::skip]
pub const TAGS: &[&str] = &[
    "a", "abbr", "address", "area", "article", "aside", "audio", "b", "base", "bdi", "bdo", "blockquote", "body",
    "br", "button", "canvas", "caption", "cite", "code", "col", "colgroup", "data", "datalist", "dd", "del",
    "details", "dfn", "dialog", "div", "dl", "dt", "em", "embed", "fieldset", "figcaption", "figure", "footer",
    "form", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header", "hgroup", "hr", "html", "i", "iframe", "img",
    "input", "ins", "kbd", "label", "legend", "li", "link", "main", "map", "mark", "menu", "meta", "meter", "nav",
    "noscript", "object", "ol", "optgroup", "option", "output", "p", "picture", "pre", "progress", "q", "rp", "rt",
    "ruby", "s", "samp", "script", "search", "section", "select", "slot", "small", "source", "span", "strong",
    "style", "sub", "summary", "sup", "table", "tbody", "td", "template", "textarea", "tfoot", "th", "thead", "time",
    "title", "tr", "track", "u", "ul", "var", "video", "wbr",
    // SVG and MathML, written inline; what's inside them follows their own rules
    "svg", "math",
];

/// Elements without content or closing tag.
#[rustfmt::skip]
pub const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr",
];

/// Elements that sit inside a line of text.
#[rustfmt::skip]
pub const INLINE: &[&str] = &[
    "a", "abbr", "b", "bdi", "bdo", "br", "button", "cite", "code", "data", "del", "dfn", "em", "i", "img", "input",
    "ins", "kbd", "label", "mark", "q", "s", "samp", "select", "small", "span", "strong", "sub", "sup", "textarea",
    "time", "u", "var", "wbr",
];

/// Elements allowed inside `head { ... }`.
pub const HEAD: &[&str] = &["base", "link", "meta", "noscript", "script", "style", "template"];

/// Attributes Hypra accepts by name on HTML elements. Names with a dash (`data-*`, `aria-*`, `hx-*`),
/// event handlers (`on*`) and quoted names (`"x-on:click"`) are always accepted.
#[rustfmt::skip]
pub const ATTRIBUTES: &[&str] = &[
    // global
    "accesskey", "autocapitalize", "autofocus", "class", "contenteditable", "dir", "draggable", "enterkeyhint",
    "hidden", "id", "inert", "inputmode", "is", "itemid", "itemprop", "itemref", "itemscope", "itemtype", "lang",
    "nonce", "part", "popover", "role", "slot", "spellcheck", "style", "tabindex", "title", "translate",
    // element specific
    "abbr", "accept", "action", "allow", "allowfullscreen", "alt", "as", "async", "autocomplete", "autoplay",
    "blocking", "charset", "checked", "cite", "cols", "colspan", "content", "controls", "coords", "crossorigin",
    "datetime", "decoding", "default", "defer", "dirname", "disabled", "download", "enctype", "fetchpriority", "for",
    "form", "formaction", "formmethod", "formnovalidate", "formtarget", "headers", "height", "high", "href",
    "hreflang", "integrity", "ismap", "kind", "label", "list", "loading", "loop", "low", "max", "maxlength", "media",
    "method", "min", "minlength", "multiple", "muted", "name", "novalidate", "open", "optimum", "pattern", "ping",
    "placeholder", "playsinline", "popovertarget", "popovertargetaction", "poster", "preload", "property",
    "readonly", "referrerpolicy", "rel", "required", "reversed", "rows", "rowspan", "sandbox", "scope", "selected",
    "shape", "size", "sizes", "span", "src", "srcdoc", "srclang", "srcset", "start", "step", "target", "type",
    "usemap", "value", "width", "wrap",
    // newer
    "autocorrect", "capture", "closedby", "color", "command", "commandfor", "data", "exportparts", "imagesizes",
    "imagesrcset", "nomodule", "shadowrootclonable", "shadowrootdelegatesfocus", "shadowrootmode",
    "shadowrootserializable", "writingsuggestions",
];

/// Attributes holding a URL, where a `javascript:` URL would run code.
pub const URL_ATTRIBUTES: &[&str] = &["action", "cite", "data", "formaction", "href", "poster", "src", "xlink:href"];

/// Attributes that are present or absent rather than holding a value.
#[rustfmt::skip]
pub const BOOLEAN_ATTRIBUTES: &[&str] = &[
    "allowfullscreen", "async", "autofocus", "autoplay", "checked", "controls", "default", "defer", "disabled",
    "formnovalidate", "hidden", "inert", "ismap", "itemscope", "loop", "multiple", "muted", "novalidate", "open",
    "playsinline", "readonly", "required", "reversed", "selected",
];

pub fn is_tag(name: &str) -> bool {
    TAGS.contains(&name)
}

pub fn is_void(name: &str) -> bool {
    VOID.contains(&name)
}

pub fn is_inline(name: &str) -> bool {
    INLINE.contains(&name)
}

/// A URL on this site, the only kind that receives the page's hidden form fields such as a CSRF token.
/// Browsers drop tabs and newlines, skip leading spaces and control characters and read `\` as `/`, so this does
/// too. Then only a URL without a scheme that doesn't start with `//` is on this site.
pub fn is_same_site(url: &str) -> bool {
    let cleaned: String = url.chars().filter(|c| !matches!(c, '\t' | '\n' | '\r')).collect();
    let cleaned = cleaned.trim_start_matches(|c: char| c <= ' ').replace('\\', "/");
    if cleaned.starts_with("//") {
        return false;
    }
    // a scheme ends with the first `:` that comes before any `/`, `?` or `#`
    cleaned.find([':', '/', '?', '#']).is_none_or(|i| cleaned.as_bytes()[i] != b':')
}

/// Attributes that are present or absent. Custom elements use presence for their own flags too,
/// but `aria-*` and `data-*` always hold text, so `aria-hidden: true` stays `aria-hidden="true"`.
pub fn is_presence_attribute(tag: &str, name: &str) -> bool {
    BOOLEAN_ATTRIBUTES.contains(&name)
        || (is_custom_element(tag) && !name.starts_with("aria-") && !name.starts_with("data-"))
}

/// Attributes whose value is the text `true` or `false`, like `aria-expanded="false"`. Any other attribute set to
/// `true` is written bare, as HTML means it (`download`, `popover`), and set to `false` is left out.
pub fn takes_true_false_text(name: &str) -> bool {
    ["aria-", "data-", "hx-", "x-", "v-", "@", ":", "on"].iter().any(|prefix| name.starts_with(prefix))
        || matches!(name, "contenteditable" | "draggable" | "spellcheck" | "autocorrect" | "writingsuggestions")
}

/// SVG and MathML: inside them any element and attribute is written as-is.
pub fn is_foreign(name: &str) -> bool {
    matches!(name, "svg" | "math")
}

/// Custom elements (web components) are written with a dash: `model-viewer`.
pub fn is_custom_element(name: &str) -> bool {
    name.contains('-') && name.starts_with(|c: char| c.is_ascii_lowercase())
}

/// The attribute an element's unnamed first argument sets: `a("/shoes")` is `href`.
pub fn main_attribute(tag: &str) -> Option<&'static str> {
    match tag {
        "a" => Some("href"),
        "form" => Some("action"),
        "audio" | "embed" | "iframe" | "img" | "script" | "source" | "track" | "video" => Some("src"),
        _ => None,
    }
}
