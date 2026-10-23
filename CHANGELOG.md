# Changelog

Hypra is pre-1.0: a minor version (0.2, 0.3) may change the language or its output, a patch version (0.1.1) only
fixes and adds checks.

## Unreleased

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
