# Changelog

Hypra is pre-1.0: a minor version (0.2, 0.3) may change the language or its output, a patch version (0.1.1) only
fixes and adds checks.

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
