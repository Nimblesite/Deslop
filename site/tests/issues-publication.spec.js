import { expect, test } from "@playwright/test";

const LONG_TIMESTAMP = /^Published \d{1,2}(?:st|nd|rd|th) of [A-Z][a-z]+ \d{4}, \d{1,2}(?:am|pm) UTC$/;

async function expectPublicationStamp(page) {
  const report = await page.evaluate(async () => (await fetch("/assets/data/issues.json")).json());
  const stamp = page.locator(".atlas-publication");
  await expect(stamp).toHaveCount(1);
  await expect(stamp).toBeVisible();
  await expect(stamp).toHaveAttribute("datetime", report.meta.published_at);
  await expect(stamp).toHaveText(LONG_TIMESTAMP);
  const box = await stamp.boundingBox();
  const viewport = page.viewportSize();
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  return stamp.textContent();
}

test("shows the publication instant on the graph report", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 844 });
  await page.goto("/issues/");
  const published = await expectPublicationStamp(page);
  const stampBox = await page.locator(".atlas-publication").boundingBox();
  const filterBox = await page.locator(".graph-filter-panel summary").boundingBox();
  expect(filterBox.y + filterBox.height <= stampBox.y || stampBox.y + stampBox.height <= filterBox.y).toBeTruthy();
  for (const button of await page.locator(".network-tool").all()) {
    const box = await button.boundingBox();
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(320);
  }
  expect(published).toMatch(LONG_TIMESTAMP);
});

test("keeps one publication instant across every planner report", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/issues/planner/");
  const published = await expectPublicationStamp(page);
  for (const view of ["Queue", "Statistics", "Runway", "Priority"]) {
    await page.getByRole("tab", { name: view, exact: true }).click();
    await expect(page.locator(".atlas-publication")).toHaveText(published);
  }
});
