# Hypra with AI coding agents

Coding agents like Claude Code, Codex, Cursor and GitHub Copilot write Hypra well when they have two things: the
language reference, and the compiler's errors after every change. In the [benchmark](../bench), a small model wrote
19 of 20 pages correctly on the first try from the reference alone, and all 20 after reading the compiler's errors.

## Tell the agent about Hypra

Agents read a file of instructions at the root of a project: `AGENTS.md` for Codex, Cursor, Copilot and most
others, `CLAUDE.md` for Claude Code. Add this to it:

```markdown
## Views

The views in this project are Hypra (.hypra files), not HTML or Blade.

- The language: https://hypralang.dev/llms-full.txt. Read it before writing a .hypra file.
- After changing a .hypra file, run `hypra check <file>` and fix every error and warning. Each one has a code, a line and a help line that says what to change.
- Don't silence a check with `allow` unless the page really needs it; give the reason.
```

For Claude Code, a `CLAUDE.md` with the line `@AGENTS.md` reads the same instructions, so they're kept in one place.

## What the agent reads

| URL | What it is |
| --- | --- |
| [`/llms-full.txt`](https://hypralang.dev/llms-full.txt) | the whole documentation in one Markdown file: the language, Laravel, PHP, the editors, security and the guides |
| [`/llms.txt`](https://hypralang.dev/llms.txt) | a list of every page, with a line on each, for agents that read only what they need |
| [`/docs/index.md`](https://hypralang.dev/docs/index.md) | the [language reference](spec.md) alone; every page of the docs has its Markdown at `index.md` |

## Checking its work

`hypra check` exits with 1 when a page has an error, so an agent sees that something is wrong even if it doesn't
read the output. Every error links to a page about its rule, with an example and the fix:

```console
$ hypra check shoes.hypra
error[A11Y-001]: image has no alt text
 --> shoes.hypra:3:7
  |
3 |   img("/shoe.jpg")
  |       ^^^^^^^^^^^
  = help: add alt: "..." or decorative: true
  = docs: https://hypralang.dev/rules/a11y-001/

1 error, 0 warnings
```

Scripts and tools that read the errors get them as JSON with `--format json`:

```json
[
  {
    "severity": "error",
    "code": "A11Y-001",
    "message": "image has no alt text",
    "file": "shoes.hypra",
    "line": 3,
    "col": 7,
    "len": 11,
    "help": "add alt: \"...\" or decorative: true",
    "docs": "https://hypralang.dev/rules/a11y-001/"
  }
]
```

In Cursor and VS Code, the [extension](../editors/vscode) shows the same errors in the editor as the agent writes,
and Cursor's agent reads them from there too.

## Pages for agents, too

This page is about agents that *write* Hypra. Pages written in Hypra also answer agents that *read* the web: in
Markdown and JSON, and with WebMCP tools for their forms and buttons. See
[Serve Markdown to LLMs from Laravel](guides/markdown-for-llms-in-laravel.md) and
[Add WebMCP tools to your website](guides/webmcp-tools-for-your-website.md).
