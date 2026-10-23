<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/logo-dark.svg">
    <img src=".github/logo.svg" alt="Hypra" height="56">
  </picture>
</p>

<p align="center">
  <b>Pages that compile.</b><br>
  A markup language whose compiler turns one file into accessible HTML for browsers,<br>
  Markdown for AI agents and typed tools, and refuses pages with mistakes.
</p>

<p align="center">
  <a href="https://hypralang.dev">Website</a> ·
  <a href="https://hypralang.dev/docs/">Docs</a> ·
  <a href="https://hypralang.dev/guides/">Guides</a> ·
  <a href="php/laravel">Laravel</a> ·
  <a href="editors/vscode">VS Code</a> ·
  <a href="https://github.com/hypralang/hypra/releases">Download</a>
</p>

```hypra
page(title: "{product.name} – Running Shoes", description: "{product.name}: a light running shoe for long distances.") {
  props(product: Product)

  img(product.photo, alt: "{product.name}, side view")
  h1 "{product.name}"
  action("cart.add", id: product.id) "Add to cart"

  form("/reviews", name: "review", description: "Leave a review") {
    field("stars", type: Int(1..5), label: "Rating")
    field("email", type: Email, label: "Your email")
    button "Send review"
  }
}
```

> Hypra is pre-1.0. The language and its output may still change.

## What works today

| Use it with | What you get |
| --- | --- |
| **Any site or language** | `hypra build` turns a page and its data (JSON) into HTML, Markdown, JSON and WebMCP files |
| **PHP 8.2+** | pages compiled to PHP render at request time with [`hypra/runtime`](php/runtime) |
| **Laravel 11, 12, 13** | `.hypra` views, Markdown and JSON for agents on the same URL, CSRF, actions, `hypra:install`, `hypra:compile` |
| **CSS and JavaScript** | anything: Tailwind, Alpine.js, htmx, web components, React or Vue islands |
| **Editors** | VS Code and Cursor: highlighting, errors as you type, go to definition, references, rename, hover, formatting; other editors through `hypra lsp` |
| **Compiler** | prebuilt for Linux, macOS and Windows on x64 and ARM, or `cargo install hypra` |

Other server languages, like Node, Python, Go or Ruby, can use the static build or call `hypra build`.

## Why

HTML accepts anything. That made the web forgiving, but it also means mistakes ship silently:

```html
<div class="btn" onclick="buy(42)">Buy</div>   <!-- not a button: no keyboard, no screen reader -->
<img src="shoe.jpg">                           <!-- no alt text -->
<input name="email">                           <!-- no label, validation written again on the server -->
<title>Shop</title>                            <!-- same title on every page, no description -->
```

Hypra moves these checks to compile time, the way TypeScript does for JavaScript.
It still outputs plain HTML, so every browser works with it today.

## The language in five rules

1. **Every element has one shape:** `name(attributes) "text" { children }`. Every part is optional.
2. **Attributes are `name: value`.** The first one may be unnamed when an element has a main attribute:
   `a("/shoes")`, `img(product.photo, ...)`, `form("/reviews", ...)`.
3. **A value says what it is:** `"text with {data}"`, `42`, `true`, `product.name` (data, lowercase),
   `Shop` (a layout, component or type, capitalized).
4. **Element names are HTML tag names.** Hypra adds only what HTML lacks: `page`, `props`, `action`, `field`,
   `raw` and `allow`.
5. **Settings live on the element they configure:** `page(title: ..., layout: Shop, robots: "noindex")`.

Besides elements there are five statements: `use "file.hypra"`, `component Name(...) { }`, `layout Name(...) { }`,
`for item in list { }` and `if [not] value { } else { }`.

The complete reference is [`docs/spec.md`](docs/spec.md). It's short enough to give to a language model, and the
[benchmark](bench) measures how well models write Hypra from it.

## One source, four outputs

```console
$ hypra build examples/product.hypra --data examples/product.json
✔ dist/product.html
✔ dist/product.md
✔ dist/product.json
✔ dist/product.webmcp.json
0 errors, 0 warnings
```

| File | For | Contains |
| --- | --- | --- |
| `product.html` | browsers, search engines | semantic HTML, meta and Open Graph tags, canonical link, real `<form>`s |
| `product.md` | LLMs and AI crawlers | the content as Markdown, plus a list of every action on the page |
| `product.json` | apps and scripts | page metadata and typed actions |
| `product.webmcp.json` | AI agents | [WebMCP](https://github.com/webmachinelearning/webmcp) tool definitions with JSON Schemas |

The action from the example, as each reader sees it:

```html
<form method="post" action="/_a/cart.add" toolname="cart.add" tooldescription="Add to cart">
  <input type="hidden" name="id" value="42">
  <button type="submit">Add to cart</button>
</form>
```

```markdown
- `cart.add(id: 42)`: Add to cart → POST /_a/cart.add
- `review(stars: integer 1..5, email: email)`: Leave a review → POST /reviews
```

## Errors you can act on

Every error has a code, a location and a fix, for people and for AI coding agents alike:

```console
$ hypra check examples/broken.hypra
…
error[A11Y-001]: image has no alt text
 --> examples/broken.hypra:5:9
  |
5 |     img("/img/shoe.jpg")
  |         ^^^^^^^^^^^^^^^
  = help: add alt: "..." or decorative: true

error[SEM-001]: unknown element `buton`
 --> examples/broken.hypra:8:5
  |
8 |     buton "Buy"
  |     ^^^^^
  = help: did you mean `button`?
…
6 errors, 0 warnings
```

Add `--format json` to get the same list as JSON. Checks on text that comes from data, like a description from
a database, run when a page is built with that data.

## Works with everything

Strict doesn't mean limited. Anything HTML can do, Hypra can do:

```hypra
// Tailwind, Bootstrap or any CSS: classes are attributes
div(class: "rounded-xl p-6 shadow md:flex") { ... }

// any attribute with a dash passes through: data-*, aria-*, htmx, Alpine
button(hx-post: "/cart", hx-target: "#cart", aria-controls: "cart") "Add"
div(x-data: "{ open: false }") { ... }

// any name at all, quoted
button("x-on:click": "open = !open", "@keyup.escape": "open = false") "Menu"

// web components and custom elements
model-viewer(src: "/shoe.glb", ar: true, camera-controls: true)

// stylesheets, scripts and any meta tag
head {
  link(rel: "stylesheet", href: "/app.css")
  script(src: "/app.js", type: "module")
  meta(name: "theme-color", content: "#3b5bdb")
}

// React, Vue or Svelte: mount them on an element from your own script
div(id: "chart", data-props: "{product.sales}")

// and as a last resort, HTML as-is
raw "<marquee>Sale</marquee>"
```

Rules can be switched off for one element, with a reason that shows up in review:

```hypra
allow("SEO-003") "the hero splits its headline into two h1"
h1 "Run faster."
```

## Text

Text after an element is always a string, and `{name}` reads data. Braces around anything else, like the
JavaScript in `x-data: "{ open: false }"`, stay text. Mix text and inline elements in a block:

```hypra
p "{product.summary}"
p { "Weighs only " b "220 g" ", see " a("/specs") "the specs" "." }
```

`{{` and `}}` write literal braces.

## Components

Components are capitalized, take named parameters with optional defaults, and render the content they're
called with wherever their body says `slot`. They only see their own parameters, never the page data:

```hypra
component Card(title: Text, featured: Bool = false) {
  article(class: "card") {
    h2 "{title}"
    if featured { p "Staff pick" }
    slot
  }
}

page(title: "Shoes", description: "All our running shoes, from light racers to trail shoes.") {
  props(products: List)
  h1 "Shoes"
  for item in products {
    Card(title: item.name, featured: item.featured) { p "{item.summary}" }
  }
}
```

## Layouts and imports

A layout is the frame around a page. Its settings are defaults that every page can override:

```hypra
// layouts/shop.hypra
layout Shop(lang: "en", title_suffix: " | Shop", site_name: "Shop") {
  head { link(rel: "stylesheet", href: "/app.css") }
  header { nav(aria-label: "Main") { a("/") "Home"  a("/shoes") "Shoes" } }
  main { slot }
  footer { p "© 2026 Shop" }
}
```

```hypra
// product.hypra
use "layouts/shop.hypra"
use "components/review-card.hypra"

page(layout: Shop, title: "Runner X", description: "...") {   // <title>Runner X | Shop</title>
  ...
}
```

Imports are relative to the importing file, and errors in imported files point at the file they're in.

## Rendering on a server

For dynamic pages, Hypra compiles to code that receives its data at request time. The PHP target needs no
Rust on the server; the generated file uses the small [`hypra/runtime`](php/runtime) package:

```console
$ hypra build examples/product.hypra --target php
✔ dist/product.php
✔ dist/product.sources.json
0 errors, 0 warnings
```

```php
$page = require 'dist/product.php';
$output = $page->render(['product' => $product]);

$output->html;      // the page
$output->markdown;  // for LLMs
$output->json;      // page metadata and actions
$output->webmcp;    // tool definitions for AI agents
```

The PHP output is tested to be identical to the static build. With `render($data, strict: true)`, data that
doesn't match the page throws a `DataError` that points at the `.hypra` source.

### Laravel

With [`hypra/laravel`](php/laravel), `.hypra` files are views. One URL serves the page as HTML to browsers,
Markdown to LLMs and JSON to apps, depending on the `Accept` header, and every POST form gets the CSRF token:

```php
Route::get('/shoes/{product}', fn (Product $product) => view('products.show', ['product' => $product]));
Route::hypra('cart.add', [CartController::class, 'add']);   // action("cart.add", ...) "Add to cart"
```

No Rust needed: `php artisan hypra:install` downloads the compiler for your machine.

## Getting started

Download the `hypra` compiler for Linux, macOS or Windows from the
[releases](https://github.com/hypralang/hypra/releases); each release lists the SHA-256 checksums in
`SHA256SUMS`. With Rust installed, `cargo install hypra` works too, or build it from source:

```console
$ git clone https://github.com/hypralang/hypra
$ cd hypra
$ cargo build --release
$ ./target/release/hypra build examples/product.hypra --data examples/product.json
```

```text
hypra build <file.hypra> [--data data.json] [--out dir]
hypra build <file.hypra> --target php [--out dir]
hypra check <file.hypra> [--data data.json] [--format json]
hypra fmt [--check] <file.hypra>...
hypra lsp
```

For VS Code and Cursor, [editors/vscode](editors/vscode) adds highlighting, errors as you type, Go to
Definition, Find All References, Rename, hover help and Format Document. They come from `hypra lsp`, a language server, so other editors get
them too: point the editor's LSP support at `hypra lsp` for `.hypra` files. In Neovim:

```lua
vim.filetype.add({ extension = { hypra = "hypra" } })
vim.lsp.config("hypra", { cmd = { "hypra", "lsp" }, filetypes = { "hypra" }, root_markers = { ".git" } })
vim.lsp.enable("hypra")
```

`hypra fmt` gives every file the same layout: indentation, spacing around punctuation, at most one blank line.
It only changes whitespace, keeps comments and line breaks, and `--check` lists the files that need it.

## Checks

Errors stop the build; warnings don't. The [spec](docs/spec.md#errors) lists every code.

| Codes | Rule |
| --- | --- |
| `SEO-001`…`006` | pages have a title, a description of 50 to 160 characters (wide characters count twice), one `h1` and an absolute canonical URL |
| `A11Y-001`…`013` | alt text that describes images, captions, link and button text, links that say where they go, heading levels in order, iframe titles, no positive `tabindex`, muted autoplay, roles for named divs |
| `FORM-001`…`006` | forms have a name, a button and a known method; fields have a type and a label |
| `ATTR-001`…`008` | unknown, duplicate, misplaced, mistyped or malformed attributes, with "did you mean" hints |
| `SEM-001`…`014` | unknown or misplaced elements, duplicate ids, language codes, action names |
| `SEC-001`…`007` | URLs that run code, data inside scripts, styles and attributes that run code, `raw` with data, forms sent to other sites |
| `TYPE-001`…`005` | undeclared or missing props, unknown data fields, unknown or misused field types |
| `COMP-001`…`010` | unknown components, missing or unknown arguments, duplicate names, recursion, content without a `slot`, pages that expand without bound |
| `LAYOUT-001`…`005` | unknown layouts, layouts without a `slot`, layouts inside themselves, duplicate or lowercase names |
| `IMPORT-001`, `LEX-001`…`003`, `PARSE-001`…`002` | missing files, syntax errors and pages nested too deeply |
| `ALLOW-001`…`003` | `allow` needs a known rule code and a reason |

## Contributing

Issues and pull requests are welcome. [ROADMAP.md](ROADMAP.md) lists what's planned and
[CHANGELOG.md](CHANGELOG.md) what changed in each release. Before opening a pull request:

```console
$ cargo fmt
$ cargo clippy --all-targets -- -D warnings
$ cargo test
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project
by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or
conditions.
