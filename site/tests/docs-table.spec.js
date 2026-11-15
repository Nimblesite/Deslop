import { expect, test } from "@playwright/test";

const PHONE = 375;
const NARROW = 320;
const DESKTOP = 1440;
const CONFIG_KEYS = ["--min-nodes", "nearly_identical_min_shape", "shape_only_max_content", "max_duplication_percent"];

/** Asserts every first-column key renders on one line and the page never scrolls sideways. */
async function expectKeysOnOneLine(page, route, width, expected) {
  await page.setViewportSize({ width, height: 900 });
  await page.goto(route);
  const names = page.locator(".key-table").first().getByRole("table").locator("tbody td:first-child code");
  await expect(names).toContainText(expected);
  await page.evaluate(() => document.fonts.ready.then(() => undefined));
  const wrapped = await names.evaluateAll((elements) =>
    elements.filter((element) => element.getClientRects().length !== 1).map((element) => element.textContent),
  );
  expect(wrapped, `${route} keys at ${width}px`).toEqual([]);
  const pageWidth = await page.evaluate(() => document.documentElement.scrollWidth);
  expect(pageWidth, `${route} page width at ${width}px`).toBeLessThanOrEqual(width);
}

test("MCP tool names fit on one line without widening the page", async ({ page }) => {
  for (const [route, width] of [["/docs/for-ai/", NARROW], ["/zh/docs/for-ai/", 390], ["/docs/for-ai/", DESKTOP]]) {
    await expectKeysOnOneLine(page, route, width, ["find-similar", "duplicates", "cluster-by-id"]);
  }
});

test("the configuration dials lead the page and stay readable on a phone", async ({ page }) => {
  for (const [route, width] of [["/docs/configuration/", PHONE], ["/zh/docs/configuration/", PHONE], ["/docs/configuration/", DESKTOP]]) {
    await expectKeysOnOneLine(page, route, width, CONFIG_KEYS);
    const firstSection = page.locator(".prose h2").first();
    await expect(firstSection).toHaveText(route.startsWith("/zh/") ? "可调参数" : "Dials");
    const effect = page.locator(".key-table").first().locator("tbody tr").first().locator("td").nth(2);
    const box = await effect.boundingBox();
    if (width === PHONE) expect(box?.width ?? 0, `${route} effect column width`).toBeGreaterThan(width / 2);
  }
});
