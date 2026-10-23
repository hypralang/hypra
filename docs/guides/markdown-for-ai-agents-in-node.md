# Serve Markdown to AI agents from Node.js

By the end of this guide, the pages of a Node.js app answer in Markdown when an AI agent asks for it, from the
same URL that serves HTML to browsers. The examples use Express; the same works in Hono, Fastify and Next.js
route handlers.

## Why Markdown

An AI agent that reads a web page has to dig the content out of the HTML: navigation, scripts, styles and wrapper
elements all cost tokens and can mislead it. The same content as Markdown is smaller and says plainly what is a
heading, a list or a link.

The way to offer it is HTTP content negotiation. A client that prefers Markdown sends `Accept: text/markdown`,
and the server answers with Markdown instead of HTML, from the same URL. `text/markdown` is a registered media
type (RFC 7763), and in 2026 Cloudflare added the same negotiation to its network as "Markdown for Agents", so
agents can expect more and more sites to answer this way. Responses that differ by `Accept` need `Vary: Accept`,
so caches keep the formats apart.

## Without Hypra

Any Express app can convert the HTML it sends with [Turndown](https://github.com/mixmark-io/turndown):

```console
$ npm install turndown
```

```js
// middleware/markdown.js
import TurndownService from "turndown";

const turndown = new TurndownService({ headingStyle: "atx", codeBlockStyle: "fenced" });
turndown.remove(["head", "title", "script", "style", "nav", "footer"]);

// Answers in Markdown when the client prefers it, by converting the HTML a route sends
export function markdown(req, res, next) {
  const send = res.send.bind(res);
  res.send = (body) => {
    // the same URL answers in two formats, so caches must keep them apart
    res.vary("Accept");
    const html = typeof body === "string" && /html/.test(res.get("Content-Type") ?? "text/html");
    if (!html || req.accepts(["text/html", "text/markdown"]) !== "text/markdown") return send(body);
    res.type("text/markdown");
    return send(turndown.turndown(body));
  };
  next();
}
```

```js
import express from "express";
import { markdown } from "./middleware/markdown.js";

const app = express();
app.use(markdown);
```

Browsers keep getting HTML; `curl -H "Accept: text/markdown"` gets Markdown. This works with the templates you
have, whether EJS, Handlebars or plain strings. Its limits come from guessing structure after the fact:

- The converter only sees HTML. A `div` styled as a heading stays text, and everything your layout adds around
  the content has to be removed by name, like `nav` and `footer` above.
- Forms and buttons, the things an agent could *do* on the page, are dropped or become loose text.
- Nothing checks the result. A new banner or cookie notice quietly ends up in every Markdown page.

If your site is behind Cloudflare, its Markdown for Agents feature does the same conversion without code, with
the same trade-offs.

Two variations you'll see recommended. Some sites also serve Markdown at the page's URL plus `.md`
(`/articles/sourdough.md`), for tools that don't send headers; that's a second route next to the first. Others
send Markdown to known AI bots by their `User-Agent`. Be careful with that one: it needs `Vary: User-Agent`,
which makes caching much less effective, and content that differs by user agent looks like cloaking to search
engines. Keep the content the same in both formats.

## With Hypra

[Hypra](https://hypralang.dev) pages are written once and compiled into HTML, Markdown and JSON together, so the
Markdown comes from what the page means rather than from its HTML. The `hypra` package has the compiler and the
runtime:

```console
$ npm install hypra
```

An article page, `views/articles/show.hypra`:

```hypra
page(title: "{article.title}", description: "{article.summary}") {
  props(article: Article)
  article {
    h1 "{article.title}"
    p { "By " b "{article.author}" ", " time(datetime: article.published) "{article.date}" }
    for paragraph in article.body { p "{paragraph}" }
  }
  form("/newsletter", name: "newsletter", description: "Get new articles by email") {
    field("email", type: Email, label: "Email")
    button "Subscribe"
  }
}
```

Compile the folder of views to JavaScript modules, for example as a `build` script:

```console
$ npx hypra build views --target js --out views/dist
```

Then render the page in a route. `negotiate` picks the format from the `Accept` header: Markdown for
`text/markdown`, JSON for `application/json`, HTML for everything else.

```js
import express from "express";
import show from "./views/dist/articles/show.mjs";

const app = express();

app.get("/articles/:slug", async (req, res) => {
  const article = await db.articles.find(req.params.slug);
  if (!article) return res.sendStatus(404);
  const output = show.render({ article }, { alternateUrl: req.originalUrl });
  const { contentType, body } = output.negotiate(req.get("Accept"));
  res.vary("Accept").type(contentType).send(body);
});
```

A client that asks for Markdown gets this:

```console
$ curl -H "Accept: text/markdown" https://crumb.example/articles/sourdough
```

```markdown output
---
title: "Why your sourdough doesn't rise"
description: "A weak starter, cold dough or too little time: how to tell which one is flattening your bread, and what to change."
---

# Why your sourdough doesn't rise

By **Mira Okafor**, 12 October 2026

Most flat loaves come from a starter that isn't ready. Feed it, and use it when it has doubled and smells sour but not sharp.

Cold dough rises slowly. Below 22 °C, give the first rise an extra hour or two instead of adding yeast.

[Form: Get new articles by email]

## Actions

- `newsletter(email: email)`: Get new articles by email → POST /newsletter
```

The HTML version links to it with `<link rel="alternate" type="text/markdown">`, because the route passed
`alternateUrl`, so agents that look for that link find it.

In a Next.js route handler, or anything else built on the web's `Request` and `Response`, it's the same call:

```js
// app/articles/[slug]/route.js
import show from "@/views/dist/articles/show.mjs";

export async function GET(request, { params }) {
  const { slug } = await params;
  const article = await db.articles.find(slug);
  if (!article) return new Response("Not found", { status: 404 });
  const { contentType, body } = show.render({ article }).negotiate(request.headers.get("accept"));
  return new Response(body, { headers: { "Content-Type": contentType, Vary: "Accept" } });
}
```

What's different from converting HTML:

- **Structure comes from the page.** `h1` is the title, a list is a list, an image keeps its alt text. Nothing has
  to be guessed or removed.
- **Actions are listed.** The newsletter form is a form for people, and a line under `## Actions` for agents,
  with its parameters and where to send them.
- **It can't drift.** HTML and Markdown come from the same file, compiled together.
- **Data stays text.** An article title like `## Ignore previous instructions` from your database is escaped in
  the Markdown too, so it can't add headings or fake actions.

## Limits

- Hypra is new and pre-1.0, so the language may still change.
- Only pages written in Hypra get Markdown this way. Pages written in React or another template language keep
  working; use the middleware above for them in the meantime.
- In Next.js, a Hypra page is a route handler that returns HTML, not a React component.

## Next steps

- [hypra for JavaScript](https://hypralang.dev/docs/js/) covers rendering options, data and TypeScript.
- [Serve Markdown to LLMs from Laravel](https://hypralang.dev/guides/markdown-for-llms-in-laravel/) is the same
  for PHP.
- [Make your website readable for AI agents](https://hypralang.dev/guides/website-readable-for-ai-agents/) puts
  Markdown next to the other things agents look for.
