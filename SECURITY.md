# Security

## Reporting a vulnerability

Please report security problems privately through GitHub's
[private vulnerability reporting](https://github.com/hypralang/hypra/security/advisories/new) rather than in a
public issue. Include a page and data that show the problem.

## Trust model

| Input | Trusted? | Why |
| --- | --- | --- |
| `.hypra` files | **yes** | written by the site's developers, like any other source code |
| data passed when rendering | **no** | comes from databases, users and requests |
| the Accept header and the request URL | **no** | sent by the visitor |

Hypra's job is to make sure untrusted data can't change what a page does. A page's author can still write
unsafe code on purpose, for example with `raw`; the compiler reports those places (see below).

## How data is written

Every place data can end up has its own rule. They're the same in the static build (`interp.rs`), the PHP
runtime (`php/runtime`) and the JavaScript runtime (`js/runtime.js`), and `tests/differential.rs` renders hundreds
of random pages with tricky data in all three to make sure (set `HYPRA_FUZZ_CASES` for more).

| Where the data goes | What happens |
| --- | --- |
| text, and attribute values | HTML-escaped: `&`, `<`, `>`, `"` |
| URL attributes (`href`, `src`, `action`, `formaction`, `poster`, `cite`, `data`, SVG's `xlink:href`, and the `to`, `from` and `by` of an SVG `set` or `animate` that changes a link) | a `javascript:`, `vbscript:` or `data:text/html` URL (ignoring case, whitespace and control characters) becomes `about:blank#blocked`; characters URLs can't contain are percent-encoded; then HTML-escaped |
| `script` and `style` | never: their text is written as it is, braces included; a `{name}` that names data is a warning (`SEC-005`) |
| JSON scripts (`application/json`, `*+json`, `importmap`) | a JSON string with `<`, `>` and `&` written as `\u003C`, `\u003E` and `\u0026`, so the data can't close the element |
| attributes that run code (`on*`, `x-*`, `@*`, `:*`, `v-on:*`, `ng-*`, `hx-on*`, `srcdoc`) or load it (`src` of `script`, `iframe` and `embed`, `data` of `object`, `href` of `link`, `content` of `meta` with `http-equiv`) | HTML-escaped, and a warning (`SEC-004`): escaping keeps the attribute closed but can't make data safe as code |
| `raw` | written as-is, with a warning (`SEC-002`) |
| the `confirm` question of an action | a JSON string inside the HTML-escaped `onsubmit` attribute |
| Markdown and JSON outputs | the same values as the HTML, unescaped for Markdown and JSON-encoded for JSON; they're served as `text/markdown` and `application/json`, never as HTML |

What's checked when a page is compiled:

- URLs written in the page that run code are errors (`SEC-003`), including `"javascript:{x}"` and URLs split by
  whitespace.
- An SVG animation that changes a link can't take data in its `values` list (`SEC-007`), since a list can't be
  checked as one URL.
- Inline event handlers like `onclick` are warnings (`SEC-001`).
- Quoted attribute names must be valid names (`ATTR-008`), so a name can't smuggle in another attribute.
- Action names only use letters, digits, dots, dashes and underscores (`SEM-014`), because they become URLs.
- Names that end up in generated PHP or JavaScript (form, field and action names, paths) are fixed text, never
  data, and every string in generated code is written as an escaped literal of that language.

## Forms and Laravel

- Hidden fields such as Laravel's CSRF token are only added to forms that post to this site. The check is an
  allowlist: a URL counts as this site only if it has no scheme and doesn't start with `//`, read the way browsers
  read it (tabs and newlines removed, leading spaces and control characters skipped, `\` treated as `/`). So
  `https://…`, `//…`, `/\evil.example` and `ht<tab>tps://…` never receive them, also when the URL comes from data.
  The Rust, PHP and JavaScript versions are tested against the same list of tricky URLs.
- A button's `formaction` must be a fixed path on this site, and `base(href: …)` must be fixed text (`SEC-006`):
  both could otherwise redirect a form, hidden fields included.
- Quoted attribute names are lowercased like HTML does, so `"HREF"` or `"ONCLICK"` get the same checks as `href`
  and `onclick`.
- Format negotiation only changes a response that is a rendered `.hypra` page; other responses are untouched.
  Every negotiated response carries `Vary: Accept`, so shared caches keep the formats apart.
- In strict mode (the default when `APP_DEBUG` is on), data that doesn't match the page throws a `DataError`
  naming the `.hypra` line. Turn `APP_DEBUG` off in production, as with any Laravel error.
- The compiler runs as a separate process with the view's path as an argument, never through a shell.

## Editors

- The VS Code extension runs the compiler for errors, formatting and the preview, and that can be a compiler the
  project installed (`node_modules/hypra`, `storage/hypra`). So it does this only in folders VS Code trusts; in
  others it only highlights.
- The preview shows the page with its scripts, like a browser would. The extension doesn't listen to the preview,
  so a script there can't do anything in VS Code.

## Limits

- **Templates are trusted.** A `.hypra` file from an untrusted source can do anything a PHP view can, starting
  with `raw`. Don't compile templates that visitors upload.
- **Data inside URLs isn't encoded per part.** `a("/search?q={query}")` percent-encodes characters URLs can't
  contain, but `&` or `=` in the data still start a new query parameter. Encode such values before rendering.
- **Attributes that run code are only warned about.** If a page passes data into `onclick` or Alpine's `x-data`,
  the data is escaped as an attribute but runs as JavaScript.
- **Compiling is bounded**, not cheap: nesting is limited to 256 levels and expanded pages to 200,000 elements,
  but a page near those limits still takes noticeable time and memory to compile. Compile trusted templates only.
