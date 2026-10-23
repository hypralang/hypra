# Contributing

Issues and pull requests are welcome. [ROADMAP.md](ROADMAP.md) lists what's planned and
[CHANGELOG.md](CHANGELOG.md) what changed in each release. For something bigger than a fix, open an
[issue](https://github.com/hypralang/hypra/issues) first, so we can agree on the idea before you write the code.

## Before opening a pull request

```console
$ cargo fmt
$ cargo clippy --all-targets -- -D warnings
$ cargo test
```

## Changing a rule

A new or changed rule needs:

- its row in [docs/spec.md](docs/spec.md)
- its page in [docs/rules](docs/rules), with an example that breaks it and one that passes:

  ````markdown
  ```hypra fails
  div(onclick: "open()") { "Menu" }
  ```

  ```hypra passes
  button(type: "button", onclick: "open()") { "Menu" }
  ```
  ````

`cargo test` compiles every example on the rule pages and fails when a `fails` example passes or a `passes`
example fails.

## Where things are

| Path | What |
| --- | --- |
| `src` | the compiler: lexer, parser, checks, IR and the HTML, PHP and JavaScript outputs |
| `js`, `php` | the runtimes for compiled pages, and the Laravel package |
| `editors/vscode` | the extension for VS Code and Cursor |
| `site` | hypralang.dev, written in Hypra |
| `starter` | the example site people copy with `npx degit hypralang/hypra/starter` |
| `examples` | the pages the README and the tests use |
| `docs` | the spec, the rule pages and the guides |

## Security

Please report vulnerabilities privately, as [SECURITY.md](SECURITY.md) describes, not in an issue.
