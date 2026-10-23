# Roadmap

What's planned for Hypra, roughly in order. Nothing here has a date. If something matters to you, or is missing,
say so in an [issue](https://github.com/hypralang/hypra/issues): that moves it up.

## Next

- **JavaScript target.** `hypra build --target js` and an npm package, so pages render at request time in Node,
  Express, Hono, Next and others, the way the PHP target does in PHP and Laravel today.

## Later

- A Vite plugin: `import page from "./product.hypra"` in Vite, Astro and SvelteKit

- A playground on hypralang.dev: write Hypra in the browser and see every output
- Link and alt text checks in more languages
- The extension on the Visual Studio Marketplace; it's on Open VSX today

## Maybe

- A Python target, for Django, Flask and FastAPI
- Ready-made setups for more editors: Zed, Helix, JetBrains
- Completion in the language server

## Not planned

- **A JavaScript framework.** Hypra writes HTML and works with whatever runs on the page: htmx, Alpine.js, React or
  Vue islands, web components.
- **Rendering in the browser.** Pages are compiled ahead of time or on the server.
- **Looser parsing.** A page with a mistake doesn't build, the way a program with a type error doesn't compile;
  `allow` switches a rule off for one element, with a reason.
