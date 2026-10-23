# Add llms.txt to your website

By the end of this guide, your site has an `/llms.txt`: a short Markdown file that tells AI tools what the site
is and links to a clean Markdown version of each page, generated from your pages so it never goes stale.

## What llms.txt is

[llms.txt](https://llmstxt.org) is a proposal from Jeremy Howard of Answer.AI, from September 2024. A site puts
a Markdown file at `/llms.txt` with:

- an `h1` with the site's or project's name, the only part that's required;
- a blockquote with a one-line summary;
- `h2` sections with lists of links, each with a short note on what's behind it;
- optionally a section called `Optional`, for links a tool can skip when it's short on space.

The links point to Markdown versions of the pages, so a tool can read exactly what it needs without HTML around
it. Many sites also offer `/llms-full.txt`, all of those pages in one file.

## Who reads it

Be realistic about what it does. As of late 2026, none of the large AI companies has committed to reading
llms.txt in its crawlers or search, and Google has said it doesn't use it. Studies of crawler traffic find that
most AI bots skip the file and read the HTML.

Chrome's side of Google is more interested: Lighthouse 13.3, from May 2026, added an experimental "Agentic
browsing" category that checks for an llms.txt, as one sign that agents can find their way around a site.

Where it does help is with tools that fetch documentation on demand: coding assistants, agents and the
documentation indexes behind them, which read a list of clean Markdown pages far more easily than a website. So
llms.txt is worth having for documentation, APIs and developer tools, and cheap enough to add anywhere. It's not
a way to rank in AI answers. For that, see
[Make your website readable for AI agents](https://hypralang.dev/guides/website-readable-for-ai-agents/).

## Without Hypra

For a small site, write it by hand and serve it as a static file:

```markdown
# Tidewater

> A command-line tool that shows tide tables for any coast.

## Docs

- [Install Tidewater](https://tidewater.dev/docs/install.md): install it on macOS, Linux or Windows
- [Configure Tidewater](https://tidewater.dev/docs/configure.md): the settings in tidewater.toml

## Optional

- [Changelog](https://tidewater.dev/changelog.md)
```

The links need Markdown versions of the pages behind them. Documentation tools such as Mintlify and Rspress
generate both the file and the Markdown pages, and Docusaurus and VitePress have community plugins that do. For
other sites you write the Markdown versions yourself, or convert your HTML, and keep the list in step with your
pages by hand.

## With Hypra

[Hypra](https://hypralang.dev) compiles every page into HTML and Markdown together, plus a JSON file with the
page's title and description. So the Markdown versions exist already, and llms.txt can be generated from them.
Two documentation pages, `pages/docs/install.hypra`:

```hypra
page(title: "Install Tidewater", description: "Install the Tidewater command-line tool on macOS, Linux or Windows, and check that it works.") {
  h1 "Install Tidewater"
  p "Tidewater runs on macOS, Linux and Windows."
  pre { code "npm install --global tidewater" }
  h2 "Check that it works"
  pre { code "tidewater --version" }
}
```

and `pages/docs/configure.hypra`:

```hypra
page(title: "Configure Tidewater", description: "The settings in tidewater.toml: where the tide data comes from, and which units Tidewater shows.") {
  h1 "Configure Tidewater"
  p { "Settings live in " code "tidewater.toml" " next to your project." }
  dl {
    dt { code "source" }
    dd "Where the tide data comes from: noaa or a URL of your own."
    dt { code "units" }
    dd "metric or imperial."
  }
}
```

Build the folder; every page gets its `.html`, `.md` and `.json`:

```console
$ npx hypra build pages --out dist
```

`dist/docs/configure.md` is the page as Markdown:

```markdown output
---
title: "Configure Tidewater"
description: "The settings in tidewater.toml: where the tide data comes from, and which units Tidewater shows."
---

# Configure Tidewater

Settings live in `tidewater.toml` next to your project.

`source`

Where the tide data comes from: noaa or a URL of your own.

`units`

metric or imperial.
```

A short script after the build writes llms.txt from the JSON files, with each page's own title and description:

```js
// scripts/llms.js: writes dist/llms.txt, a link to the Markdown of every page hypra built
import { readdirSync, readFileSync, writeFileSync } from "node:fs";

const site = "https://tidewater.dev";
const pages = readdirSync("dist", { recursive: true })
  .filter((file) => file.endsWith(".json") && !file.endsWith(".webmcp.json"))
  .sort();

const links = pages.map((file) => {
  const { title, description } = JSON.parse(readFileSync(`dist/${file}`, "utf8"));
  const url = `${site}/${file.replaceAll("\\", "/").replace(/\.json$/, ".md")}`;
  return `- [${title}](${url}): ${description}`;
});

writeFileSync(
  "dist/llms.txt",
  `# Tidewater\n\n> A command-line tool that shows tide tables for any coast.\n\n## Docs\n\n${links.join("\n")}\n`,
);
```

```console
$ npx hypra build pages --out dist && node scripts/llms.js
```

```markdown
# Tidewater

> A command-line tool that shows tide tables for any coast.

## Docs

- [Configure Tidewater](https://tidewater.dev/docs/configure.md): The settings in tidewater.toml: where the tide data comes from, and which units Tidewater shows.
- [Install Tidewater](https://tidewater.dev/docs/install.md): Install the Tidewater command-line tool on macOS, Linux or Windows, and check that it works.
```

What's different from keeping it by hand:

- **It can't go stale.** A new page is in llms.txt with the next build, and a changed description changes there
  too.
- **The descriptions are good ones.** Hypra requires every page to have a title and a description, and warns when
  they're too long or too short for search results, so the notes in llms.txt get the same care.
- **The Markdown is the page's content**, written from the same source as the HTML, not converted from it.

[hypralang.dev/llms.txt](https://hypralang.dev/llms.txt) is made this way, with
[llms-full.txt](https://hypralang.dev/llms-full.txt) next to it.

## Limits

- llms.txt is a proposal, not a standard, and most AI crawlers don't read it today.
- Hypra is new and pre-1.0, so the language may still change.
- Pages that need data, like a product page, are built for each item with `--data`, or rendered on request; the
  script above covers pages that don't.

## Next steps

- [Serve Markdown to AI agents from Node.js](https://hypralang.dev/guides/markdown-for-ai-agents-in-node/): the
  same Markdown from the page's own URL, for agents that ask for it.
- The [language reference](https://hypralang.dev/docs/) has everything a page can contain.
