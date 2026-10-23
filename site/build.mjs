// Builds hypralang.dev into dist/. The pages are Hypra; their content comes from the repository's Markdown docs and
// from running the compiler on a sample, so the site always shows what the current version does.
import { execFileSync, spawnSync } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Marked } from "marked";
import { createHighlighter } from "shiki";

const site = dirname(fileURLToPath(import.meta.url));
const repo = resolve(site, "..");
const dist = join(site, "dist");
const origin = "https://hypralang.dev";
const github = "https://github.com/hypralang/hypra";

// the docs, in the order of the sidebar; the slug is the folder under /docs/
const docs = [
  {
    slug: "",
    file: "docs/spec.md",
    nav: "Language",
    description: "The complete Hypra reference: elements, data, components, layouts, forms, actions and every check.",
  },
  {
    slug: "laravel",
    file: "php/laravel/README.md",
    nav: "Laravel",
    description: "Use .hypra files as Laravel views: install the compiler, render pages, answer agents in Markdown and JSON.",
  },
  {
    slug: "php",
    file: "php/runtime/README.md",
    nav: "PHP runtime",
    description: "Render compiled Hypra pages from any PHP app with the small hypra/runtime package.",
  },
  {
    slug: "vscode",
    file: "editors/vscode/README.md",
    nav: "VS Code",
    description: "Hypra in VS Code and Cursor: errors as you type, go to definition, references, rename, hover help and formatting.",
  },
  {
    slug: "security",
    file: "SECURITY.md",
    nav: "Security",
    description: "How Hypra keeps data from changing what a page does, what it checks, and how to report a problem.",
  },
];

// the guides, in the order of the sidebar; each solves one problem, with and without Hypra
const guides = [
  {
    section: "guides",
    slug: "markdown-for-llms-in-laravel",
    file: "docs/guides/markdown-for-llms-in-laravel.md",
    nav: "Markdown for LLMs in Laravel",
    description: "Answer AI agents in Markdown from the same URL as your HTML, with Laravel middleware or with Hypra.",
  },
  {
    section: "guides",
    slug: "accessibility-checks-at-compile-time",
    file: "docs/guides/accessibility-checks-at-compile-time.md",
    nav: "Accessibility checks at compile time",
    description: "Catch missing alt text, unlabeled fields, empty buttons and skipped headings before a page ships.",
  },
  {
    section: "guides",
    slug: "webmcp-tools-for-your-website",
    file: "docs/guides/webmcp-tools-for-your-website.md",
    nav: "WebMCP tools for your website",
    description: "Turn your forms and buttons into tools AI agents can call, with WebMCP attributes and JSON Schemas.",
  },
];

// the rules: a page for every code the compiler reports, from docs/rules, grouped by the code's prefix
const groups = {
  A11Y: "Accessibility",
  SEO: "Search engines",
  SEC: "Security",
  FORM: "Forms",
  ATTR: "Attributes",
  SEM: "Structure",
  TYPE: "Data",
  COMP: "Components",
  LAYOUT: "Layouts",
  IMPORT: "Imports",
  ALLOW: "allow",
  LEX: "Syntax",
  PARSE: "Syntax",
};
const rules = readdirSync(join(repo, "docs/rules"))
  .filter((name) => name.endsWith(".md"))
  .sort()
  .map((name) => {
    const code = name.replace(/\.md$/, "");
    return { section: "rules", slug: code.toLowerCase(), file: `docs/rules/${name}`, nav: code, code, group: groups[code.split("-")[0]] };
  });

execFileSync("cargo", ["build", "--release", "--quiet"], { cwd: repo, stdio: "inherit" });
const hypra = join(repo, "target/release/hypra");
const version = execFileSync(hypra, ["--version"], { encoding: "utf8" }).trim().replace("hypra ", "");

const highlighter = await createHighlighter({
  themes: ["github-light", "github-dark"],
  langs: [
    "html", "markdown", "json", "shellscript", "php", "javascript", "css", "yaml",
    { ...JSON.parse(readFileSync(join(repo, "editors/vscode/syntaxes/hypra.tmLanguage.json"), "utf8")), name: "hypra" },
  ],
});
const aliases = { console: "shellscript", sh: "shellscript", bash: "shellscript", text: "text", "": "text" };
/** Highlighted code in a block that gets a copy button; `wrap` breaks long lines instead of scrolling, and `label`
 * names the block, e.g. "Output" for what the compiler wrote. */
const highlight = (code, lang, wrap = false, label = "") => {
  const name = aliases[lang] ?? lang;
  const html = highlighter.codeToHtml(code.replace(/\n$/, ""), {
    lang: highlighter.getLoadedLanguages().includes(name) ? name : "text",
    themes: { light: "github-light", dark: "github-dark" },
    defaultColor: "light",
  });
  return `<div class="codeblock${wrap ? " wrap" : ""}"${label ? ` data-label="${label}"` : ""}>${html}</div>`;
};
const escape = (text) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
/** The compiler's report, colored the way a terminal shows it. Its lines don't wrap, so the `^^^` under a line keep
 * pointing at the right place; on a narrow screen the block scrolls instead. */
function terminal(report, label = "") {
  const lines = escape(report).split("\n").map((line) => {
    if (/^error\[/.test(line)) return `<span class="e">${line}</span>`;
    if (/^warning\[/.test(line)) return `<span class="w">${line}</span>`;
    if (/^\s*= help/.test(line)) return `<span class="h">${line}</span>`;
    if (/^\s*(--&gt;|\||\d+ \|)/.test(line)) return `<span class="m">${line}</span>`;
    return line;
  });
  return `<div class="codeblock terminal"${label ? ` data-label="${label}"` : ""}><pre><code>${lines.join("\n")}</code></pre></div>`;
}

/** Runs the compiler on a page with data and fails the build on any error or warning. */
function build(page, data, out) {
  const dataFile = join(mkdtempSync(join(tmpdir(), "hypra-site-")), "data.json");
  writeFileSync(dataFile, JSON.stringify(data));
  const result = spawnSync(hypra, ["build", page, "--data", dataFile, "--out", out], { encoding: "utf8" });
  if (result.status !== 0 || !result.stderr.includes("0 errors, 0 warnings")) {
    throw new Error(`${relative(site, page)} didn't build cleanly:\n${result.stderr}`);
  }
}

// --- the landing page: a sample page, compiled for real

const sample = {
  source: `page(title: "Runner X", description: "A light running shoe for long distances, with free returns.") {
  props(product: Product)
  h1 "{product.name}"
  img(product.photo, alt: "{product.name}, side view")
  p "{product.summary}"
  action("cart.add", id: product.id) "Add to cart"
}
`,
  data: { product: { id: 42, name: "Runner X", photo: "/img/runner-x.jpg", summary: "220 g, made for long distances." } },
  broken: `page(title: "Shoes") {
  h1 "Shoes"
  img("/shoe.jpg")
  buton "Buy"
}
`,
};
const work = mkdtempSync(join(tmpdir(), "hypra-sample-"));
writeFileSync(join(work, "product.hypra"), sample.source);
writeFileSync(join(work, "broken.hypra"), sample.broken);
build(join(work, "product.hypra"), sample.data, join(work, "out"));
const read = (file) => readFileSync(join(work, "out", file), "utf8");
const body = read("product.html").split("<body>\n")[1].split("</body>")[0].replace(/^ {2}/gm, "");
const check = spawnSync(hypra, ["check", "broken.hypra"], { cwd: work, encoding: "utf8" });

const shell = (lines) => highlight(lines.join("\n"), "shellscript", true);
const landing = {
  version,
  sample: {
    source: highlight(sample.source, "hypra", true),
    html: highlight(body, "html", true),
    markdown: highlight(read("product.md"), "markdown", true),
    tools: highlight(JSON.stringify(JSON.parse(read("product.webmcp.json")), null, 2), "json", true),
    errors: terminal(check.stderr.trim()),
    install_cli: shell(["cargo install hypra"]),
    install_laravel: shell(["composer require hypra/laravel", "php artisan hypra:install"]),
    install_vscode: shell([
      "# Cursor, VSCodium and other editors that install from Open VSX",
      "cursor --install-extension hypralang.hypra",
      "# VS Code: the extension from the release",
      `curl -LO ${github}/releases/download/v${version}/hypra-${version}.vsix`,
      `code --install-extension hypra-${version}.vsix`,
    ]),
  },
};

// --- the docs: Markdown from the repository

const slug = (text) => text.toLowerCase().replace(/<[^>]+>|`/g, "").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
const pageOf = (file) => [...docs, ...guides, ...rules].find((d) => d.file === file);
const urlOf = (doc) => `/${doc.section ?? "docs"}/${doc.slug ? `${doc.slug}/` : ""}`;

/** Where a link in a doc goes on the site: another doc, or the file on GitHub. With `absolute`, links within the
 * site get the origin too, for Markdown read outside of it. */
function linkOf(doc, href, absolute = false) {
  if (/^([a-z]+:|#)/.test(href)) return href;
  const [path, hash = ""] = href.split("#");
  const target = relative(repo, resolve(repo, dirname(doc.file), path));
  // a link to a folder means its README
  const other = pageOf(target) ?? pageOf(join(target, "README.md"));
  const url = other ? `${absolute ? origin : ""}${urlOf(other)}` : `${github}/${target.includes(".") ? "blob" : "tree"}/main/${target}`;
  return `${url}${hash ? `#${hash}` : ""}`;
}

/** Converts a doc to HTML: highlighted code, headings with ids, links to other docs within the site. */
function render(doc, markdown) {
  const toc = [];
  const failures = doc.code ? ruleChecks(doc) : [];
  const marked = new Marked({
    renderer: {
      code({ text, lang }) {
        // ```html output marks what the compiler writes; tests/guides.rs checks it is
        const [language, kind, ...rest] = (lang ?? "").split(/\s+/);
        if (language === "console" && kind === "output") return terminal(text, "Output");
        // a rule's examples: what `hypra check` says follows each one that fails
        if (kind === "fails") return highlight(text, language, true) + terminal(failures.shift(), "hypra check");
        if (kind === "file") return highlight(text, language, true, rest.join(" "));
        if (kind === "data") return highlight(text, language, true, "Data");
        return highlight(text, language, true, kind === "output" ? "Output" : "");
      },
      heading({ tokens, depth, text }) {
        const id = slug(text);
        if (depth === 2) toc.push({ id, text: text.replace(/`/g, "") });
        return `<h${depth} id="${id}">${this.parser.parseInline(tokens)}</h${depth}>\n`;
      },
      link({ href, tokens }) {
        return `<a href="${linkOf(doc, href)}">${this.parser.parseInline(tokens)}</a>`;
      },
    },
  });
  // every rule code's row gets an id, so editors can link an error to where it's explained: /docs/#a11y-001
  const html = marked
    .parse(markdown)
    .replace(/<tr>\n<td>(?:<a [^>]+>)?<code>([A-Z0-9]+-\d{3})<\/code>/g, (row, code) => row.replace("<tr>", `<tr id="${code.toLowerCase()}">`));
  return { html, toc };
}

/** What `hypra check` says about each failing example of a rule, in order. The examples import the rule's other
 * files and are checked with the data before them; tests/rule_docs.rs checks that they fail as they should. */
function ruleChecks(doc) {
  const dir = mkdtempSync(join(tmpdir(), "hypra-rule-"));
  const blocks = new Marked().lexer(doc.markdown).filter((token) => token.type === "code");
  for (const { lang, text } of blocks) {
    const [, kind, path] = lang.split(/\s+/);
    if (kind !== "file") continue;
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    writeFileSync(join(dir, path), `${text}\n`);
  }
  let data = [];
  const reports = [];
  for (const { lang, text } of blocks) {
    const [language, kind] = lang.split(/\s+/);
    if (language === "json" && kind === "data") {
      writeFileSync(join(dir, "data.json"), text);
      data = ["--data", "data.json"];
    }
    if (kind !== "fails") continue;
    writeFileSync(join(dir, "page.hypra"), `${text}\n`);
    const result = spawnSync(hypra, ["check", "page.hypra", ...data], { cwd: dir, encoding: "utf8" });
    if (result.stdout || !result.stderr.includes(`[${doc.code}]`)) {
      throw new Error(`${doc.file}: an example doesn't fail with ${doc.code}:\n${result.stdout}${result.stderr}`);
    }
    reports.push(result.stderr.trim());
  }
  rmSync(dir, { recursive: true, force: true });
  return reports;
}

rmSync(dist, { recursive: true, force: true });
cpSync(join(site, "public"), dist, { recursive: true });
build(
  join(site, "pages/index.hypra"),
  { ...landing, guides: guides.map((g) => ({ url: urlOf(g), title: g.nav, description: g.description })) },
  dist,
);

const load = (doc) => {
  const markdown = readFileSync(join(repo, doc.file), "utf8");
  const [, title] = markdown.match(/^# (.+)$/m);
  const rest = markdown.replace(/^# .+\n/m, "");
  return { ...doc, title, markdown, rest };
};
const loaded = docs.map(load);
const loadedGuides = guides.map(load);
const loadedRules = rules.map(load).map((rule) => {
  const what = rule.title.replace(/^[^:]+: /, "");
  return { ...rule, what, description: `Hypra rule ${rule.code}, ${what}: why it's checked, a page that breaks it and the fix.` };
});
const allRules = { url: "/rules/", title: "All rules", current: false };
for (const doc of [...loaded, ...loadedGuides, ...loadedRules]) {
  const { html, toc } = render(doc, doc.rest);
  const out = join(dist, doc.section ?? "docs", doc.slug);
  const links = (list) => list.map((d) => ({ url: urlOf(d), title: d.nav, current: d === doc }));
  // a rule's page lists the other rules of its group; the docs and guides link to all of them
  const rulesSection = doc.code
    ? { title: `${doc.group} rules`, pages: [allRules, ...links(loadedRules.filter((r) => r.group === doc.group))] }
    : { title: "Rules", pages: [allRules] };
  build(
    join(site, "pages/docs/index.hypra"),
    {
      doc: { title: doc.title, description: doc.description, url: urlOf(doc), html, toc, markdown: "index.md" },
      sections: [{ title: "Docs", pages: links(loaded) }, { title: "Guides", pages: links(loadedGuides) }, rulesSection],
    },
    out,
  );
  // agents get the docs as they were written
  writeFileSync(join(out, "index.md"), doc.markdown);
}

// --- the list of guides
build(
  join(site, "pages/guides.hypra"),
  { guides: loadedGuides.map((g) => ({ url: urlOf(g), title: g.title, description: g.description })) },
  join(dist, "guides"),
);

// --- the list of rules, by group
build(
  join(site, "pages/rules.hypra"),
  {
    groups: [...new Set(Object.values(groups))].map((title) => ({
      title,
      rules: loadedRules.filter((r) => r.group === title).map((r) => ({ url: urlOf(r), code: r.code, what: r.what })),
    })),
  },
  join(dist, "rules"),
);

// the compiler names its outputs after the page; these are their folders' index
for (const page of ["guides", "rules"]) {
  for (const ext of ["html", "md", "json", "webmcp.json"]) {
    renameSync(join(dist, page, `${page}.${ext}`), join(dist, page, `index.${ext}`));
  }
}

// --- llms.txt: where agents find the docs as Markdown
writeFileSync(
  join(dist, "llms.txt"),
  `# Hypra\n\n> Hypra is a strict markup language that compiles to HTML for people and to Markdown, JSON and WebMCP tools for AI agents.\n\n## Docs\n\n` +
    loaded.map((d) => `- [${d.title}](${origin}${urlOf(d)}index.md): ${d.description}`).join("\n") +
    "\n\n## Guides\n\n" +
    loadedGuides.map((d) => `- [${d.title}](${origin}${urlOf(d)}index.md): ${d.description}`).join("\n") +
    "\n\n## Rules\n\nEvery message from the compiler has a code, and every code a page: why it's checked, a page that breaks it and the fix.\n\n" +
    loadedRules.map((d) => `- [${d.title}](${origin}${urlOf(d)}index.md)`).join("\n") +
    "\n",
);

// --- for search engines: every page, and where the list is
const pages = [
  `${origin}/`,
  ...loaded.map((d) => `${origin}${urlOf(d)}`),
  `${origin}/guides/`,
  ...loadedGuides.map((d) => `${origin}${urlOf(d)}`),
  `${origin}/rules/`,
  ...loadedRules.map((d) => `${origin}${urlOf(d)}`),
];
writeFileSync(
  join(dist, "sitemap.xml"),
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n` +
    pages.map((url) => `  <url><loc>${url}</loc></url>`).join("\n") +
    "\n</urlset>\n",
);
writeFileSync(join(dist, "robots.txt"), `User-agent: *\nAllow: /\n\nSitemap: ${origin}/sitemap.xml\n`);

// --- the stylesheet, from the classes in the Hypra pages
execFileSync(join(site, "node_modules/.bin/tailwindcss"), ["-i", join(site, "styles.css"), "-o", join(dist, "site.css"), "--minify"], {
  cwd: site,
  stdio: ["ignore", "ignore", "inherit"],
});

mkdirSync(dist, { recursive: true });
console.log(`built hypralang.dev ${version} into ${relative(process.cwd(), dist) || "."}`);
