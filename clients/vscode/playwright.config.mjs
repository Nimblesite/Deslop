// Playwright configuration for the extension's browser-driven specs.
//
// Exists for one reason: `outputDir`. With no config, Playwright defaults its
// output directory to `test-results/` beside the extension — the packaging
// root — so `.last-run.json`, traces, screenshots and videos from a failing
// run all landed inside the VSIX (#472). Every build artifact in this
// repository belongs under `target/`, alongside the screenshot directories the
// specs themselves already write there. Three call sites run Playwright
// (`test:playwright`, `test:playwright:html`, and `scripts/webview-coverage.mjs`),
// so the setting lives here once rather than as a flag on each of them.
//
// Everything else stays on Playwright's defaults on purpose: the specs are
// passed by path, and this file must not change what they discover or how they
// report. The VSIX allow-list in `scripts/package-contents-gate.mjs` is the
// backstop if this ever regresses.

import { defineConfig } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";

/** `clients/vscode`, the directory holding this config. */
const extensionRoot = path.dirname(fileURLToPath(import.meta.url));

/** Repository root: `clients/vscode` sits two levels below it. */
const repoRoot = path.resolve(extensionRoot, "..", "..");

export default defineConfig({
  outputDir: path.join(repoRoot, "target", "playwright-test-results"),
});
