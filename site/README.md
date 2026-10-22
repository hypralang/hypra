# hypralang.dev

The Hypra website, written in Hypra. The pages are in `pages/`; their content comes from the Markdown docs in the
repository, and the landing page shows real output of the compiler.

```console
$ npm ci
$ npm run build      # builds the compiler, then the site into dist/
```

Every page is also available as Markdown at `index.md`, and `llms.txt` lists them for AI agents.
