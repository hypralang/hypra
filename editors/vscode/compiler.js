// Finds the hypra compiler a project installed, so the extension works without one on the PATH.
const { existsSync } = require("node:fs");
const { join } = require("node:path");

// the release builds the npm package holds, as in js/bin/hypra.js
const targets = {
  "linux-x64": "x86_64-unknown-linux-musl",
  "linux-arm64": "aarch64-unknown-linux-musl",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
  "win32-arm64": "aarch64-pc-windows-msvc",
};

/**
 * The compiler in one of `folders`: from `npm install hypra`, or from `php artisan hypra:install` in a Laravel
 * app. Undefined when there's none.
 */
function projectCompiler(folders, platform = process.platform, arch = process.arch) {
  const exe = platform === "win32" ? "hypra.exe" : "hypra";
  const target = targets[`${platform}-${arch}`];
  for (const folder of folders) {
    const candidates = [join(folder, "storage", "hypra", exe)];
    if (target) candidates.unshift(join(folder, "node_modules", "hypra", "bin", target, exe));
    const found = candidates.find((path) => existsSync(path));
    if (found) return found;
  }
  return undefined;
}

module.exports = { projectCompiler };
