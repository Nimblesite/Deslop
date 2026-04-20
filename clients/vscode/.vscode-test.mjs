// @ts-check
// Basilisk/Forge-style: @vscode/test-cli drives a real VS Code, runs the
// unit + E2E suites compiled under out/, emits c8+lcov coverage over out/.
import { defineConfig } from "@vscode/test-cli";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const fixture = path.join(here, "out", "test", "fixtures", "csharp-small");
const releaseDir = path.join(here, "..", "..", "target", "release");

export default defineConfig({
  tests: [
    {
      files: "out/test/**/*.test.js",
      workspaceFolder: fixture,
      launchArgs: ["--disable-extensions"],
      env: {
        DESLOP_TEST_FIXTURE: fixture,
        DESLOP_BINARY_DIR: releaseDir,
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
