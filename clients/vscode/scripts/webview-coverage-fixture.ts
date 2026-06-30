// [VSIX-WEBVIEW-COVERAGE] Playwright fixture that records V8 JS coverage for every webview-bundle test
// and writes the raw entries to disk when WEBVIEW_COVERAGE=1. The smoke spec
// imports `test`/`expect` from here instead of `@playwright/test`, so the same
// suite that proves the webviews render also drives the coverage number — one
// set of interactions, no duplicate rendering harness. The post-run converter
// (scripts/webview-coverage.mjs) maps these raw entries back to webview-ui/src.

import { test as base, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const rawDir = path.join(process.cwd(), "coverage", "webview", "raw");

export const test = base.extend({
  page: async ({ page }, use, testInfo) => {
    const collect = process.env.WEBVIEW_COVERAGE === "1";
    if (collect) {
      // The webviews load as inline ESM (setContent), which V8 treats as an
      // anonymous script — Playwright drops those unless asked for explicitly.
      await page.coverage.startJSCoverage({ resetOnNavigation: false, reportAnonymousScripts: true });
    }
    await use(page);
    if (!collect) return;
    const entries = await page.coverage.stopJSCoverage();
    // Keep only the inline ESM bundle — it carries the inline sourcemap the
    // converter needs; the tiny acquireVsCodeApi shim has none.
    const mapped = entries.filter((entry) => entry.source?.includes("sourceMappingURL"));
    fs.mkdirSync(rawDir, { recursive: true });
    const safeTitle = testInfo.title.replace(/[^a-z0-9]+/gi, "-").slice(0, 60);
    fs.writeFileSync(
      path.join(rawDir, `${testInfo.testId}-${safeTitle}.json`),
      JSON.stringify(mapped),
    );
  },
});

export { expect };
