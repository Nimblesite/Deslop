import { expect, test } from "@playwright/test";

test("MCP tool names fit on one line without widening the page", async ({ page }) => {
  for (const [route, width] of [["/docs/for-ai/", 320], ["/zh/docs/for-ai/", 390], ["/docs/for-ai/", 1440]]) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto(route);
    const table = page.locator(".mcp-tool-table").getByRole("table");
    const names = table.locator("tbody td:first-child code");
    await expect(names).toContainText(["find-similar", "duplicates", "cluster-by-id"]);
    await page.evaluate(() => document.fonts.ready.then(() => undefined));
    const wrapped = await names.evaluateAll((elements) =>
      elements.filter((element) => element.getClientRects().length !== 1)
        .map((element) => element.textContent),
    );
    expect(wrapped, `tool names at ${width}px`).toEqual([]);
    const pageWidth = await page.evaluate(() => document.documentElement.scrollWidth);
    expect(pageWidth).toBeLessThanOrEqual(width);
  }
});
