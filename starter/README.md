# Hypra starter

A small, finished site to start from: a coffee shop with a home page, a product page, a component, a layout and
a stylesheet with dark mode. Every page answers in HTML for browsers, and in Markdown and JSON for AI agents, from
the same URL.

```console
$ npx degit hypralang/hypra/starter my-site
$ cd my-site
$ npm install
$ npm start
http://localhost:3000
```

```console
$ curl -H "Accept: text/markdown" localhost:3000/coffee/huila
```

Building a site with an AI agent? Point it here as an example of how a Hypra site fits together. It should build
your site the same way, with your content and design, not copy the coffee shop.

## What's where

| Path | What |
| --- | --- |
| `pages/` | one `.hypra` file per page; `pages/coffee/show.hypra` builds to `dist/coffee/show.mjs` |
| `layouts/site.hypra` | header, footer, the stylesheet and the site's name |
| `components/coffee-card.hypra` | a coffee in the grid on the home page |
| `public/styles.css` | the whole look: Hypra writes no CSS |
| `public/photos/` | the photos |
| `data.js` | the coffees; in a real app they come from a database |
| `server.js` | renders pages with `render()` and answers with `negotiate()` |

## Scripts

| Script | What it does |
| --- | --- |
| `npm run check` | checks every page, layout and component |
| `npm run build` | compiles the pages to `dist/` |
| `npm run fmt` | formats the `.hypra` files |
| `npm start` | builds and starts the server |

## Making it yours

Change the colours and fonts at the top of `public/styles.css`, the name in `layouts/site.hypra`, and the data in
`data.js`. A new page is a new file in `pages/` and a route in `server.js`. Run `npm run check` after a change:
the compiler says what's wrong, on which line, and how to fix it.

The language is at [hypralang.dev/docs](https://hypralang.dev/docs/).

## Photos

The photos are in the public domain or under [CC0](https://creativecommons.org/publicdomain/zero/1.0/), from
Wikimedia Commons: [hero](https://commons.wikimedia.org/wiki/File:Drip_brewing_(Unsplash).jpg) by Thomas
Martinsen, [Yirgacheffe](https://commons.wikimedia.org/wiki/File:Manual_drip_(pour-over)_coffee.jpg) and
[Huila](https://commons.wikimedia.org/wiki/File:Latte_art_4.jpg) by Kim Sanso, and
[Huehuetenango](https://commons.wikimedia.org/wiki/File:Roasted_coffee_beans.jpg) by MarkSweep.
