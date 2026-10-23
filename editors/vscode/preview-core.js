// The parts of the preview that don't need VS Code: rendering a page, finding its data, and the preview's HTML.
const { execFile } = require("node:child_process");
const { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } = require("node:fs");
const { tmpdir } = require("node:os");
const { basename, dirname, extname, join, resolve } = require("node:path");
const { pathToFileURL } = require("node:url");

/** Whether a file has a page, rather than only layouts and components that pages bring in with `use`. */
function hasPage(source) {
  return /^\s*page\s*\(/m.test(source);
}

/** The files a file brings in with `use`, as full paths. */
function uses(file, source) {
  return [...source.matchAll(/^\s*use\s+"([^"]+)"/gm)].map((m) => resolve(dirname(file), m[1]));
}

/**
 * The page that shows a file: the file itself if it has a page, else a page that uses it, directly or through
 * other layouts and components. `files` are the project's .hypra files as [path, source] pairs.
 * Undefined when no page uses it.
 */
function pageFor(file, files) {
  // the page closest to the top of the project, an index page first, so a layout shows in the home page
  const depth = (path) => path.split(/[\\/]/).length;
  const order = ([a], [b]) =>
    depth(a) - depth(b) || (basename(b, ".hypra") === "index") - (basename(a, ".hypra") === "index") || a.localeCompare(b);
  const sources = new Map([...files].sort(order));
  if (hasPage(sources.get(file) ?? readFileSync(file, "utf8"))) return file;
  const seen = new Set([file]);
  let found = [file];
  while (found.length > 0) {
    const next = [];
    for (const [path, source] of sources) {
      if (seen.has(path) || !uses(path, source).some((used) => found.includes(used))) continue;
      if (hasPage(source)) return path;
      seen.add(path);
      next.push(path);
    }
    found = next;
  }
  return undefined;
}

/** What the preview says about a file that no page uses yet. */
function noPage(file) {
  const name = basename(file);
  return (
    `${name} has layouts and components, not a page. A page brings them in with use, with the path from the ` +
    `page's folder; for a page next to the ${basename(dirname(file))} folder:\n\n` +
    `  use "${basename(dirname(file))}/${name}"\n\n` +
    `Once a page uses ${name}, this preview shows that page, with your changes to ${name} in it.`
  );
}

/** The data a page is previewed with: `product.json` next to `product.hypra`, if there is one. */
function dataFile(file) {
  const data = join(dirname(file), `${basename(file, extname(file))}.json`);
  return existsSync(data) ? data : undefined;
}

/**
 * Compiles a page to JavaScript with `hypra build --target js` and renders it with the runtime from the `hypra`
 * npm package (at `runtime`), the way a Node server would. Data the page leaves out renders empty, so a page
 * previews without data too. Resolves to its HTML, Markdown and JSON, or to what went wrong.
 */
function build(compiler, file, runtime) {
  const out = mkdtempSync(join(tmpdir(), "hypra-preview-"));
  const data = dataFile(file);
  const stem = basename(file, extname(file));
  return new Promise((resolve) => {
    execFile(compiler, ["build", file, "--target", "js", "--out", out], { cwd: dirname(file) }, (error, _stdout, stderr) => {
      // a code that isn't a number means the compiler didn't start
      if (error && typeof error.code !== "number") return resolve({ problem: `Can't run ${compiler}: ${error.message}` });
      if (error) return resolve({ problem: stderr, data });
      resolve(render(join(out, `${stem}.mjs`), data, runtime));
    });
  }).finally(() => rmSync(out, { recursive: true, force: true }));
}

async function render(module, data, runtime) {
  try {
    // the module imports "hypra/runtime"; point it at the runtime file, and load it fresh each time
    const code = readFileSync(module, "utf8").replace('from "hypra/runtime"', `from ${JSON.stringify(pathToFileURL(runtime).href)}`);
    const preview = module.replace(/\.mjs$/, ".preview.mjs");
    writeFileSync(preview, code);
    const page = (await import(`${pathToFileURL(preview).href}?${Date.now()}`)).default;
    const output = page.render(data ? JSON.parse(readFileSync(data, "utf8")) : {});
    const json = JSON.stringify(JSON.parse(output.negotiate("application/json").body), null, 2);
    return { html: output.html, markdown: output.markdown, json, data };
  } catch (e) {
    return { problem: e.message, data };
  }
}

/** The folder a site's paths start from: the nearest one above the page with a `public` folder, else `top`. */
function siteRoot(file, top) {
  for (let dir = dirname(file); dir.startsWith(top); dir = dirname(dir)) {
    if (existsSync(join(dir, "public"))) return dir;
    if (dir === dirname(dir)) break;
  }
  return top;
}

/**
 * Points the page's site paths (`/styles.css`) at files in the project: `public/styles.css` if it exists, else
 * `styles.css` at the project's root. `toUri` turns a file path into a URL the preview can load.
 */
function siteFiles(html, root, toUri) {
  const local = (path) => {
    const [file, rest = ""] = path.split(/(?=[?#])/);
    for (const base of [join(root, "public"), root]) {
      if (existsSync(join(base, file))) return toUri(join(base, file)) + rest;
    }
    return path;
  };
  return html
    .replace(/\b(src|href|poster)="(\/(?!\/)[^"]*)"/g, (_, name, path) => `${name}="${local(path)}"`)
    .replace(/\bsrcset="([^"]*)"/g, (_, set) =>
      `srcset="${set.replace(/(^|,\s*)(\/(?!\/)[^\s,]+)/g, (_, sep, path) => sep + local(path))}"`,
    );
}

const escape = (text) => text.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

/** What the preview may load: the project's files (`csp`), and anything over https, like a CDN's Tailwind. */
function policy(csp) {
  return [
    "default-src 'none'",
    `img-src ${csp} https: data:`,
    `media-src ${csp} https:`,
    `font-src ${csp} https: data:`,
    `style-src ${csp} https: 'unsafe-inline'`,
    `script-src ${csp} https: 'unsafe-inline'`,
  ].join("; ");
}

/**
 * The page as a browser shows it. It's the webview's own document, the way VS Code previews Markdown, so it can
 * load the project's files; the extension doesn't listen to it, so its scripts can't do anything in VS Code.
 */
function pageView(html, csp) {
  const meta = `<meta http-equiv="Content-Security-Policy" content="${policy(csp)}">`;
  return /<head>/.test(html) ? html.replace("<head>", `<head>\n  ${meta}`) : meta + html;
}

/** Markdown, JSON or the compiler's messages, as text in the editor's colours. */
function textView(text, { csp, note = "", problem = false }) {
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="${policy(csp)}">
<style>
  body { margin: 0; padding: 12px 16px; font: 13px var(--vscode-font-family); color: var(--vscode-foreground); background: var(--vscode-editor-background); }
  p { opacity: 0.75; margin: 0 0 12px; }
  pre { margin: 0; font: 12px/1.5 var(--vscode-editor-font-family); white-space: pre-wrap; }
  .problem pre { color: var(--vscode-errorForeground); }
</style>
</head>
<body${problem ? ' class="problem"' : ""}>
${note ? `<p>${escape(note)}</p>` : ""}<pre>${escape(text)}</pre>
</body>
</html>`;
}

/** What the preview says about its data. */
function dataNote(result, file) {
  return result.data
    ? `With data from ${basename(result.data)}.`
    : `Without data: add ${basename(file, extname(file))}.json next to the page to fill it in.`;
}

module.exports = { build, dataFile, dataNote, hasPage, noPage, pageFor, pageView, siteFiles, siteRoot, textView, uses };
