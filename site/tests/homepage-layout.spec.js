import { expect, test } from "@playwright/test";

const PRIMARY_NAV_FONT_SIZE = "16px";
const SECONDARY_NAV_FONT_SIZE = "15px";
const NAV_LABEL_FONT_SIZE = "14px";

// [SITE-HOME-VIDEO] One compact introduction contains the real demo.
const HERO_RHYTHM_TOLERANCE_PX = 1;
const HERO_COPY_ELEMENTS = 5;
const MAX_HERO_HEIGHT = 850;
const MAX_NEXT_SECTION_GAP = 80;
test.use({ viewport: { width: 1920, height: 1080 } });

test("introduction and narrated demo share one compact hero", async ({ page }) => {
  for (const route of ['/', '/zh/']) {
    await page.goto(route);
    await expect(page.locator('.hero video')).toHaveCount(1);
    await expect(page.locator('.home video')).toHaveCount(1);
    await expect(page.locator('.home > .showcase')).toHaveCount(0);
    await expect(page.locator('.motion-reel')).toHaveCount(0);
    const bounds = await page.evaluate(() => {
      const hero = document.querySelector('.hero').getBoundingClientRect();
      const video = document.querySelector('.hero video').getBoundingClientRect();
      const next = document.querySelector('.testimonials').getBoundingClientRect();
      return {height:hero.height, videoBottom:video.bottom, gap:next.top-hero.bottom};
    });
    expect(bounds.height).toBeLessThan(MAX_HERO_HEIGHT);
    expect(bounds.videoBottom).toBeLessThan(1080);
    expect(bounds.gap).toBeLessThanOrEqual(MAX_NEXT_SECTION_GAP);
  }
});

test("desktop hero gives the product demonstration enough room", async ({ page }) => {
  await page.goto('/');
  const [copy, video] = await Promise.all([
    page.locator('.hero__left').boundingBox(), page.locator('.hero video').boundingBox()
  ]);
  expect(video.x).toBeGreaterThan(copy.x + copy.width);
  expect(video.width).toBeGreaterThan(copy.width);
  expect(Math.abs(video.y + video.height / 2 - copy.y - copy.height / 2)).toBeLessThanOrEqual(2);
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

  await page.goto("/releases/");
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
