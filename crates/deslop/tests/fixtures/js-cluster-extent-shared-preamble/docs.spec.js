import { test, expect } from "@playwright/test";

const DOCS_LINK_COUNT = 12;

test("localizes the grouped documentation navigation", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/zh/docs/");

  const docsNav = page.locator(".docs-sidebar__nav");
  await expect(docsNav.locator("a")).toHaveCount(DOCS_LINK_COUNT);
  await expect(docsNav.locator("summary")).toHaveText(["入门", "参考"]);
});
