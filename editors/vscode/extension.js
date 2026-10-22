// Connects VS Code to `hypra lsp`, the language server in the hypra compiler: errors as you type, Go to Definition,
// references, rename, hover help and Format Document all come from there.
const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

async function start() {
  const command = vscode.workspace.getConfiguration("hypra").get("path") || "hypra";
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
    vscode.window.showWarningMessage(
      `Hypra: can't start "${command} lsp" (${e.message}). Update hypra, or set hypra.path to it.`,
    );
  }
}

async function activate(context) {
  await start();
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (!event.affectsConfiguration("hypra.path")) return;
      await client?.stop();
      await start();
    }),
  );
}

function deactivate() {
  return client?.stop();
}

module.exports = { activate, deactivate };
