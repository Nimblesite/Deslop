// @ts-check
// Basilisk/Forge-style: @vscode/test-cli drives a real VS Code, runs the
// unit + E2E suites compiled under out/, emits c8+lcov coverage over out/.
import { defineConfig } from "@vscode/test-cli";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { vscodeTestUserDataDir } from "./scripts/vscode-test-user-data-dir.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const fixture = path.join(here, "out", "test", "fixtures", "csharp-small");
// Keeps VS Code's IPC socket inside the 103-byte kernel cap however deep
// this checkout sits — see scripts/vscode-test-user-data-dir.mjs.
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
        DESLOP_BINARY_DIR: "",
        DESLOP_LSP_PATH: "",
        DESLOP_MCP_PATH: "",
      },
      mocha: {
        ui: "tdd",
        timeout: 60_000,
        bail: true,
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
        DESLOP_BINARY_DIR: "",
        DESLOP_LSP_PATH: "",
        DESLOP_MCP_PATH: "",
      },
      mocha: {
        ui: "tdd",
        timeout: 60_000,
        bail: true,
      },
    },
  ],
  coverage: {
    includeAll: true,
    include: ["**/out/**/*.js"],
    exclude: [
      "**/out/test/**",
      "**/out/logging.js",
    ],
    reporter: ["text", "text-summary", "json-summary", "lcov"],
  },
});
