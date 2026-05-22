#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const platformPackages = {
  "linux:x64": "openclash-bin-linux-x64-musl",
  "linux:arm64": "openclash-bin-linux-arm64-musl",
  "darwin:arm64": "openclash-bin-darwin-arm64",
  "win32:ia32": "openclash-bin-win32-ia32",
};

const platformKey = `${process.platform}:${process.arch}`;
const platformPackage = platformPackages[platformKey];

if (!platformPackage) {
  console.error(`openclash npm package currently supports linux-x64, linux-arm64, darwin-arm64, and win32-ia32; got ${process.platform}-${process.arch}`);
  process.exit(1);
}

let binary;
try {
  const packageJson = require.resolve(`${platformPackage}/package.json`);
  binary = path.join(path.dirname(packageJson), "bin", process.platform === "win32" ? "openclash.exe" : "openclash");
} catch (error) {
  console.error(`Missing optional dependency: ${platformPackage}`);
  process.exit(1);
}

try {
  fs.chmodSync(binary, 0o755);
} catch (error) {
  // Ignore chmod failures; exec below will report the real problem if the file is not runnable.
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}
process.exit(result.status ?? 1);
