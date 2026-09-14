import { test, expect } from "@playwright/test";

async function expectPublicationStamp(page) {
  const stamp = page.locator(".atlas-publication");
  await expect(stamp).toBeVisible();
  return stamp;
}

test("shows the publication instant on the graph report", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 844 });
  await page.goto("/issues/");
  const published = await expectPublicationStamp(page);
  const stampBox = await published.boundingBox();
  const filterBox = await page.locator(".graph-filter-panel summary").boundingBox();
  expect(stampBox.y).toBeLessThan(filterBox.y);
});

test("keeps one publication instant across every planner report", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 844 });
  await page.goto("/issues/");
  const published = await expectPublicationStamp(page);
  for (const tab of await page.locator(".view-tab").all()) {
    await tab.click();
    await expect(page.locator(".atlas-publication")).toHaveText(await published.textContent());
  }
});
