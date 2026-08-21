import { expect, test } from "@playwright/test";

const PRIMARY_NAV_FONT_SIZE = "15px";
const SECONDARY_NAV_FONT_SIZE = "14px";
const NAV_LABEL_FONT_SIZE = "12px";

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

test("uses a consistent, readable navigation type scale", async ({ page }) => {
  const fontSize = (selector) =>
    page.locator(selector).first().evaluate((element) => getComputedStyle(element).fontSize);

  await page.goto("/");
  expect(await fontSize(".nav-link")).toBe(PRIMARY_NAV_FONT_SIZE);
  expect(await fontSize(".lang-link")).toBe(SECONDARY_NAV_FONT_SIZE);
  expect(await fontSize(".footer-section a")).toBe(SECONDARY_NAV_FONT_SIZE);

  await page.goto("/docs/");
  expect(await fontSize(".docs-sidebar__link")).toBe(SECONDARY_NAV_FONT_SIZE);
  expect(await fontSize(".docs-nav-group__summary")).toBe(NAV_LABEL_FONT_SIZE);
  expect(await fontSize(".docs-breadcrumb")).toBe(SECONDARY_NAV_FONT_SIZE);
  expect(await fontSize(".docs-sidebar__cta")).toBe(SECONDARY_NAV_FONT_SIZE);

  await page.goto("/blog/");
  expect(await fontSize(".post-card__more")).toBe(SECONDARY_NAV_FONT_SIZE);
});

test("keeps primary navigation readable in the mobile drawer", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await page.getByRole("button", { name: "Toggle menu" }).click();

  const mobileNavSize = await page
    .locator(".nav-link")
    .first()
    .evaluate((element) => getComputedStyle(element).fontSize);
  expect(mobileNavSize).toBe(PRIMARY_NAV_FONT_SIZE);
});
