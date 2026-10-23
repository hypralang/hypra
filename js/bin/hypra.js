#!/usr/bin/env node
// Runs the hypra compiler built for this machine. The package holds one for every system releases are built for,
// so nothing is downloaded when it's installed.
import { chmodSync, existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const targets = {
  "linux-x64": "x86_64-unknown-linux-musl",
  "linux-arm64": "aarch64-unknown-linux-musl",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
  "win32-arm64": "aarch64-pc-windows-msvc",
};
const system = `${process.platform}-${process.arch}`;
const target = targets[system];
const exe = target && join(dirname(fileURLToPath(import.meta.url)), target, process.platform === "win32" ? "hypra.exe" : "hypra");
if (!exe || !existsSync(exe)) {
  console.error(
    `hypra: this package has no compiler for ${system}. Install it with \`cargo install hypra\`, or download it from https://github.com/hypralang/hypra/releases`,
  );
  process.exit(2);
}

let result = spawnSync(exe, process.argv.slice(2), { stdio: "inherit" });
// a package manager that drops the executable bit
if (result.error?.code === "EACCES") {
  chmodSync(exe, 0o755);
  result = spawnSync(exe, process.argv.slice(2), { stdio: "inherit" });
}
if (result.error) {
  console.error(`hypra: can't run ${exe}: ${result.error.message}`);
  process.exit(2);
}
process.exit(result.status ?? 1);
