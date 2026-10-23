# Make your website readable for AI agents

By the end of this guide, you know what AI crawlers, assistants and agents read on a website, which changes make
your pages easier for them to understand and use, and which of those matter today rather than someday.

## How AI reads the web

There are three different readers, and they need different things:

- **Crawlers** collect pages ahead of time, for training or for the search indexes behind AI answers: GPTBot,
  ClaudeBot and PerplexityBot, for example, next to the usual search engine crawlers.
- **Assistants** fetch a page while someone is asking about it, like ChatGPT, Claude or Perplexity when they
  search the web, and coding assistants reading documentation.
- **Agents** use a site on someone's behalf: they open pages in a browser, fill in forms and press buttons.

Nobody can promise that a page gets cited in AI answers; the "generative engine optimization" (GEO) advice that
does is guessing. What you can control is whether these readers can get at your content, understand it and act
on it. That's what the steps below are about, roughly from the most proven to the most experimental.

## 1. Put the content in the HTML

Most AI crawlers don't run JavaScript. A page that builds its content in the browser, like a single-page app
without server rendering, looks empty to them. Render on the server or build static pages, and check what a
crawler sees:

```console
$ curl -s https://example.com/articles/sourdough | grep "Why your sourdough"
```

If the text isn't in what `curl` returns, most AI crawlers don't see it either. Keep the rest of the basics too:
a sitemap listed in `robots.txt`, stable URLs, and no login or paywall in front of what you want read.

## 2. Say what each page is

The same things that help search engines and screen readers help language models: a `title` and a
`<meta name="description">` that say what the page is about, one `h1`, headings that follow each other in order,
`alt` text that describes images, links whose text says where they go, and real elements, like `button` for
buttons and `nav` for navigation, instead of styled `div`s. A model reading the page has no styles to go by;
the structure is all it gets.

Write each section so it stands on its own. AI answers often quote one passage, not the whole page, so put the
answer before the explanation and state the facts plainly, like prices, dates, opening hours and versions. A
"last updated" date on pages that change, and questions people really ask answered in their own words, help too.
None of this is AI-specific: it's what good search results have always looked like.

## 3. Add structured data

Search engines read [schema.org](https://schema.org) data in a JSON-LD script to know that a page is an article,
a product, a recipe or an event, with its author, price or date, and AI answers built on search indexes inherit
that. Add it for the kinds of pages that have a schema.org type:

```html
<script type="application/ld+json">
  {"@context": "https://schema.org", "@type": "Article", "headline": "Why your sourdough doesn't rise",
   "author": {"@type": "Person", "name": "Mira Okafor"}, "datePublished": "2026-10-12"}
</script>
```

## 4. Decide what crawlers may do

The big AI companies each run several bots with different jobs: one collects pages for training, one builds the
index their AI search answers from, and one fetches a page when a person asks about it. OpenAI's are GPTBot,
OAI-SearchBot and ChatGPT-User; Anthropic's are ClaudeBot, Claude-SearchBot and Claude-User. They follow
`robots.txt`, so you can treat them differently. For example, to keep your pages out of training but stay in AI
search and in answers people ask for:

```text
User-agent: GPTBot
Disallow: /

User-agent: Google-Extended
Disallow: /

User-agent: OAI-SearchBot
Allow: /

User-agent: ChatGPT-User
Allow: /
```

Check each company's documentation for its current bot names: they add and rename them. Blocking a search bot
keeps your pages out of that AI search; blocking a training bot doesn't.

`robots.txt` isn't the only gate. CDNs and firewalls, Cloudflare among them, can block AI bots on their own, in
some setups by default. If AI tools can't read your site, check there too.

## 5. Offer Markdown

Assistants and agents turn HTML into text before they read it, and lose structure on the way. A site can save
them the trouble: when a request says `Accept: text/markdown`, answer with the page as Markdown, from the same
URL. Cloudflare added this to its network in 2026 as "Markdown for Agents". How to do it yourself:
[from Node.js](https://hypralang.dev/guides/markdown-for-ai-agents-in-node/) and
[from Laravel](https://hypralang.dev/guides/markdown-for-llms-in-laravel/).

## 6. Add llms.txt for documentation

[llms.txt](https://llmstxt.org) is a Markdown file at `/llms.txt` that lists a site's pages, with links to their
Markdown versions. The big AI crawlers don't read it today, but coding assistants and documentation tools do, so
it's worth it for docs and developer tools:
[Add llms.txt to your website](https://hypralang.dev/guides/llms-txt-for-your-website/).

## 7. Describe what agents can do

Agents that use a site in a browser guess which button does what. WebMCP, a proposal that Chrome and Edge are
testing, lets a form say it directly with two attributes, `toolname` and `tooldescription`, from which the
browser builds a tool with typed parameters. Browsers that don't know them ignore them, so it's safe to add now:
[Add WebMCP tools to your website](https://hypralang.dev/guides/webmcp-tools-for-your-website/).

## What matters today

| Step | Who uses it today | Effort |
| --- | --- | --- |
| Content in the HTML | every crawler and assistant | none for server-rendered sites; real work for single-page apps |
| Titles, headings, alt text, real elements | everyone, including search engines and screen readers | small, page by page |
| Structured data | search engines, and the AI answers built on them | small per page type |
| robots.txt rules for AI bots, and the CDN's bot settings | the bots of the big AI companies | a few lines |
| Markdown on request | some assistants and agents; Cloudflare offers it | a middleware, or a page that compiles to it |
| llms.txt | coding assistants and documentation tools | a generated file |
| WebMCP | browsers testing it | two attributes per form |

## With Hypra

[Hypra](https://hypralang.dev) is a markup language whose compiler turns one file into the HTML for browsers and
crawlers, the Markdown for assistants and the WebMCP tools for agents, and checks steps 1 and 2 before a page can
ship. A page with its structured data:

```hypra
page(title: "{article.title}", description: "{article.summary}") {
  props(article: Article)
  head {
    script(type: "application/ld+json") """
      {"@context": "https://schema.org", "@type": "Article", "headline": {article.title},
       "author": {"@type": "Person", "name": {article.author}}, "datePublished": {article.published}}
      """
  }
  article {
    h1 "{article.title}"
    for paragraph in article.body { p "{paragraph}" }
  }
  form("/newsletter", name: "newsletter", description: "Get new articles by email") {
    field("email", type: Email, label: "Email")
    button "Subscribe"
  }
}
```

From this one file:

- **The HTML** has the content, the title, description and Open Graph tags, and the JSON-LD with the data written
  safely into it.
- **The Markdown** has the article and a list of what an agent can do, here the newsletter form with its
  parameter, served to clients that ask for `text/markdown`.
- **The form** is a WebMCP tool, `newsletter(email)`, with a JSON Schema for its parameter.
- **The compiler refuses** a page without a description, an image without `alt`, a field without a label or a
  clickable `div`, and warns about skipped headings and links that say "click here".

## Limits

- None of this guarantees that AI answers cite you. It makes your pages easy to read, understand and use; what a
  model does with them is up to the model.
- llms.txt and WebMCP are proposals and may change or fade. Steps 1 to 4 have been good practice for search for
  years and will stay so.
- Hypra is new and pre-1.0, so the language may still change.

## Next steps

- [Serve Markdown to AI agents from Node.js](https://hypralang.dev/guides/markdown-for-ai-agents-in-node/)
- [Add llms.txt to your website](https://hypralang.dev/guides/llms-txt-for-your-website/)
- [Add WebMCP tools to your website](https://hypralang.dev/guides/webmcp-tools-for-your-website/)
- [Accessibility checks at compile time](https://hypralang.dev/guides/accessibility-checks-at-compile-time/), the
  checks behind step 2.
