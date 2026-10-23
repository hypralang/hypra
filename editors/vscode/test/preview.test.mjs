// Checks the preview's parts that don't need VS Code: data, site paths, the view, and a real build when the
// compiler is built in this repository.
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { join } from "node:path";
import assert from "node:assert/strict";

const { build, dataFile, dataNote, hasPage, noPage, pageFor, pageView, siteFiles, siteRoot, textView } = createRequire(import.meta.url)("../preview-core.js");

const site = mkdtempSync(join(tmpdir(), "hypra-site-"));
for (const file of ["public/styles.css", "public/photos/a.jpg", "logo.svg", "pages/shop/cart.hypra", "pages/shop/cart.json"]) {
  mkdirSync(join(site, file, ".."), { recursive: true });
  writeFileSync(join(site, file), "");
}

// data next to the page
assert.equal(dataFile(join(site, "pages/shop/cart.hypra")), join(site, "pages/shop/cart.json"));
assert.equal(dataFile(join(site, "pages/index.hypra")), undefined);

// the site's root is the nearest folder with public/, else the workspace folder
assert.equal(siteRoot(join(site, "pages/shop/cart.hypra"), tmpdir()), site);
const bare = mkdtempSync(join(tmpdir(), "hypra-bare-"));
assert.equal(siteRoot(join(bare, "page.hypra"), bare), bare);

// site paths point at the project's files; others stay as they are
const uri = (path) => `vscode-resource:${path}`;
const html = siteFiles(
  '<link rel="stylesheet" href="/styles.css?v=2"><img src="/photos/a.jpg" srcset="/photos/a.jpg 1x, /photos/b.jpg 2x">' +
    '<img src="/logo.svg"><a href="/about">About</a><script src="//cdn.example/x.js"></script><a href="https://x.dev/">x</a>',
  site,
  uri,
);
assert.match(html, new RegExp(`href="vscode-resource:${join(site, "public/styles.css")}\\?v=2"`));
assert.match(html, new RegExp(`src="vscode-resource:${join(site, "public/photos/a.jpg")}"`));
assert.match(html, new RegExp(`srcset="vscode-resource:${join(site, "public/photos/a.jpg")} 1x, /photos/b.jpg 2x"`));
assert.match(html, new RegExp(`src="vscode-resource:${join(site, "logo.svg")}"`));
assert.match(html, /href="\/about"/, "a page of the site that isn't a file stays a link");
assert.match(html, /src="\/\/cdn.example\/x.js"/);

// the page is shown as it is, with a policy for what it may load; text is escaped
const page = pageView("<!doctype html>\n<html>\n<head>\n  <title>x</title>", "vscode-resource:");
assert.match(page, /<head>\n  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src vscode-resource: https: data:/);
const text = textView("<b>&</b>", { csp: "x", note: "With data from a.json." });
assert.match(text, /<pre>&lt;b&gt;&amp;&lt;\/b&gt;<\/pre>/);
assert.match(text, /<p>With data from a\.json\.<\/p>/);
assert.match(textView("error", { csp: "x", problem: true }), /<body class="problem">/);
assert.equal(dataNote({}, "/x/home.hypra"), "Without data: add home.json next to the page to fill it in.");
assert.equal(dataNote({ data: "/x/home.json" }, "/x/home.hypra"), "With data from home.json.");

// a layout or component is shown in a page that uses it, directly or through another component
const views = "/app/views";
const files = [
  [`${views}/layouts/game.hypra`, 'layout Game(lang: "en") {\n  main { slot }\n}\n'],
  [`${views}/components/card.hypra`, 'use "../layouts/game.hypra"\ncomponent Card(t: Text) { p "{t}" }\n'],
  [`${views}/components/unused.hypra`, "component Unused() { p \"x\" }\n"],
  [`${views}/shop/list.hypra`, 'use "../components/card.hypra"\npage(title: "List") { h1 "List" }\n'],
  [`${views}/home.hypra`, '// the home page\nuse "layouts/game.hypra"\n\npage(layout: Game, title: "Home") { h1 "Home" }\n'],
];
assert.equal(hasPage(files[4][1]), true);
assert.equal(hasPage(files[0][1]), false);
assert.equal(pageFor(`${views}/home.hypra`, files), `${views}/home.hypra`, "a page shows itself");
assert.equal(pageFor(`${views}/layouts/game.hypra`, files), `${views}/home.hypra`, "the page closest to the top");
const starter = [
  ["/s/layouts/site.hypra", "layout Site() { slot }\n"],
  ["/s/pages/coffee/show.hypra", 'use "../../layouts/site.hypra"\npage(title: "C") { h1 "C" }\n'],
  ["/s/pages/about.hypra", 'use "../layouts/site.hypra"\npage(title: "A") { h1 "A" }\n'],
  ["/s/pages/index.hypra", 'use "../layouts/site.hypra"\npage(title: "H") { h1 "H" }\n'],
];
assert.equal(pageFor("/s/layouts/site.hypra", starter), "/s/pages/index.hypra", "an index page before others");
assert.equal(pageFor(`${views}/components/card.hypra`, files), `${views}/shop/list.hypra`);
assert.equal(pageFor(`${views}/components/unused.hypra`, files), undefined);
assert.match(noPage(`${views}/layouts/game.hypra`), /use "layouts\/game\.hypra"/);

// a real build, when the compiler is built here, rendered with the runtime in js/
const compiler = new URL("../../../target/debug/hypra", import.meta.url).pathname;
const runtime = new URL("../../../js/runtime.js", import.meta.url).pathname;
if (existsSync(compiler)) {
  const examples = new URL("../../../examples/", import.meta.url).pathname;
  const product = await build(compiler, join(examples, "product.hypra"), runtime);
  assert.ok(product.html.includes("<h1>Runner X</h1>"), product.problem ?? "built with product.json");
  assert.equal(product.data, join(examples, "product.json"));
  assert.match(product.markdown, /^---\ntitle:/);
  assert.match(product.json, /^\{\n  "title"/);

  // without data, the page still renders, with what it needs left empty
  const starter = new URL("../../../starter/pages/index.hypra", import.meta.url).pathname;
  const home = await build(compiler, starter, runtime);
  assert.ok(home.html?.includes("Coffee that tastes like where it grew"), home.problem);
  assert.equal(home.data, undefined);

  const broken = await build(compiler, join(examples, "broken.hypra"), runtime);
  assert.match(broken.problem, /error\[/);
  const missing = await build("/nonexistent/hypra", join(examples, "product.hypra"), runtime);
  assert.match(missing.problem, /^Can't run/);
  console.log("preview ok");
} else {
  console.log("preview ok (no compiler built, so no real build)");
}
