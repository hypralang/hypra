// Connects VS Code to `hypra lsp`, the language server in the hypra compiler: errors as you type, Go to Definition,
// references, rename, hover help and Format Document all come from there.
const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");
const { projectCompiler } = require("./compiler");

let client;

/** The `hypra.path` setting, else the compiler the project installed, else `hypra` on the PATH. */
function compiler() {
  const configured = vscode.workspace.getConfiguration("hypra").get("path");
  if (configured && configured !== "hypra") return configured;
  const folders = (vscode.workspace.workspaceFolders ?? []).map((f) => f.uri.fsPath);
  return projectCompiler(folders) ?? "hypra";
}

async function start() {
  // a compiler inside the project only runs once the folder is trusted; until then, there's highlighting only
  if (!vscode.workspace.isTrusted) return;
  const command = compiler();
  client = new LanguageClient(
    "hypra",
    "Hypra",
    { command, args: ["lsp"] },
    { documentSelector: [{ scheme: "file", language: "hypra" }] },
  );
  try {
    await client.start();
  } catch (e) {
    client = undefined;
    const help = "How to install";
    const choice = await vscode.window.showWarningMessage(
      `Hypra: can't start "${command} lsp" (${e.message}). Install the compiler with \`npm install hypra\` or ` +
        "`cargo install hypra`, or set hypra.path to it.",
      help,
    );
    if (choice === help) vscode.env.openExternal(vscode.Uri.parse("https://hypralang.dev/docs/vscode/"));
  }
}

async function restart() {
  await client?.stop();
  client = undefined;
  await start();
}

async function activate(context) {
  await start();
  context.subscriptions.push(
    vscode.workspace.onDidGrantWorkspaceTrust(restart),
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (event.affectsConfiguration("hypra.path")) await restart();
    }),
  );
}

function deactivate() {
  return client?.stop();
}

module.exports = { activate, deactivate };
