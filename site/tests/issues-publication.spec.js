import { expect, test } from "@playwright/test";

const LONG_TIMESTAMP = /^Published \d{1,2}(?:st|nd|rd|th) of [A-Z][a-z]+ \d{4}, \d{1,2}(?:am|pm) UTC$/;

async function contrastRatio(locator) {
  return locator.evaluate((element) => {
    const luminance = (color) => {
      const values = color.match(/[\d.]+/g).slice(0, 3).map((value) => Number(value) / 255);
      const linear = values.map((value) => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
      return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
    };
    const style = getComputedStyle(element);
    const foreground = luminance(style.color);
    const backgroundColor = style.backgroundColor === "rgba(0, 0, 0, 0)" ? getComputedStyle(element.parentElement).backgroundColor : style.backgroundColor;
    const background = luminance(backgroundColor);
    return (Math.max(foreground, background) + 0.05) / (Math.min(foreground, background) + 0.05);
  });
}

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
  expect(await contrastRatio(stamp)).toBeGreaterThanOrEqual(4.5);
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
  await page.locator(".graph-filter-panel summary").click();
  await page.locator('input[name="search"]').fill("no-such-issue-at-all");
  await expect(page.locator(".view-empty")).toBeVisible();
  await expect(page.locator(".atlas-publication")).toHaveText(published);
});

test("keeps one publication instant across every planner report", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 844 });
  await page.goto("/issues/planner/");
  const published = await expectPublicationStamp(page);
  for (const tab of await page.locator(".view-tab").all()) {
    expect(await contrastRatio(tab)).toBeGreaterThanOrEqual(4.5);
    expect(await tab.evaluate((element) => element.scrollWidth)).toBeLessThanOrEqual(await tab.evaluate((element) => element.clientWidth));
  }
  for (const view of ["Queue", "Statistics", "Runway", "Priority"]) {
    await page.getByRole("tab", { name: view, exact: true }).click();
    await expect(page.locator(".atlas-publication")).toHaveText(published);
  }
});

test("keeps planner tabs and publication text separate at every narrow width", async ({ page }) => {
  for (const width of [320, 601, 768, 900, 1024]) {
    await page.setViewportSize({ width, height: 844 });
    await page.goto("/issues/planner/");
    await expectPublicationStamp(page);
    for (const tab of await page.locator(".view-tab").all()) {
      expect(await tab.evaluate((element) => element.scrollWidth), `${width}px ${await tab.textContent()}`).toBeLessThanOrEqual(await tab.evaluate((element) => element.clientWidth));
    }
  }
});

test("retains the publication instant after the statistics redirect", async ({ page }) => {
  await page.goto("/issues/statistics/");
  await expect(page).toHaveURL(/\/issues\/planner\/\?view=statistics$/);
  await expectPublicationStamp(page);
});
