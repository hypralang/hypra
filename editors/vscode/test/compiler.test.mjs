// Checks where the extension finds the compiler a project installed.
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { join } from "node:path";
import assert from "node:assert/strict";

const { projectCompiler } = createRequire(import.meta.url)("../compiler.js");

function project(...files) {
  const folder = mkdtempSync(join(tmpdir(), "hypra-"));
  for (const file of files) {
    mkdirSync(join(folder, file, ".."), { recursive: true });
    writeFileSync(join(folder, file), "");
  }
  return folder;
}

const npm = project("node_modules/hypra/bin/x86_64-unknown-linux-musl/hypra");
assert.equal(projectCompiler([npm], "linux", "x64"), join(npm, "node_modules/hypra/bin/x86_64-unknown-linux-musl/hypra"));
// the npm package has no build for this system
assert.equal(projectCompiler([npm], "linux", "ia32"), undefined);

const laravel = project("storage/hypra/hypra.exe");
assert.equal(projectCompiler([laravel], "win32", "x64"), join(laravel, "storage/hypra/hypra.exe"));
assert.equal(projectCompiler([laravel], "linux", "x64"), undefined);

// the first folder that has one
const empty = project();
assert.equal(projectCompiler([empty, laravel, npm], "win32", "x64"), join(laravel, "storage/hypra/hypra.exe"));
assert.equal(projectCompiler([empty], "linux", "x64"), undefined);
assert.equal(projectCompiler([], "linux", "x64"), undefined);

console.log("compiler ok");
