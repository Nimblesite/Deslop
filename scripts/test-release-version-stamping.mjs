// Tests for first-class release/test version stamping.

import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

const repoRoot = resolve(new URL("..", import.meta.url).pathname);
const stamper = join(repoRoot, "scripts/stamp-release-version.mjs");
const version = "9.8.7-test.1";

const tests = [
  sourceProjectsUseVersionPlaceholder,
  stamperSetsEveryProjectVersion,
  stamperRejectsInvalidVersion,
];

let failed = 0;
for (const test of tests) {
  const work = mkdtempSync(join(tmpdir(), "deslop-version-stamp-"));
  try {
    test(work);
    console.log(`ok ${test.name}`);
  } catch (error) {
    failed++;
    console.error(`not ok ${test.name}`);
    console.error(`  ${error instanceof Error ? error.message : String(error)}`);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

if (failed > 0) {
  console.error(`\n${failed} release version stamping test(s) failed`);
  process.exit(1);
}
console.log(`\n${tests.length} release version stamping tests passed`);

function sourceProjectsUseVersionPlaceholder() {
  const placeholder = "0.0.0-dev";
  assertIncludes(read(repoRoot, "Cargo.toml"), `version = "${placeholder}"`);
  assertIncludes(read(repoRoot, "Cargo.lock"), `name = "deslop"\nversion = "${placeholder}"`);
  assertIncludes(read(repoRoot, "Cargo.lock"), `name = "deslop-mcp"\nversion = "${placeholder}"`);
  assertJsonVersion(repoRoot, "deployment-toolkit.json", placeholder);
  assertJsonVersion(repoRoot, "clients/vscode/deployment-toolkit.json", placeholder);
  assertJsonVersion(repoRoot, "clients/vscode/package.json", placeholder);
  assertJsonVersion(repoRoot, "clients/vscode/package-lock.json", placeholder);
  assertJsonVersion(repoRoot, "clients/vscode/webview-ui/package.json", placeholder);
  assertJsonVersion(repoRoot, "clients/vscode/webview-ui/package-lock.json", placeholder);
  assertJsonVersion(repoRoot, "site/package.json", placeholder);
  assertJsonVersion(repoRoot, "site/package-lock.json", placeholder);
}

function stamperSetsEveryProjectVersion(work) {
  copyStampInputs(work);
  const result = spawnSync("node", [stamper, version, "--root", work], { encoding: "utf8" });
  if (result.status !== 0) throw new Error(`stamper failed: ${result.stderr}`);

  assertIncludes(read(work, "Cargo.toml"), `version = "${version}"`);
  assertIncludes(read(work, "Cargo.lock"), `name = "deslop"\nversion = "${version}"`);
  assertIncludes(read(work, "Cargo.lock"), `name = "deslop-lsp"\nversion = "${version}"`);
  assertJsonVersion(work, "deployment-toolkit.json", version);
  assertJsonVersion(work, "clients/vscode/deployment-toolkit.json", version);
  assertJsonVersion(work, "clients/vscode/package.json", version);
  assertJsonVersion(work, "clients/vscode/package-lock.json", version);
  assertJsonVersion(work, "clients/vscode/webview-ui/package.json", version);
  assertJsonVersion(work, "clients/vscode/webview-ui/package-lock.json", version);
  assertJsonVersion(work, "site/package.json", version);
  assertJsonVersion(work, "site/package-lock.json", version);
}

function stamperRejectsInvalidVersion(work) {
  copyStampInputs(work);
  const result = spawnSync("node", [stamper, "v9.8", "--root", work], { encoding: "utf8" });
  if (result.status === 0) throw new Error("stamper accepted an invalid version");
  assertIncludes(`${result.stdout}\n${result.stderr}`, "semantic version");
}

function copyStampInputs(work) {
  for (const file of [
    "Cargo.toml",
    "Cargo.lock",
    "deployment-toolkit.json",
    "clients/vscode/deployment-toolkit.json",
    "clients/vscode/package.json",
    "clients/vscode/package-lock.json",
    "clients/vscode/webview-ui/package.json",
    "clients/vscode/webview-ui/package-lock.json",
    "site/package.json",
    "site/package-lock.json",
  ]) {
    const dest = join(work, file);
    mkdirSync(dirname(dest), { recursive: true });
    copyFileSync(join(repoRoot, file), dest);
  }
}

function assertJsonVersion(root, file, expected) {
  const value = JSON.parse(read(root, file));
  if (value.version !== expected && value.product?.version !== expected) {
    throw new Error(`${file} was not stamped to ${expected}`);
  }
  for (const component of value.components ?? []) {
    if (component.expectedVersion !== expected) {
      throw new Error(`${file} has stale ${component.id} expectedVersion`);
    }
  }
  if (value.packages?.[""]?.version && value.packages[""].version !== expected) {
    throw new Error(`${file} root package lock version was not stamped`);
  }
}

function read(root, file) {
  return readFileSync(join(root, file), "utf8");
}

function assertIncludes(value, expected) {
  if (!value.includes(expected)) throw new Error(`missing ${expected}`);
}
