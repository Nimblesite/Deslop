// @ts-check
// Basilisk/Forge-style: @vscode/test-cli drives a real VS Code, runs the
// unit + E2E suites compiled under out/, emits c8+lcov coverage over out/.
import { defineConfig } from "@vscode/test-cli";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const fixture = path.join(here, "out", "test", "fixtures", "csharp-small");

export default defineConfig({
  tests: [
    {
      // Pin the VS Code build: 1.127.0 broke @vscode/test-cli's extension-host
      // V8 coverage collection — activate() reads as ~0% covered even though the
      // whole E2E suite passes, sinking the line-coverage gate below threshold.
      // 1.119.0 is the last build where host coverage flushes correctly; revisit
      // when the test-cli / newer-VS-Code coverage-collection regression is fixed.
      version: "1.119.0",
      // Excludes `out/test/ollama/**` — those run only via
      // `.vscode-test-ollama.mjs` / `npm run test:ollama` /
      // `make _vsix-test-ollama`. See docs/specs/vsix.md.
      files: ["out/test/suite/**/*.test.js", "out/test/unit/**/*.test.js"],
      workspaceFolder: fixture,
      launchArgs: ["--disable-extensions"],
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
