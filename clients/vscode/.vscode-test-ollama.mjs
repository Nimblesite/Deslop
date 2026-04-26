// @ts-check
// Ollama-gated VSIX e2e tests. Points a REAL VS Code instance at the
// bundled deslop-lsp binary with provider="ollama" + model="nomic-embed-text",
// over the csharp-type4 fixture (recursive vs. iterative — semantically
// equivalent, structurally distinct, so only an embedding pass can match).
//
// Runs ONLY via `npm run test:ollama` or `make vsix-test-ollama`. NEVER
// part of `make ci` / `make vsix-test` / default `npm test` — those stay
// Ollama-free and CI-safe. See docs/specs/vsix.md for the gating policy.
import { defineConfig } from "@vscode/test-cli";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const fixture = path.join(here, "out", "test", "fixtures", "csharp-type4");

export default defineConfig({
  tests: [
    {
      label: "ollama-e2e",
      files: "out/test/ollama/**/*.test.js",
      workspaceFolder: fixture,
      launchArgs: ["--disable-extensions"],
      env: {
        DESLOP_TEST_FIXTURE: fixture,
        DESLOP_BINARY_DIR: "",
        DESLOP_LSP_PATH: "",
        DESLOP_MCP_PATH: "",
      },
      mocha: {
        ui: "tdd",
        timeout: 120_000,
        bail: true,
      },
    },
  ],
});
