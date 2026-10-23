# Changelog

Hypra is pre-1.0: a minor version (0.2, 0.3) may change the language or its output, a patch version (0.1.1) only
fixes and adds checks.

## Unreleased

- The VS Code extension previews a page next to the editor (Ctrl+K V): as a browser shows it, and the Markdown
  and JSON it compiles to. It's rendered again on every save, with the data from `product.json` next to
  `product.hypra` if there is one
- [starter](starter): a small, styled site to start from, with pages, a layout, a component, dark mode and a
  Node server that answers in HTML, Markdown and JSON: `npx degit hypralang/hypra/starter my-site`. The
  instructions for agents point to it as an example to build from, not a site to copy.
- "From HTML" in the reference shows how HTML is written in Hypra, and the instructions for agents say to design
  pages as they would in HTML

## 0.3.1

- `hypra fmt` takes folders too, like `check` and `build`
- A build given a folder of only components and layouts passes over it instead of stopping:
  `hypra build pages layouts` builds the pages

## 0.3.0

- `hypra check` and `hypra build` take several files, and folders: `hypra check views` checks every `.hypra`
  file in it, and `hypra build views --target js --out dist` builds every page, in the same subfolders
- The reference says how to style pages: Hypra writes no CSS, so a page needs a stylesheet or a framework like
  Tailwind, best in its layout

## 0.2.1

- The VS Code extension finds the compiler a project installed, with `npm install hypra` or
  `php artisan hypra:install`, so it works without `hypra` on the `PATH`. In a folder VS Code doesn't trust, it
  highlights `.hypra` files without running a compiler; before, it stayed off there.

## 0.2.0

- JavaScript: `hypra build --target js` compiles a page to an ES module that renders at request time in Node, Bun
  and Deno, exactly like the static build. The `hypra` npm package has the runtime, with types, and the compiler
  for every system: `npx hypra build`. `output.negotiate(accept)` serves HTML, Markdown or JSON from one URL.
- [Hypra with AI coding agents](https://hypralang.dev/docs/agents/): what to tell Claude Code, Codex, Cursor and
  Copilot so they write Hypra, and [llms-full.txt](https://hypralang.dev/llms-full.txt) with all the docs in one file
- Every rule has a page on [hypralang.dev/rules](https://hypralang.dev/rules/), with an example that breaks it and
  the fix, and every message links to it: `= docs:` in the terminal, `docs` in `--format json`, and the code in
  the editor
- A link or button that holds only an icon, like a decorative image, has no name for screen readers: A11Y-002
  and A11Y-005 now report it. A link with an `aria-label` and no content no longer gets A11Y-002.
- Link and alt text checks know French, Spanish, Portuguese, Italian and Dutch
- New errors A11Y-014, a click handler on something that isn't a control, like `div(onclick: ...)`, and A11Y-015,
  an `input`, `select` or `textarea` without a label

## 0.1.1

- Link and alt text are checked against the words of the page's `lang`, English or German, and a page in another
  language isn't checked against English words. Components are checked in the language of the page that uses
  them.
- New check A11Y-013: links with the same text that go to different places, unless a list item, table row or card
  around each link tells them apart
- Titles and descriptions are measured by how wide they show in search results: Chinese, Japanese and Korean
  characters count twice
- `php artisan hypra:compile` says "1 page" for one page

## 0.1.0

The first release: the language and its compiler, the static build to HTML, Markdown, JSON and WebMCP, the PHP
target with `hypra/runtime`, the Laravel package `hypra/laravel`, `hypra fmt`, the language server `hypra lsp`
and the extension for VS Code and Cursor.
