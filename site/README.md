# hypralang.dev

The Hypra website, written in Hypra. The pages are in `pages/`; their content comes from the Markdown docs in the
repository, and the landing page shows real output of the compiler.

```console
$ rustup target add wasm32-unknown-unknown   # once, for the playground's compiler
$ npm ci
$ npm run build      # builds the compiler, then the site into dist/
```

The playground at `/play/` runs the compiler in the browser: `wasm/` builds it as WebAssembly, and `play/` has the
page's script and its examples, which the build checks.

Every page is also available as Markdown at `index.md`, and `llms.txt` lists them for AI agents.
