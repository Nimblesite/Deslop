import { test, expect } from "@playwright/test";

test("the publication stamp is above the graph filters", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 844 });
  await page.goto("/issues/");
  const stampBox = await page.locator(".atlas-publication").boundingBox();
  const filterBox = await page.locator(".graph-filter-panel summary").boundingBox();
  expect(stampBox.y).toBeLessThan(filterBox.y);
});
