# The Hypra language

Hypra is a strict markup language that compiles to HTML, plus Markdown, JSON and WebMCP tools. This document
is the complete reference, written so that a person or a language model can write correct Hypra from it alone.

## A complete page

```hypra
// product.hypra: comments start with //
use "layouts/shop.hypra"   // imports the Shop layout from another file (see `layout` below)

page(layout: Shop, title: "{product.name} – Running Shoes", description: "{product.name}: a light running shoe for long distances.") {
  props(product: Product)

  img(product.photo, alt: "{product.name}, side view", width: 600, height: 400)
  h1 "{product.name}"
  p { "Only " b "{product.weight} g" " per shoe." }
  ul {
    for feature in product.features { li "{feature}" }
  }
  action("cart.add", id: product.id) "Add to cart"

  form("/reviews", name: "review", description: "Leave a review") {
    field("stars", type: Int(1..5), label: "Rating")
    field("body", type: Text(max: 2000), label: "Review", optional: true)
    button "Send review"
  }
}
```

## Rules

1. **Every element has one shape:** `name(attributes) "text" { children }`. Each part is optional, and they
   combine freely: `button(hx-get: "/more", hx-target: "#list") "Load more"`. The text and the opening `{` must
   be on the same line as the name; the block itself may span as many lines as you like.
2. **Attributes are `name: value`, separated by commas.** Some elements take their main attribute first, without
   a name (writing the name works too, e.g. `action(name: "cart.add")`): `a("/shoes")`,
   `img("/photo.jpg", alt: "...")`, `form("/reviews", ...)`, `field("email", ...)`, `action("cart.add", ...)`,
   `script("/app.js")`, `video("/clip.mp4", ...)`. It can be data too:
   `a(product.url) "View"`, `img(photo.src, alt: "{photo.caption}")`.
3. **A value says what it is:**
   - `"text"` is text. `{name}` inside it reads data: `"Hello {user.name}"`.
   - `42`, `3.5`, `-1` are numbers; `true`, `false` are booleans.
   - `product.name` (lowercase, no quotes) is data.
   - `Shop`, `Email`, `Product` (capitalized) are layouts, components and types.
4. **Element names are HTML tag names:** `div`, `p`, `h1`…`h6`, `ul`, `li`, `a`, `img`, `section`, `table`, …
   Hypra adds only `page`, `props`, `action`, `field`, `raw` and `allow`.
5. **Settings belong to the element they configure:** `page(title: ..., layout: Shop, robots: "noindex")`.

## Text

Text after an element is always a quoted string: `h1 "Welcome"`, `p "{post.body}"`. Never write data without
quotes after an element (`h1 post.title` is wrong; write `h1 "{post.title}"`).

- `{name}` reads data only when written without spaces and with a valid name: `{user.name}`, `{item}`.
  Braces around anything else are text, so JavaScript like `"{ open: false }"` works as is.
- `{name}` works in any quoted value, text or attribute: `alt: "Photo of {product.name}"`.
- `{{` and `}}` write literal braces.
- Write `&`, `<`, `©`, `ü` and any other character as is; Hypra escapes what HTML needs.
- A backslash starts an escape: `\"` (quote), `\\` (backslash), `\n` (new line), `\t` (tab). Any other is an error,
  so a Windows path is written `"C:\\files"`.
- Text over several lines goes between `"""` lines. The indentation of the closing `"""` is removed from every
  line, and the text is taken as written: quotes and backslashes need no escaping, so code keeps its own escapes
  like `"\n"`. `{name}` still reads data, except in scripts and styles:

  ```hypra
  pre """
    fn main() {
        println!("hi");
    }
    """
  ```
- Numbers are written exactly as in the source: `lng: 13.40` stays `13.40`.
- To mix text with inline elements, use a block of strings and elements:
  `p { "Read " a("/terms") "the terms" " first." }`. `if` and `for` work inside such a line too:
  `p { "Hello" if user.vip { b " (VIP)" } "!" }`.
- Several elements may share a line: `nav { a("/") "Home"  a("/shop") "Shop" }`.

## The page

Every file you compile has exactly one `page` at the top level. Components and layouts can be defined in the
same file (before or after the page) or imported from other files with `use`.

```hypra
page(title: "About us", description: "Who we are: a small bakery that bakes bread by hand every night.") {
  h1 "About us"
}
```

| Attribute | Meaning |
| --- | --- |
| `title` | **required.** The `<title>`; keep it under 60 characters wide |
| `description` | **required**, unless the layout sets one or `robots` has `noindex`. For search results and link previews; 50 to 160 characters wide |
| `layout` | a layout name, e.g. `layout: Shop` |
| `canonical` | the page's full canonical URL, starting with `https://` |
| `image` | the image for link previews |
| `robots` | any robots value, e.g. `robots: "noindex"` or `robots: "noindex, nofollow"` |
| `lang` | the page language as a code, e.g. `lang: "de"` or `"pt-BR"` (default `"en"`), never a name like `"German"` |

Any other HTML attribute of the page goes on `<html>`: `page(title: "...", dir: "rtl", class: "dark")`. Hypra
writes `html`, `body` and `title` itself; to give `<body>` attributes, write `body(class: "...")` without content
directly inside the page or layout. The page's `body(...)` wins over the layout's for the same attribute.

### Data: `props`

A page declares the data it uses. Using undeclared data is an error.

```hypra
page(title: "{post.title}", description: "{post.summary}") {
  props(post: Post, user: User)
  h1 "{post.title}"
}
```

Type names are capitalized and document the data; any capitalized word works (`Post`, `User`, `List` for a list).
They aren't checked against the data.

### `head`

`head` goes directly inside a `page`, `layout` or `component` and takes `meta`, `link`, `script`, `style`, `base`,
`noscript` and `template`, and can stand anywhere in the page (usually first). Hypra already writes the title,
description, canonical, Open Graph, Twitter card and viewport tags; a `meta` with the same `name` or `property`
(or a canonical `link`) in `head` replaces Hypra's. A page loads the libraries its attributes rely on, such
as htmx or Alpine.js, itself; local paths and full URLs both work. The same goes for CSS: class names like
Tailwind's don't load anything, so link the stylesheet (or the framework's script) in `head`.

```hypra
head {
  link(rel: "stylesheet", href: "/app.css")
  link(rel: "icon", href: "/favicon.svg")
  script("/app.js", type: "module")
  script("https://unpkg.com/htmx.org@2.0.3", defer: true)
  meta(name: "theme-color", content: "#3b5bdb")
}
```

## HTML elements and attributes

Any HTML element works with its normal attributes: `div(class: "card", id: "main")`, `input(type: "search",
name: "q")`, `td(colspan: 2) "Total"`.

- `true` writes an attribute bare and `false` leaves it out: `hidden: true`, `download: true`, `popover: true`.
  Attributes whose value is the text `"true"` or `"false"` keep it: `aria-*`, `data-*`, `hx-*`, Alpine and Vue
  attributes, event handlers, `contenteditable`, `draggable` and `spellcheck`, so `aria-expanded: false` becomes
  `aria-expanded="false"`. Data works too: `checked: user.admin` writes `checked` when the value is true, for
  HTML's boolean attributes (`hidden`, `disabled`, `checked`, …); other attributes get the value as text.
- An object or list from data is written as JSON: `data-props: island.props`.
- Attributes with a dash pass through: `data-id: "7"`, `aria-label: "Close"`, `hx-post: "/like"`,
  `x-data: "{ open: false }"`.
- Any other name can be quoted: `"@click": "open = !open"`, `"x-on:keyup.escape": "open = false"`. A quoted name
  can't contain spaces, quotes, `=`, `>` or `/`.
- An attribute Hypra doesn't know is written as-is with a warning, since HTML keeps gaining attributes; quote its
  name to say it's meant. A near miss of a Hypra setting, like `decortive: true` on `img`, is an error.
- URLs from data that would run code (`javascript:`, `vbscript:`, `data:text/html`) are replaced with
  `about:blank#blocked`. Written in the page, they're an error.
- `script` and `style` take their code as text and write it exactly as it is: braces in JavaScript and CSS are
  never data, so `import {createRoot} from "react"` works. Pass data to a script in a `data-*` attribute, or in a
  JSON script, where `{name}` is data written as a JSON string (so don't put quotes around it):

  ```hypra
  script(type: "application/ld+json") """
    {"@context": "https://schema.org", "@type": "Product", "name": {product.name}}
    """
  ```
- Custom elements (with a dash in the name) take any attribute: `model-viewer(src: "/shoe.glb", ar: true)`.
- `svg` and `math` work inline. Inside them, any element and attribute is written as-is, following SVG and
  MathML: `svg(viewBox: "0 0 24 24") { path(d: "M12 2L2 22h20z") }`.
- Inside an HTML `template`, `slot` is the web components element: `template(shadowrootmode: "open") { slot }`.
- Void elements (`img`, `input`, `br`, `hr`, `meta`, `link`, …) have no text and no children.

Elements with Hypra rules:

| Element | Rule |
| --- | --- |
| `img(src, alt: "...")` | needs `alt`, or `decorative: true` for images that carry no meaning. `priority: true` loads it first |
| `a(href) "text"` | needs text |
| `button "text"` | needs text, or `aria-label` for an icon button |
| `video(src, captions: "file.vtt")` | needs captions, unless it's `muted: true`; `source` and `track` elements work inside it, and `controls: false` hides the controls |
| `h1` | exactly one per page, counted as the page renders: a component with an `h1` used twice, or an `h1` in a loop, is too many |

## Forms

```hypra
form("/signup", name: "signup", description: "Create an account") {
  field("name", type: String(1..80), label: "Full name")
  field("email", type: Email, label: "Email")
  field("password", type: Password(min: 12), label: "Password")
  field("plan", type: Choice("free", "pro"), label: "Plan", default: "free")
  field("bio", type: Text(max: 500), label: "About you", optional: true)
  field("terms", type: Accept, label: "I accept the terms")
  button "Sign up"
}
```

- `form(url, name: "...")`: the `name` makes the form a tool agents can call, so a form with fields should have
  one. Also: `description`, and `method`: `"post"` by default; `"get"`, `"put"`, `"patch"` and `"delete"` also
  work, and `"dialog"` closes the surrounding `dialog` without sending anything, so it's never a tool.
- A form needs a `button` to submit it.
- `field("name", type: ..., label: "...")` goes anywhere inside the form, also inside `fieldset` or a `div`, but
  not inside `for` or `if`: fields are the form's fixed inputs. `type` and `label` are required. Fields are
  required to fill in unless they have `optional: true`. Also: `default: "..."`, and any input attribute such as
  `placeholder: "..."`. Plain `input`, `select` and `textarea` work too, but agents only see fields.
- Forms can't be nested, and an `action` is a form of its own, so it can't be inside a form or a line of text.

| Field type | Input |
| --- | --- |
| `String`, `String(1..80)` | one line of text, with optional length range |
| `Text`, `Text(max: 2000)` | several lines |
| `Password`, `Password(min: 12)` | a password, with optional length range; it has no `default` |
| `Email`, `Url`, `Tel` | an email address, a URL, a phone number; also with a length range |
| `Int`, `Int(1..5)` | a whole number, with optional range |
| `Number`, `Number(min: 0, max: 9.99)` | any number, with optional range |
| `Date`, `Time` | a date or time, with optional `min: "2025-01-01"` and `max: ...` |
| `Choice("s", "m", "l")` | one of the listed options |
| `Choice(de: "Germany", at: "Austria")` | one of the options, each with its own text; the form sends `de` or `at` |
| `Accept` | a checkbox that must be ticked |
| `File` | a file upload; the form is sent as `multipart/form-data` |

A range is `1..80`, `1..` or `..80`, or written out as `min: 1, max: 80`. A field's `default` can come from data,
e.g. `default: user.country` selects the user's country.

## Actions

An action is a button that does one thing on the server, like adding to a cart:

```hypra
action("cart.add", id: product.id) "Add to cart"
action("cart.remove", id: item.id, confirm: "Remove this item?") "Remove"
action("cart.checkout") "Check out"
```

The first argument is the action's name, written in quotes with letters, digits, dots, dashes and underscores.
Attributes that style the button or hook up a script go on the button: `class`, `style`, `title`, `disabled`,
`form`, and `aria-*`, `data-*`, `hx-*`, `x-*`, `@*` and `:*`. Every other attribute except `confirm` is sent with
the action. The text is the button label and is required. Actions post to `/_a/<name>`. Hidden fields such as a
CSRF token are only added to forms that post to this site.

An action shown in a loop is one tool for agents: values that differ between the copies become its choices.
`for item in cart.items { action("cart.remove", id: item.id) "Remove" }` is the tool `cart.remove(id: 1|2|3)`.

## Statements

### `use`

Imports components and layouts from another file, relative to this one. `use` lines go at the top. A file that
holds only components and layouts has no `page`; `hypra check` checks it on its own, and pages build it.

```hypra
use "components/card.hypra"
```

### `component`

```hypra
component Card(title: Text, featured: Bool = false) {
  article(class: "card") {
    h3 "{title}"
    if featured { p "Staff pick" }
    slot
  }
}
```

- Names are capitalized. Parameters are `name: Type`, optionally with a default `= value`.
- A component sees only its parameters, never the page's data. Pass what it needs.
- Components are expanded into the page, so a component can't use itself, directly or through another one.
  Render nested data, like a menu tree, with `for` loops instead.
- A component that renders one HTML element passes HTML attributes from the call on to it, and joins classes:
  `Card(title: "Shoes", class: "mt-4", id: "shoes")` renders `<article class="card mt-4" id="shoes">`.
- A component's `head` goes into the head of every page that uses it, once however often it's used, so a
  component can bring its own script or stylesheet: `component Map(...) { head { script("/map.js") } ... }`.
- `slot` renders the content the component was called with. Text right after a call is that content too:
  `Badge(kind: "new") "New"` is `Badge(kind: "new") { "New" }`.
- A component can have named slots besides the default one. The call fills them with `slot("name") { ... }`; a
  named slot the call leaves out shows its own content instead:

  ```hypra
  component Panel(title: Text) {
    section {
      header { h2 "{title}"  slot("actions") { a("/help") "Help" } }
      slot
    }
  }

  Panel(title: "Orders") {
    slot("actions") { a("/orders/new") "New order" }
    p "Your recent orders"
  }
  ```
- Call it like an element with named arguments: `Card(title: post.title, featured: true) { p "{post.summary}" }`.

### `layout`

The frame around a page. Its settings are defaults every page can override.

```hypra
layout Shop(lang: "en", title_suffix: " | Shop", site_name: "Shop") {
  head { link(rel: "stylesheet", href: "/app.css") }
  header { nav(aria-label: "Main") { a("/") "Home"  a("/shoes") "Shoes" } }
  main { slot }
  footer { p "© 2026 Shop" }
}
```

Settings: `lang`, `title_suffix`, `site_name`, `description`, `image`, `robots`, and `layout`. A layout must
contain `slot`, where the page's content goes. A page uses it with `page(layout: Shop, ...)`.

A layout can sit inside another one with its own `layout` setting. The page goes into the inner layout's slot,
which goes into the outer one's. Settings come from the page first, then from the innermost layout that has them;
heads and `body(...)` attributes of all of them are kept:

```hypra
layout Docs(layout: Shop, title_suffix: " · Docs") {
  div(class: "docs") {
    aside { a("/docs/install") "Install" }
    article { slot }
  }
}
```

### `for`

```hypra
for post in posts { article { h2 "{post.title}" } }
for size in ["S", "M", "L"] { option "{size}" }
```

`for` repeats its body for every item of a list, from data or written in brackets. Inside the body, the name
before `in` is the current item.

A list in brackets holds fixed values: text, numbers, `true` and `false`. Besides `for`, it works as a component
argument or an attribute, where it's written as JSON: `div(data-sizes: ["S", "M"])`.

### `if`

```hypra
if user.admin { a("/admin") "Admin" } else { p "Welcome, {user.name}" }
if not cart.items { p "Your cart is empty." }
if order.status == "shipped" { p "On its way" } else if order.status != "paid" { p "Waiting for payment" }
```

A value is false when it's `false`, missing, empty text, `0` or an empty list. `==` and `!=` compare a value
with fixed text, a number, `true` or `false`, as text: `if count == 3` matches `3`. To compare two pieces of
data, prepare the answer in the controller (`if user.is_owner { ... }`).

## Writing good pages

These rules give warnings, except two that are errors: more than one `h1`, and `javascript:` URLs. A warning doesn't
stop the build, but fix it unless there's a reason (then use `allow`).

- Every page has exactly one `h1` with its main topic. Headings go down one level at a time: `h2` after `h1`,
  `h3` after `h2`, never `h2` straight to `h4`. This is checked on the page as it renders, so a component used
  right after an `h1` starts with an `h2`.
- `alt` describes what the image shows ("Blue running shoe, side view"), never `"image"`, `"photo"` or a file name.
  Images without meaning get `decorative: true`.
- Link text says where it goes ("Read the setup guide"), never just "click here", "here" or "read more". Links
  with the same text go to the same place, unless what's around them tells them apart: a list item, a table row
  or cell, or an `article` or `section` with its own heading. Otherwise put what's different into the text
  ("Read more about {post.title}") or into `aria-label`.
- Titles and descriptions are measured by how wide they are, like in search results: Chinese, Japanese and Korean
  characters count twice.
- Vague link and alt texts ("click here", "image") are checked in the page's language (`lang`): English, German,
  French, Spanish, Portuguese, Italian and Dutch so far. A page in another language isn't checked for them, and
  neither is one whose `lang` comes from data.
- An `iframe` has a `title`. A video that autoplays is `muted: true`. `tabindex` is `0` or `-1`, never positive.
- An `id` is used once per page, so a component with a fixed `id` is used once.
- `raw` with data writes it unescaped; only use it for trusted HTML. `javascript:` URLs are errors.

## Escape hatches

- `raw "<marquee>Sale</marquee>"` writes HTML as-is.
- `allow("SEO-003") "reason"` turns one rule off for the element right after it, so put it directly before the
  element that breaks the rule. The reason is required:

```hypra
allow("SEO-003") "the hero splits its headline into two h1"
h1 "Run faster."
```

An `allow` at the very start of a page covers the whole page, including rules about the page itself:

```hypra
page(title: "Dashboard", robots: "noindex") {
  allow("SEO-004") "an app screen has no main heading"
  h2 "Today"
}
```

## Formatting

`hypra fmt` lays out files the same way: two spaces per level of nesting, `name: value` and `a, b` with one
space, `p { "x" }` with spaces inside one-line blocks, and at most one blank line. Comments, line breaks and the
two spaces between items on one line stay as written. Run `hypra fmt --check` in CI to keep it that way.

## Errors

Every message has a code, a position, a fix (`help`) and a link to its rule's page (`docs`), which explains the
rule with a page that breaks it and the fixed page. Read the help line; it usually says exactly what to write. Errors stop the build; warnings don't. `allow("CODE") "reason"` turns one off for the next element.

| Code | Meaning |
| --- | --- |
| [`A11Y-001`](rules/A11Y-001.md) | image has no alt text |
| [`A11Y-002`](rules/A11Y-002.md) | link has no text |
| [`A11Y-003`](rules/A11Y-003.md) | video has no captions |
| [`A11Y-004`](rules/A11Y-004.md) | action has no label |
| [`A11Y-005`](rules/A11Y-005.md) | button has no text |
| [`A11Y-006`](rules/A11Y-006.md) | heading level skipped (warning) |
| [`A11Y-007`](rules/A11Y-007.md) | link text doesn't say where it goes (warning) |
| [`A11Y-008`](rules/A11Y-008.md) | alt text doesn't describe the image (warning) |
| [`A11Y-009`](rules/A11Y-009.md) | iframe has no title (warning) |
| [`A11Y-010`](rules/A11Y-010.md) | positive tabindex (warning) |
| [`A11Y-011`](rules/A11Y-011.md) | autoplaying video isn't muted (warning) |
| [`A11Y-012`](rules/A11Y-012.md) | ARIA name or state on a div or span without a role (warning) |
| [`A11Y-013`](rules/A11Y-013.md) | links with the same text go to different places (warning) |
| [`ALLOW-001`](rules/ALLOW-001.md) | allow has no reason |
| [`ALLOW-002`](rules/ALLOW-002.md) | allow has no rule code |
| [`ALLOW-003`](rules/ALLOW-003.md) | allow names an unknown rule (warning) |
| [`ATTR-001`](rules/ATTR-001.md) | unknown attribute (a warning on HTML elements, unless it's a near miss of a Hypra setting) |
| [`ATTR-002`](rules/ATTR-002.md) | raw has no HTML |
| [`ATTR-003`](rules/ATTR-003.md) | wrong kind of value |
| [`ATTR-004`](rules/ATTR-004.md) | unnamed argument where a name is needed |
| [`ATTR-005`](rules/ATTR-005.md) | attribute set twice |
| [`ATTR-006`](rules/ATTR-006.md) | text on an element that takes none |
| [`ATTR-007`](rules/ATTR-007.md) | default outside a component parameter |
| [`ATTR-008`](rules/ATTR-008.md) | invalid attribute name |
| [`COMP-001`](rules/COMP-001.md) | unknown component |
| [`COMP-002`](rules/COMP-002.md) | missing component argument |
| [`COMP-003`](rules/COMP-003.md) | unknown component argument |
| [`COMP-004`](rules/COMP-004.md) | component defined twice |
| [`COMP-005`](rules/COMP-005.md) | component name isn't capitalized |
| [`COMP-006`](rules/COMP-006.md) | unnamed component argument |
| [`COMP-007`](rules/COMP-007.md) | component calls itself |
| [`COMP-008`](rules/COMP-008.md) | malformed component parameter |
| [`COMP-009`](rules/COMP-009.md) | page expands to too many elements |
| [`COMP-010`](rules/COMP-010.md) | content given to a component without a `slot` |
| [`FORM-001`](rules/FORM-001.md) | field has no name or type |
| [`FORM-002`](rules/FORM-002.md) | field has no label |
| [`FORM-003`](rules/FORM-003.md) | form has no button (warning) |
| [`FORM-004`](rules/FORM-004.md) | form with fields has no name (warning) |
| [`FORM-005`](rules/FORM-005.md) | field outside a form, or inside for or if in it |
| [`FORM-006`](rules/FORM-006.md) | unknown form method |
| [`IMPORT-001`](rules/IMPORT-001.md) | imported file can't be read |
| [`LAYOUT-001`](rules/LAYOUT-001.md) | unknown layout |
| [`LAYOUT-002`](rules/LAYOUT-002.md) | layout has no slot |
| [`LAYOUT-003`](rules/LAYOUT-003.md) | layout defined twice |
| [`LAYOUT-004`](rules/LAYOUT-004.md) | layout name isn't capitalized |
| [`LAYOUT-005`](rules/LAYOUT-005.md) | layout inside itself |
| [`LEX-001`](rules/LEX-001.md) | unexpected character |
| [`LEX-002`](rules/LEX-002.md) | unterminated string |
| [`LEX-003`](rules/LEX-003.md) | unknown escape in a string |
| [`PARSE-001`](rules/PARSE-001.md) | syntax error |
| [`PARSE-002`](rules/PARSE-002.md) | nested too deeply |
| [`SEC-001`](rules/SEC-001.md) | inline event handler (warning) |
| [`SEC-002`](rules/SEC-002.md) | raw HTML from data (warning) |
| [`SEC-003`](rules/SEC-003.md) | URL that runs code |
| [`SEC-004`](rules/SEC-004.md) | data inside an attribute that runs or loads code (warning) |
| [`SEC-005`](rules/SEC-005.md) | data name in a script or style, which is written as-is (warning) |
| [`SEC-006`](rules/SEC-006.md) | `formaction` or `base` that could send forms to another site |
| [`SEC-007`](rules/SEC-007.md) | data in the `values` of an SVG animation that sets a link |
| [`SEM-001`](rules/SEM-001.md) | unknown element |
| [`SEM-002`](rules/SEM-002.md) | file has no page |
| [`SEM-003`](rules/SEM-003.md) | element in the wrong place |
| [`SEM-004`](rules/SEM-004.md) | image or video has no source |
| [`SEM-005`](rules/SEM-005.md) | action has no name |
| [`SEM-006`](rules/SEM-006.md) | slot outside a component or layout |
| [`SEM-007`](rules/SEM-007.md) | imported file contains a page |
| [`SEM-008`](rules/SEM-008.md) | void element has children |
| [`SEM-009`](rules/SEM-009.md) | html, body or title written by hand |
| [`SEM-010`](rules/SEM-010.md) | head in the wrong place |
| [`SEM-011`](rules/SEM-011.md) | element doesn't belong in head |
| [`SEM-012`](rules/SEM-012.md) | id used more than once (warning) |
| [`SEM-013`](rules/SEM-013.md) | lang isn't a language code (warning) |
| [`SEM-014`](rules/SEM-014.md) | invalid action name |
| [`SEO-001`](rules/SEO-001.md) | page has no title or description |
| [`SEO-002`](rules/SEO-002.md) | title too long (warning) |
| [`SEO-003`](rules/SEO-003.md) | more than one h1 |
| [`SEO-004`](rules/SEO-004.md) | page has no h1 (warning) |
| [`SEO-005`](rules/SEO-005.md) | canonical URL isn't absolute (warning) |
| [`SEO-006`](rules/SEO-006.md) | description too short or too long (warning) |
| [`TYPE-001`](rules/TYPE-001.md) | prop missing from the data |
| [`TYPE-002`](rules/TYPE-002.md) | unknown name or field |
| [`TYPE-003`](rules/TYPE-003.md) | value isn't a list |
| [`TYPE-004`](rules/TYPE-004.md) | unknown field type |
| [`TYPE-005`](rules/TYPE-005.md) | wrong argument or default for a field type |

## Common mistakes

| Wrong | Right |
| --- | --- |
| `text "Hello"`, `heading(1) "Hi"` | `p "Hello"`, `h1 "Hi"` |
| `h1 post.title` | `h1 "{post.title}"` |
| `a(href: "/x", "Label")` | `a("/x") "Label"` |
| `img("/a.jpg")` | `img("/a.jpg", alt: "What the image shows")` |
| `lang: en`, `robots: noindex` | `lang: "en"`, `robots: "noindex"` |
| `layout: shop`, `type: email` | `layout: Shop`, `type: Email` |
| `field("email", type: Email)` | `field("email", type: Email, label: "Email")` |
| `title "About"` inside the page | `page(title: "About", ...)` |
| `p "Hi"` on one line, `{` on the next | `p "Hi" {` on one line |
| using data without `props` | `props(user: User)` first |
| `@click: "open = true"` | `"@click": "open = true"` (names without a dash are quoted) |
| `onclick: "..."` | a script in `head`, or `"@click"` with Alpine (`onclick` works but warns) |
