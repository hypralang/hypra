# hypra for JavaScript

The [Hypra](https://hypralang.dev) compiler, and the runtime for the pages it compiles to JavaScript. A page
renders at request time in Node, Bun or Deno, from Express, Hono, Fastify, Next.js route handlers or any other
server: as HTML for browsers, Markdown for LLMs, JSON for apps and WebMCP tools for AI agents.

```console
$ npm install hypra
$ npx hypra build views --target js --out views/dist
✔ views/dist/home.mjs
✔ views/dist/home.d.mts
✔ views/dist/home.sources.json
✔ views/dist/product.mjs
✔ views/dist/product.d.mts
✔ views/dist/product.sources.json
2 files, 0 errors, 0 warnings
```

The package brings the compiler for Linux, macOS and Windows on x64 and ARM; nothing is downloaded when it's
installed. `npx hypra check views` and `npx hypra fmt views` work the same way.

## Rendering a page

A compiled page is an ES module. Its `render` takes the page's data, the props it declares:

```js
import product from "./views/dist/product.mjs";

const output = product.render({ product: await db.products.find(42) });

output.html;      // the page
output.markdown;  // for LLMs
output.json;      // page metadata and actions
output.webmcp;    // tool definitions for AI agents
```

One URL can serve all of them. `negotiate` picks what the request's `Accept` header asks for: Markdown for
`text/markdown`, JSON for `application/json`, HTML for everything else.

```js
import express from "express";
import product from "./views/dist/product.mjs";

const app = express();

app.get("/shoes/:id", async (req, res) => {
  const output = product.render({ product: await db.products.find(req.params.id) });
  const { contentType, body } = output.negotiate(req.get("accept"));
  res.vary("Accept").type(contentType).send(body);
});
```

With Hono, or anything else built on the web's `Request` and `Response`:

```js
app.get("/shoes/:id", async (c) => {
  const output = product.render({ product: await db.products.find(c.req.param("id")) });
  const { contentType, body } = output.negotiate(c.req.header("accept"));
  return c.body(body, 200, { "Content-Type": contentType, Vary: "Accept" });
});
```

## Options

`render(data, options)` takes:

| Option | Meaning |
| --- | --- |
| `strict` | throw a `DataError` naming the `.hypra` line when data is missing, instead of rendering it empty |
| `formFields` | a function returning hidden fields for POST forms to this site, e.g. `() => ({ _csrf: req.csrfToken() })` |
| `alternateUrl` | where the page's Markdown is served; adds `<link rel="alternate" type="text/markdown">` |

```js
import { DataError } from "hypra/runtime";

try {
  product.render({}, { strict: true });
} catch (error) {
  if (error instanceof DataError) console.error(error.message);
  // views/product.hypra:13:9: prop `product` is missing from the data
}
```

Hidden form fields only go to forms that post to this site, never to a form whose address leads somewhere else.

## Data

Data is what JSON can hold, plus what JavaScript apps pass around:

- objects, arrays, strings, numbers, booleans and `null`
- `Map` and `Set`, read like objects and arrays
- class instances, whose getters are fields: `{user.initial}` reads `get initial()`
- values with a `toJSON` method, like `Date`, which show as what they turn into: `2026-01-02T00:00:00.000Z`

A page renders exactly as `hypra build` writes it with the same data as JSON; the tests compare the two on
thousands of generated pages. One difference comes from JavaScript itself: it has one kind of number, so `1.0`
from JSON shows as `1`, where the compiler's static build writes `1.0`.

## TypeScript

The package has types, and the compiler writes a `.d.mts` next to every page, so `render`, `negotiate` and
the output are typed:

```ts
import product from "./views/dist/product.mjs";
import type { Output } from "hypra/runtime";

const output: Output = product.render({ product });
```

## Building every page

Give `build`, `check` and `fmt` a folder, and they take every `.hypra` file in it. Pages keep their place
relative to that folder: `views/shop/cart.hypra` becomes `views/dist/shop/cart.mjs`. Files with only components
and layouts are built into the pages that import them:

```json
{
  "scripts": {
    "build:views": "hypra build views --target js --out views/dist",
    "check:views": "hypra check views",
    "fmt:views": "hypra fmt --check views"
  }
}
```

## Rebuilding

`product.sources.json` lists the `.hypra` files a page was built from, so a build script or a file watcher knows
which pages to rebuild when one of them changes.

## License

MIT OR Apache-2.0
