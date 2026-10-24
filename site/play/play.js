// The playground on hypralang.dev: the compiler as WebAssembly, an editor for each file and for the data, and what
// the compiler makes of them. build.mjs bundles this with esbuild and puts the examples in EXAMPLES.
import { indentWithTab } from "@codemirror/commands";
import { json } from "@codemirror/lang-json";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { setDiagnostics } from "@codemirror/lint";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";
import { basicSetup } from "codemirror";
import { hypra } from "./hypra-lang.js";

/* global EXAMPLES */
const examples = EXAMPLES;
const $ = (id) => document.getElementById(id);

// --- the compiler: requests and answers are JSON in the module's memory (site/wasm/src/lib.rs)

const module = await WebAssembly.compileStreaming(fetch("/play/hypra.wasm"));
let compiler = await WebAssembly.instantiate(module);

function call(fn, request) {
  const bytes = new TextEncoder().encode(JSON.stringify(request));
  try {
    // `input` can grow the memory, so the buffer is looked up after it
    const ptr = compiler.exports.input(bytes.length);
    new Uint8Array(compiler.exports.memory.buffer, ptr, bytes.length).set(bytes);
    const len = compiler.exports[fn]();
    return JSON.parse(new TextDecoder().decode(new Uint8Array(compiler.exports.memory.buffer, compiler.exports.output(), len)));
  } catch (error) {
    // a crash leaves the module unusable: start a new one for the next try
    compiler = new WebAssembly.Instance(module);
    return { ok: false, crashed: String(error), diagnostics: [] };
  }
}

// --- colors for the code, from the site's stylesheet, so they follow dark mode

const highlight = HighlightStyle.define([
  { tag: [t.keyword, t.operator], color: "var(--hl-keyword)" },
  { tag: [t.string, t.special(t.string)], color: "var(--hl-string)" },
  { tag: [t.tagName, t.heading], color: "var(--hl-tag)", fontWeight: "600" },
  { tag: [t.attributeName, t.propertyName], color: "var(--hl-attribute)" },
  { tag: [t.typeName, t.className], color: "var(--hl-type)" },
  { tag: [t.number, t.bool, t.atom], color: "var(--hl-number)" },
  { tag: [t.special(t.variableName), t.escape], color: "var(--hl-variable)" },
  { tag: [t.comment, t.meta, t.processingInstruction], color: "var(--hl-comment)", fontStyle: "italic" },
  { tag: [t.link, t.url], color: "var(--hl-string)", textDecoration: "underline" },
  { tag: t.strong, fontWeight: "700" },
  { tag: t.emphasis, fontStyle: "italic" },
]);
const theme = EditorView.theme({
  "&": { height: "100%", fontSize: "14px", backgroundColor: "var(--color-code)", color: "var(--color-ink)" },
  ".cm-scroller": { fontFamily: "var(--font-mono)", lineHeight: "1.6" },
  ".cm-gutters": { backgroundColor: "var(--color-code)", color: "var(--color-mute)", border: "none" },
  ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "color-mix(in srgb, var(--color-ink) 5%, transparent)" },
  ".cm-content": { caretColor: "var(--color-ink)" },
  "&.cm-focused .cm-cursor": { borderLeftColor: "var(--color-ink)" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
    backgroundColor: "color-mix(in srgb, var(--color-pass) 25%, transparent) !important",
  },
});
const looks = [theme, syntaxHighlighting(highlight)];

// --- the files: the page first, then the files it imports, then the data

let docs = [];
let current = 0;
let example = "";
let pristine = "";

const editor = new EditorView({ parent: $("editor") });
const stateFor = (name, text) =>
  EditorState.create({
    doc: text,
    extensions: [
      basicSetup,
      keymap.of([indentWithTab, { key: "Shift-Alt-f", run: () => (format(), true) }]),
      name.endsWith(".json") ? json() : hypra,
      looks,
      EditorView.updateListener.of((update) => update.docChanged && changed()),
      EditorView.contentAttributes.of({ "aria-label": name }),
    ],
  });

/** Starts over with these files and data. */
function open(files, data, name = "") {
  docs = [...files.map(([file, text]) => ({ name: file, state: stateFor(file, text) })), { name: "data.json", state: stateFor("data.json", data) }];
  current = 0;
  example = name;
  good = null;
  editor.setState(docs[0].state);
  pristine = JSON.stringify(snapshot());
  $("example").value = name;
  tabs();
  compile();
}

/** The files and data as they are now. */
function snapshot() {
  docs[current].state = editor.state;
  const files = docs.filter((d) => d.name !== "data.json").map((d) => [d.name, d.state.doc.toString()]);
  return { files, data: docs.at(-1).state.doc.toString() };
}

function show(index) {
  docs[current].state = editor.state;
  current = index;
  editor.setState(docs[index].state);
  tabs();
  marks();
}

/** The file tabs, with a button to add a file, and to remove every file but the page. */
function tabs() {
  const bar = $("files");
  bar.replaceChildren();
  docs.forEach((doc, i) => {
    const tab = document.createElement("button");
    tab.type = "button";
    tab.role = "tab";
    tab.textContent = doc.name;
    tab.ariaSelected = String(i === current);
    tab.addEventListener("click", () => show(i));
    const errors = last.diagnostics?.filter((d) => d.file === doc.name && d.severity === "error").length;
    if (errors) tab.dataset.errors = errors;
    bar.append(tab);
    if (i > 0 && doc.name !== "data.json") {
      const remove = document.createElement("button");
      remove.type = "button";
      remove.className = "remove";
      remove.ariaLabel = `Remove ${doc.name}`;
      remove.textContent = "×";
      remove.addEventListener("click", () => {
        if (!confirm(`Remove ${doc.name}?`)) return;
        docs[current].state = editor.state;
        docs.splice(i, 1);
        if (current === i) current = 0;
        else if (current > i) current -= 1;
        editor.setState(docs[current].state);
        tabs();
        compile();
      });
      bar.append(remove);
    }
  });
  const add = document.createElement("button");
  add.type = "button";
  add.className = "add";
  add.textContent = "+ File";
  add.addEventListener("click", addFile);
  bar.append(add);
}

function addFile() {
  const name = prompt("Name of the new file, e.g. card.hypra. Import it in the page with use \"card.hypra\".", "card.hypra");
  if (!name) return;
  if (!/^[a-z0-9_-]+(\/[a-z0-9_-]+)*\.hypra$/i.test(name) || docs.some((d) => d.name === name)) {
    alert(`${name} can't be added: use a new name that ends in .hypra, with letters, numbers, - and _.`);
    return;
  }
  const component = name.replace(/^.*\//, "").replace(/\.hypra$/, "").replace(/(^|[-_])([a-z0-9])/gi, (_, __, c) => c.toUpperCase());
  const text = `// in the page: use "${name}"\ncomponent ${/^[A-Z]/.test(component) ? component : "Card"}(title: Text) {\n  h2 "{title}"\n}\n`;
  docs[current].state = editor.state;
  docs.splice(docs.length - 1, 0, { name, state: stateFor(name, text) });
  show(docs.length - 2);
  compile();
}

// --- compiling, as you type

let last = { diagnostics: [] };
let good = null;
let timer = 0;

function changed() {
  clearTimeout(timer);
  timer = setTimeout(compile, 120);
}

function compile() {
  const { files, data } = snapshot();
  let parsed;
  try {
    parsed = JSON.parse(data || "{}");
  } catch (error) {
    last = { ok: false, diagnostics: [], dataError: error.message };
    report();
    return;
  }
  const started = performance.now();
  last = call("compile", { files: Object.fromEntries(files), data: parsed });
  last.ms = performance.now() - started;
  if (last.ok) good = last;
  report();
  remember(files, data);
}

/** The messages, the status line, the marks in the editor and the outputs, for the last compile. */
function report() {
  const list = last.diagnostics;
  const errors = list.filter((d) => d.severity === "error").length;
  const warnings = list.length - errors;
  const status = $("status");
  if (last.crashed) {
    status.textContent = "The compiler crashed on this page";
    status.dataset.state = "fail";
  } else if (last.dataError) {
    status.textContent = "data.json isn't valid JSON";
    status.dataset.state = "fail";
  } else {
    const count = (n, word) => `${n} ${word}${n === 1 ? "" : "s"}`;
    status.textContent = `${count(errors, "error")}, ${count(warnings, "warning")} · ${last.ms < 1 ? "<1" : Math.round(last.ms)} ms`;
    status.dataset.state = errors ? "fail" : warnings ? "warn" : "pass";
  }
  messages();
  marks();
  tabs();
  output();
}

const escape = (text) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** Each message as `hypra check` prints it; its location jumps to the place in the editor. */
function messages() {
  const box = $("problems");
  if (last.crashed) {
    box.innerHTML = `<pre><code><span class="e">The compiler stopped with an error: ${escape(last.crashed)}</span>\nThat's a bug in Hypra. Please <a href="https://github.com/hypralang/hypra/issues/new">open an issue</a> with the link from “Copy link”.</code></pre>`;
    return;
  }
  if (last.dataError) {
    box.innerHTML = `<pre><code><span class="e">data.json: ${escape(last.dataError)}</span></code></pre>`;
    return;
  }
  if (!last.diagnostics.length) {
    box.innerHTML = `<pre><code><span class="h">No problems: the compiler wrote every output.</span></code></pre>`;
    return;
  }
  const pre = document.createElement("pre");
  const code = document.createElement("code");
  for (const d of last.diagnostics) {
    const lines = escape(d.text.trimEnd()).split("\n").map((line) => {
      if (/^error\[/.test(line)) return `<span class="e">${line}</span>`;
      if (/^warning\[/.test(line)) return `<span class="w">${line}</span>`;
      if (/^\s*= help/.test(line)) return `<span class="h">${line}</span>`;
      if (/^\s*= docs/.test(line)) return `<span class="m">${line.replace(/https:\S+/, (url) => `<a href="${url}">${url}</a>`)}</span>`;
      if (/^\s*--&gt;/.test(line)) return `<button type="button" class="jump">${line}</button>`;
      if (/^\s*(\||\d+ \|)/.test(line)) return `<span class="m">${line}</span>`;
      return line;
    });
    const item = document.createElement("div");
    item.innerHTML = `${lines.join("\n")}\n\n`;
    item.querySelector(".jump")?.addEventListener("click", () => jump(d));
    code.append(item);
  }
  pre.append(code);
  box.replaceChildren(pre);
}

/** Where a message points, in the file it's about. */
function range(d, state) {
  const line = state.doc.line(Math.min(Math.max(d.line, 1), state.doc.lines));
  const from = Math.min(line.from + Math.max(d.col - 1, 0), line.to);
  return { from, to: Math.min(from + Math.max(d.len, 1), line.to) };
}

function jump(d) {
  const index = docs.findIndex((doc) => doc.name === d.file);
  if (index < 0) return;
  show(index);
  const { from, to } = range(d, editor.state);
  editor.dispatch({ selection: { anchor: from, head: to }, scrollIntoView: true });
  editor.focus();
}

/** The messages about the open file, underlined in the editor. */
function marks() {
  const name = docs[current].name;
  const list = (last.diagnostics ?? []).filter((d) => d.file === name).map((d) => ({
    ...range(d, editor.state),
    severity: d.severity,
    message: `${d.code}: ${d.message}${d.help ? `\n${d.help}` : ""}`,
  }));
  editor.dispatch(setDiagnostics(editor.state, list));
}

// --- the outputs

let tab = "preview";
const viewer = new EditorView({ parent: $("output") });
// HTML and Markdown come with large languages, so they load the first time their tab opens
const languages = { json: json(), tools: json() };
const loaders = {
  html: () => import("@codemirror/lang-html").then((m) => m.html()),
  markdown: () => import("@codemirror/lang-markdown").then((m) => m.markdown()),
};
function language(kind) {
  if (!languages[kind]) {
    languages[kind] = [];
    loaders[kind]().then((loaded) => {
      languages[kind] = loaded;
      viewer.dom.dataset.kind = "";
      if (tab === kind) output();
    });
  }
  return languages[kind];
}
const viewerState = (text, kind) =>
  EditorState.create({ doc: text, extensions: [basicSetup, language(kind), looks, EditorState.readOnly.of(true), EditorView.editable.of(false)] });

// the preview shows the page with a plain stylesheet; links open nothing and scripts don't run (the frame is sandboxed)
const previewStyle = `<base target="_blank"><style>
:root { color-scheme: light dark; background: Canvas; color: CanvasText; font-family: system-ui, sans-serif; line-height: 1.55; }
body { max-width: 44rem; margin: 0 auto; padding: 1.5rem; }
img { max-width: 100%; height: auto; }
header, nav, footer { display: flex; flex-wrap: wrap; gap: 1rem; align-items: baseline; }
header { justify-content: space-between; border-bottom: 1px solid #8884; padding-bottom: .75rem; }
footer { border-top: 1px solid #8884; margin-top: 2rem; font-size: .9em; }
article { border-bottom: 1px solid #8882; padding-bottom: .5rem; }
form { display: grid; gap: .75rem; max-width: 26rem; margin-top: 1.5rem; }
label { display: grid; gap: .25rem; font-weight: 600; }
input, select, textarea, button { font: inherit; padding: .4rem .6rem; }
button { justify-self: start; font-weight: 600; cursor: pointer; }
</style>`;

function output() {
  const result = last.ok ? last : good;
  const stale = !last.ok;
  $("output").dataset.stale = stale ? (good ? "old" : "none") : "";
  const preview = $("preview");
  preview.hidden = tab !== "preview";
  viewer.dom.hidden = tab === "preview";
  if (!result) {
    preview.srcdoc = "";
    viewer.setState(viewerState("", "html"));
    return;
  }
  if (tab === "preview") {
    const page = result.html.replace(/<head>/, `<head>${previewStyle}`);
    if (preview.srcdoc !== page) preview.srcdoc = page;
  } else if (viewer.state.doc.toString() !== result[tab] || viewer.dom.dataset.kind !== tab) {
    viewer.setState(viewerState(result[tab], tab));
    viewer.dom.dataset.kind = tab;
  }
}

for (const button of $("outputs").querySelectorAll("[data-output]")) {
  button.addEventListener("click", () => {
    tab = button.dataset.output;
    for (const other of $("outputs").querySelectorAll("[data-output]")) other.ariaSelected = String(other === button);
    output();
  });
}

// --- formatting, sharing and examples

function format() {
  const doc = docs[current];
  if (doc.name.endsWith(".json")) {
    try {
      replace(JSON.stringify(JSON.parse(editor.state.doc.toString()), null, 2) + "\n");
    } catch {}
    return;
  }
  const answer = call("format", { source: editor.state.doc.toString() });
  if (answer.ok) replace(answer.source);
}

function replace(text) {
  if (text !== editor.state.doc.toString()) editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: text } });
}

$("format").addEventListener("click", format);

// a shared playground is its files and data in the address, compressed: hypralang.dev/play/#code=...
async function encode(value) {
  const stream = new Blob([JSON.stringify(value)]).stream().pipeThrough(new CompressionStream("deflate-raw"));
  const bytes = new Uint8Array(await new Response(stream).arrayBuffer());
  return btoa(String.fromCharCode(...bytes)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

async function decode(text) {
  const bytes = Uint8Array.from(atob(text.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0));
  const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
  return JSON.parse(await new Response(stream).text());
}

let saving = 0;
function remember(files, data) {
  // an example as it came is just its name; anything else is the code itself
  const now = JSON.stringify({ files, data });
  clearTimeout(saving);
  saving = setTimeout(async () => {
    const hash = example && now === pristine ? `#example=${example}` : `#code=${await encode({ files, data })}`;
    if (location.hash !== hash) history.replaceState(null, "", hash);
  }, 300);
}

$("share").addEventListener("click", async () => {
  const { files, data } = snapshot();
  const now = JSON.stringify({ files, data });
  const hash = example && now === pristine ? `#example=${example}` : `#code=${await encode({ files, data })}`;
  history.replaceState(null, "", hash);
  await navigator.clipboard.writeText(location.href);
  $("share").textContent = "Copied";
  setTimeout(() => ($("share").textContent = "Copy link"), 1500);
});

for (const [name, { title }] of Object.entries(examples)) {
  const option = document.createElement("option");
  option.value = name;
  option.textContent = title;
  $("example").append(option);
}
$("example").addEventListener("change", (event) => {
  const name = event.target.value;
  if (!name) return;
  if (JSON.stringify(snapshot()) !== pristine && !confirm("Replace what you wrote with the example?")) {
    event.target.value = example;
    return;
  }
  openExample(name);
});

function openExample(name) {
  const { files, data } = examples[name] ?? Object.values(examples)[0];
  open(files, data, examples[name] ? name : Object.keys(examples)[0]);
}

async function start() {
  const hash = new URLSearchParams(location.hash.slice(1));
  if (hash.has("code")) {
    try {
      const { files, data } = await decode(hash.get("code"));
      if (Array.isArray(files) && files.length) return open(files, data ?? "{}");
    } catch {}
  }
  openExample(hash.get("example") ?? "");
}

addEventListener("hashchange", start);
await start();
