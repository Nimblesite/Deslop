import { expect, test } from "@playwright/test";

test.use({ viewport: { width: 1920, height: 1080 } });

test("homepage hero fills the desktop viewport", async ({ page }) => {
  await page.goto("/");
  const heroBottom = await page
    .locator(".hero")
    .evaluate((element) => element.getBoundingClientRect().bottom);
  const viewportHeight = await page.evaluate(() => window.innerHeight);

  expect(
    heroBottom,
    "homepage hero must reach the bottom of the desktop viewport",
  ).toBeGreaterThanOrEqual(viewportHeight);
});
