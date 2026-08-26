// @ts-check
// Basilisk/Forge-style: @vscode/test-cli drives a real VS Code, runs the
// unit + E2E suites compiled under out/. Coverage is NOT collected here:
// the desktop extension host ignores NODE_V8_COVERAGE on every injection
// channel for plain-Mocha extensionTestsPath suites, so `--coverage` could
// only ever print `All files | 0` — gh #440. The webview floor is measured
// by scripts/webview-coverage.mjs.
import { defineConfig } from "@vscode/test-cli";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { vscodeTestUserDataDir } from "./scripts/vscode-test-user-data-dir.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const fixture = path.join(here, "out", "test", "fixtures", "csharp-small");
// Keeps VS Code's IPC socket inside the 103-byte kernel cap however deep
// this checkout sits — see scripts/vscode-test-user-data-dir.mjs.
// [VSIX-TESTING-COVERAGE] Root hook that dumps the host's coverage table.
const COVERAGE_HOOK = path.join(here, "out", "test", "coverage-dump.js");
// VS Code does not hand arbitrary parent env to the extension host, so the
// collection directory rides the config's own env channel.
const COVERAGE_DIR_ENV = "DESLOP_EXTENSION_COVERAGE_DIR";
const coverageDir = process.env[COVERAGE_DIR_ENV] ?? "";
const launchArgs = ["--disable-extensions", "--user-data-dir", vscodeTestUserDataDir(here)];

export default defineConfig({
  tests: [
    {
      // Excludes `out/test/ollama/**` — those run only via
      // `.vscode-test-ollama.mjs` / `npm run test:ollama` /
      // `make _vsix-test-ollama`. See docs/specs/vsix.md.
      files: ["out/test/suite/**/*.test.js", "out/test/unit/**/*.test.js"],
      workspaceFolder: fixture,
      launchArgs,
      env: {
        DESLOP_TEST_FIXTURE: fixture,
        // [VSIX-BUNDLED-BINARY-TESTS] Clear the override env so resolution
        // falls to ${extensionPath}/bin/<platform>/ — proves the bundle.
        [COVERAGE_DIR_ENV]: coverageDir,
        DESLOP_BINARY_DIR: "",
        DESLOP_LSP_PATH: "",
        DESLOP_MCP_PATH: "",
      },
      mocha: {
        ui: "tdd",
        timeout: 60_000,
        bail: true,
        require: [COVERAGE_HOOK],
      },
    },
    {
      // [#201] No `workspaceFolder` → VS Code launches an EMPTY window. This
      // is the only config that exercises activate()'s no-folder path (the
      // LSP-launch guard + the "ready" idle else-branch); the fixture entry
      // above always has a folder open. Kept in its own launch so the
      // folder-dependent suites still get their workspace.
      files: ["out/test/no-folder/**/*.test.js"],
      launchArgs,
      env: {
        // Same bundled-binary resolution as the fixture entry: clear the
        // override env so it falls to ${extensionPath}/bin/<platform>/.
        [COVERAGE_DIR_ENV]: coverageDir,
        DESLOP_BINARY_DIR: "",
        DESLOP_LSP_PATH: "",
        DESLOP_MCP_PATH: "",
      },
      mocha: {
        ui: "tdd",
        timeout: 60_000,
        bail: true,
        require: [COVERAGE_HOOK],
      },
    },
  ],

  // [VSIX-TESTING-COVERAGE] The extension host writes no V8 profile for our
  // code (gh #440), so coverage is instrumented into the bundle at build time
  // and dumped from the host by this root hook. See
  // scripts/istanbul-esbuild-plugin.mjs and scripts/extension-coverage.mjs.
});
