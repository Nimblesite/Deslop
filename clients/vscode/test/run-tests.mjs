// E2E entry point.
//
// Builds the REAL codededup-lsp + codededup-mcp binaries from the workspace
// with `cargo build --release -p codededup-lsp -p codededup-mcp` and points
// the extension at them via CODEDEDUP_BINARY_DIR. No LSP stubs, ever.

import { runTests } from "@vscode/test-electron";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { existsSync } from "node:fs";
import { spawnSync } from "node:child_process";

const here = dirname(fileURLToPath(import.meta.url));
const extensionDevelopmentPath = resolve(here, "..");
const extensionTestsPath = resolve(here, "suite", "index.js");
const fixture = resolve(here, "fixtures", "csharp-small");
const repoRoot = resolve(here, "..", "..", "..");
const releaseDir = resolve(repoRoot, "target", "release");

const exe = process.platform === "win32" ? ".exe" : "";
const lspBinary = resolve(releaseDir, `codededup-lsp${exe}`);
const mcpBinary = resolve(releaseDir, `codededup-mcp${exe}`);

if (!existsSync(lspBinary) || !existsSync(mcpBinary)) {
  console.log("building real codededup-lsp + codededup-mcp from the workspace…");
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

const exitCode = await runTests({
  extensionDevelopmentPath,
  extensionTestsPath,
  launchArgs: ["--disable-extensions", "--new-window", fixture],
  extensionTestsEnv: {
    CODEDEDUP_TEST_FIXTURE: fixture,
    CODEDEDUP_BINARY_DIR: releaseDir,
  },
});
process.exit(exitCode);
