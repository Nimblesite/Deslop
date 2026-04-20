// Runs the full VSIX test suite.
//
// 1) Unit tests (pure modules) — mocha in this process, directly.
// 2) E2E tests — @vscode/test-electron spawning real VS Code against the
//    real codededup-lsp binary (built from the Rust workspace).
//
// When `NODE_V8_COVERAGE` is set (c8 sets it automatically), both
// processes emit v8 coverage into the same directory, c8 merges them.

import { runTests } from "@vscode/test-electron";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { existsSync } from "node:fs";
import { spawnSync } from "node:child_process";

const here = dirname(fileURLToPath(import.meta.url));
const extensionDevelopmentPath = resolve(here, "..");
const extensionTestsPath = resolve(extensionDevelopmentPath, "out", "test", "test", "suite", "index.js");
const fixture = resolve(here, "fixtures", "csharp-small");
const repoRoot = resolve(here, "..", "..", "..");
const releaseDir = resolve(repoRoot, "target", "release");

const exe = process.platform === "win32" ? ".exe" : "";
const lspBinary = resolve(releaseDir, `codededup-lsp${exe}`);
const mcpBinary = resolve(releaseDir, `codededup-mcp${exe}`);

if (!existsSync(lspBinary) || !existsSync(mcpBinary)) {
  console.log("building real codededup-lsp + codededup-mcp…");
  const result = spawnSync(
    "cargo",
    ["build", "--release", "-p", "codededup-lsp", "-p", "codededup-mcp"],
    { cwd: repoRoot, stdio: "inherit" },
  );
  if (result.status !== 0) {
    console.error("cargo build failed; VSIX E2E cannot run without the real binaries.");
    process.exit(result.status ?? 1);
  }
}
if (!existsSync(lspBinary) || !existsSync(mcpBinary)) {
  console.error(`expected binaries missing after build: ${lspBinary}, ${mcpBinary}`);
  process.exit(1);
}

// 1) Unit tests — fast, no VS Code.
console.log("\n=== Unit tests ===");
const unitResult = spawnSync(
  process.execPath,
  [resolve(here, "run-unit-tests.mjs")],
  { cwd: extensionDevelopmentPath, stdio: "inherit", env: process.env },
);
if (unitResult.status !== 0) process.exit(unitResult.status ?? 1);

// 2) E2E tests inside real VS Code.
console.log("\n=== E2E tests ===");
const e2eEnv = {
  ...process.env,
  CODEDEDUP_TEST_FIXTURE: fixture,
  CODEDEDUP_BINARY_DIR: releaseDir,
};

const exitCode = await runTests({
  extensionDevelopmentPath,
  extensionTestsPath,
  launchArgs: [fixture],
  extensionTestsEnv: e2eEnv,
});
process.exit(exitCode);
