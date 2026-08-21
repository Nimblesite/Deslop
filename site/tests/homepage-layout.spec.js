import { expect, test } from "@playwright/test";

const PRIMARY_NAV_FONT_SIZE = "16px";
const SECONDARY_NAV_FONT_SIZE = "15px";
const NAV_LABEL_FONT_SIZE = "13px";

/* The hero is a single centred block: equal air above and below the copy, the
   product shot sharing its centre line, and one repeated gap between every
   element of the copy. Tolerances absorb sub-pixel layout rounding only. */
const HERO_CENTRING_TOLERANCE_PX = 2;
const HERO_RHYTHM_TOLERANCE_PX = 1;
const HERO_OVERFLOW_TOLERANCE_PX = 1;
const HERO_COPY_ELEMENTS = 5;

test.use({ viewport: { width: 1920, height: 1080 } });

test("homepage hero fills the desktop viewport exactly", async ({ page }) => {
  await page.goto("/");
  const heroBottom = await page
    .locator(".hero")
    .evaluate((element) => element.getBoundingClientRect().bottom);
  const viewportHeight = await page.evaluate(() => window.innerHeight);

  expect(
    heroBottom,
    "homepage hero must reach the bottom of the desktop viewport",
  ).toBeGreaterThanOrEqual(viewportHeight);
  expect(
    heroBottom - viewportHeight,
    "homepage hero must not overshoot the viewport and force a scroll",
  ).toBeLessThanOrEqual(HERO_OVERFLOW_TOLERANCE_PX);
});

test("centres the hero copy and product shot in the viewport", async ({ page }) => {
  await page.goto("/");
  const hero = await page.locator(".hero").evaluate((element) => {
    const box = (selector) => element.querySelector(selector).getBoundingClientRect();
    const outer = element.getBoundingClientRect();
    const copy = box(".hero__left");
    const shot = box(".hero__right");
    return {
      airAbove: copy.top - outer.top,
      airBelow: outer.bottom - copy.bottom,
      copyCentre: (copy.top + copy.bottom) / 2,
      shotCentre: (shot.top + shot.bottom) / 2,
    };
  });

  expect(
    Math.abs(hero.airAbove - hero.airBelow),
    `hero copy must sit centred: ${hero.airAbove}px above vs ${hero.airBelow}px below`,
  ).toBeLessThanOrEqual(HERO_CENTRING_TOLERANCE_PX);
  expect(
    Math.abs(hero.copyCentre - hero.shotCentre),
    "product shot must share the hero copy's centre line",
  ).toBeLessThanOrEqual(HERO_CENTRING_TOLERANCE_PX);
});

test("spaces every hero element on one rhythm", async ({ page }) => {
  await page.goto("/");
  const { count, gaps } = await page.locator(".hero__left").evaluate((column) => {
    const rects = [...column.children].map((child) => child.getBoundingClientRect());
    return {
      count: rects.length,
      gaps: rects.slice(1).map((rect, index) => rect.top - rects[index].bottom),
    };
  });

  expect(count, "hero copy is chip, headline, lede, CTA and sublede").toBe(HERO_COPY_ELEMENTS);
  for (const [index, gap] of gaps.entries()) {
    expect(
      Math.abs(gap - gaps[0]),
      `hero gap ${index + 1} is ${gap}px but the first gap is ${gaps[0]}px`,
    ).toBeLessThanOrEqual(HERO_RHYTHM_TOLERANCE_PX);
  }
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
  expect(await fontSize(".blog-nav-link")).toBe(SECONDARY_NAV_FONT_SIZE);
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
