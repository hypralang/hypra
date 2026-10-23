// "Hypra: Open Preview to the Side": the page next to the editor, as a browser shows it, or as the Markdown and
// JSON it compiles to. The buttons in the preview's title bar switch between them.
const vscode = require("vscode");
const { dirname, join } = require("node:path");
const { readFile } = require("node:fs/promises");
const { basename } = require("node:path");
const { build, dataNote, noPage, pageFor, pageView, siteFiles, siteRoot, textView } = require("./preview-core");

const panels = new Map();
let active;

/** Opens the preview of the current .hypra file, or shows it if it's already open. */
function open(compiler) {
  const editor = vscode.window.activeTextEditor;
  if (editor?.document.languageId !== "hypra" || editor.document.uri.scheme !== "file") {
    vscode.window.showInformationMessage("Hypra: open a .hypra file to preview it.");
    return;
  }
  const file = editor.document.uri.fsPath;
  const existing = panels.get(file);
  if (existing) return existing.panel.reveal(vscode.ViewColumn.Beside, true);

  const folder = vscode.workspace.getWorkspaceFolder(editor.document.uri)?.uri.fsPath ?? dirname(file);
  const root = siteRoot(file, folder);
  const panel = vscode.window.createWebviewPanel(
    "hypra.preview",
    `Preview ${editor.document.uri.path.split("/").pop()}`,
    { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
    { enableScripts: true, localResourceRoots: [vscode.Uri.file(root)] },
  );
  panel.iconPath = vscode.Uri.joinPath(vscode.Uri.file(__dirname), "..", "icon.png");
  const preview = { panel, file, root, compiler, show: "page" };
  panels.set(file, preview);
  active = preview;
  panel.onDidChangeViewState(({ webviewPanel }) => {
    if (webviewPanel.active) active = preview;
  });
  panel.onDidDispose(() => {
    panels.delete(file);
    if (active === preview) active = undefined;
  });
  render(preview);
}

async function render(preview) {
  // a layout or component is shown in a page that uses it
  const found = await vscode.workspace.findFiles("**/*.hypra", "**/{node_modules,vendor}/**");
  const files = await Promise.all(found.map(async (uri) => [uri.fsPath, await readFile(uri.fsPath, "utf8")]));
  const page = pageFor(preview.file, files);
  const name = basename(preview.file);
  preview.panel.title = page && page !== preview.file ? `Preview ${name} in ${basename(page)}` : `Preview ${name}`;
  preview.page = page;
  preview.result = page
    ? await build(preview.compiler(), page, join(__dirname, "runtime.mjs"))
    : { note: noPage(preview.file) };
  draw(preview);
}

function draw({ panel, page, root, result, show }) {
  const csp = panel.webview.cspSource;
  if (result.note) {
    panel.webview.html = textView(result.note, { csp });
  } else if (result.problem) {
    panel.webview.html = textView(result.problem, { csp, note: "The page doesn't build yet:", problem: true });
  } else if (show === "page") {
    const local = (path) => panel.webview.asWebviewUri(vscode.Uri.file(path)).toString();
    panel.webview.html = pageView(siteFiles(result.html, root, local), csp);
  } else {
    panel.webview.html = textView(show === "markdown" ? result.markdown : result.json, { csp, note: dataNote(result, page) });
  }
}

/** Shows the page, its Markdown or its JSON in the preview that's in front. */
function show(what) {
  if (!active?.result) return;
  active.show = what;
  draw(active);
}

/** Builds every open preview again: a page changes with its layouts, components, data and styles. */
function refresh(document) {
  if (!/\.(hypra|json|css)$/.test(document.uri.fsPath)) return;
  for (const preview of panels.values()) render(preview);
}

module.exports = { open, refresh, show };
