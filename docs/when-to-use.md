# When to use Hypra

Hypra is a language you write pages in, so it asks more of you than a package you add to the site you have. This
page says when that's worth it, and what to use instead when it isn't.

## Hypra fits when

- **You start a site or a new part of one,** and want it accessible, readable for AI agents and checked from the
  first page, rather than fixing it later.
- **AI writes your pages.** Coding agents make the same mistakes again and again: images without alt text,
  fields without labels, clickable `div`s. Hypra's compiler refuses them and says how to fix each one, so the
  agent fixes its own work.
- **You want one source for people and agents:** the HTML, the Markdown, the JSON and the WebMCP tools come from
  the same file and can't drift apart.
- **Your pages are content and forms:** documentation, marketing pages, articles, shops, sign-ups, dashboards
  rendered on the server.

## Use something else when

| You want | Use | Why |
| --- | --- | --- |
| Markdown for AI agents on a site you already have | [laravel-markdown-response](https://spatie.be/docs/laravel-markdown-response/v1) for Laravel, [markdown-for-agents](https://github.com/KKonstantinov/markdown-for-agents) for Node, or Cloudflare's Markdown for Agents | one middleware, no templates to rewrite |
| Accessibility checks in React, Vue or Angular | [eslint-plugin-jsx-a11y](https://github.com/jsx-eslint/eslint-plugin-jsx-a11y), eslint-plugin-vuejs-accessibility, angular-eslint | they check the templates you have |
| Accessibility checks on any rendered page | [axe](https://github.com/dequelabs/axe-core) in your end-to-end tests | sees what only a browser sees, like contrast; keep it next to Hypra too |
| WebMCP on a few existing forms | the `toolname` and `tooldescription` attributes by hand | two attributes per form |
| An app that runs in the browser | React, Vue, Svelte | Hypra writes HTML; it isn't a JavaScript framework |
| Emails | [MJML](https://mjml.io) | email clients need their own HTML, which Hypra doesn't write |

## Compared with templates

Blade, Twig, Handlebars, EJS and JSX all write HTML. The difference is what they know about it:

| | Blade, Twig, Handlebars, EJS | JSX with a linter | Hypra |
| --- | --- | --- | --- |
| An image without alt text | renders | linter warning | doesn't build |
| A field without a label | renders | linter warning, if the rule is on | doesn't build |
| Data that contains HTML | escaped, unless a raw output is used | escaped | always escaped, also in Markdown and JSON |
| A Markdown version for agents | converted from the HTML, if at all | converted from the HTML, if at all | written from the page |
| Forms as tools for agents | by hand | by hand | from the form's fields, with their types |
| A page missing its title or description | renders | renders | doesn't build |

Hypra also works next to them: a Laravel app can render some views from Hypra and keep the rest in Blade, and a
Node app can render Hypra pages from the routes that need them.

## What Hypra doesn't do (yet)

- **No Vite plugin yet,** so Vite, Astro and SvelteKit projects compile pages with the CLI for now. It's next on
  the [roadmap](https://github.com/hypralang/hypra/blob/main/ROADMAP.md).
- **Two server languages:** JavaScript (Node, Bun, Deno) and PHP (including Laravel). Other languages can use the
  static build or call `hypra build`.
- **The editor extension is on Open VSX**, which Cursor, VSCodium and Windsurf use. In VS Code, install it from the
  release.
- **Pre-1.0:** the language may still change between minor versions; the [changelog](https://github.com/hypralang/hypra/blob/main/CHANGELOG.md)
  says what changed.

## Try it

The [playground](https://hypralang.dev/play/) compiles as you type, in the browser: open the "Mistakes" example to
see what the compiler refuses, or the form example to see the tools agents get.
