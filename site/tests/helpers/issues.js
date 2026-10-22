import { readFileSync } from "node:fs";
import { test as base } from "@playwright/test";

export { expect } from "@playwright/test";

/**
 * [SITE-ISSUES-FIXTURE] The issue report every issue test runs against. `pretest` generates it from the committed GitHub snapshot in `tests/fixtures/github-issues.json`, never from live GitHub, so the suite sees the same issues on every run.
 */
const FIXTURE_REPORT = new URL("../.generated/issues.json", import.meta.url);
/** Where the issue pages and the tests themselves fetch the report from. */
const REPORT_ROUTE = "**/assets/data/issues.json";
const JSON_TYPE = "application/json";

/** Playwright's `test`, with every page served the fixture report in place of the published one. */
export const test = base.extend({
  page: async ({ page }, use) => {
    const body = readFileSync(FIXTURE_REPORT, "utf8");
    await page.route(REPORT_ROUTE, (route) => route.fulfill({ contentType: JSON_TYPE, body }));
    await use(page);
  },
});
