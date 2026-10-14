import { expect } from "@playwright/test";

const HOME_ROUTE = "/";
export const REEL_NAME = "What Deslop does";
export const OPENING_SECONDS = 2;
// [SITE-HOME-VIDEO] A moment in each scene after everything in it has entered.
export const SCENE_SECONDS = { find: 2.5, languages: 7, rank: 11, agents: 16.5, brand: 20 };
const CLOCK_ORIGIN = new Date("2026-01-01T00:00:00Z");
const MILLISECONDS_PER_SECOND = 1000;
const REEL_SECONDS = 21;
const ONE_PLAYER = 1;
const ONE_SCENE = 1;
const SHAPE_DIGITS = 2;
const LEFT_EDGE = 0;
const SHOWN = "1";

// [SITE-HOME-VIDEO] Open a homepage reel on a frozen page clock, so every frame is reached deterministically.
export async function openMotion(page, route = HOME_ROUTE, name = REEL_NAME) {
  await page.clock.install({ time: CLOCK_ORIGIN });
  await page.clock.pauseAt(CLOCK_ORIGIN);
  await page.goto(route);
  return page.getByRole("figure", { name });
}

export async function reelPosition(reel) {
  return Number(await reel.getAttribute("data-position"));
}

// [SITE-HOME-VIDEO] Run the frozen clock forward until the reel reaches a second of its loop.
export async function advanceTo(page, reel, seconds) {
  const elapsed = (seconds - (await reelPosition(reel)) + REEL_SECONDS) % REEL_SECONDS;
  await page.clock.runFor(Math.round(elapsed * MILLISECONDS_PER_SECOND));
}

// [SITE-HOME-VIDEO] Exactly one scene is shown, its headline reads in full and every element of it has finished entering.
export async function expectScene(reel, { id, headline, items = [] }) {
  const scene = reel.locator(`[data-scene="${id}"]`);
  const shown = reel.locator('[data-scene][aria-hidden="false"]');
  await expect(shown).toHaveCount(ONE_SCENE);
  await expect(shown).toHaveAttribute("data-scene", id);
  for (const other of await reel.locator(`[data-scene]:not([data-scene="${id}"])`).all()) await expect(other).toBeHidden();
  await expect(scene.locator(".motion-reel__headline")).toBeVisible();
  await expect(scene.locator(".motion-reel__headline")).toHaveText(headline);
  await expect(scene).toHaveCSS("opacity", SHOWN);
  for (const [selector, texts] of items) {
    await expect(scene.locator(selector)).toHaveText(texts);
    for (const element of await scene.locator(selector).all()) await expect(element).toHaveCSS("opacity", SHOWN);
  }
}

// [SITE-HOME-VIDEO] Verify the entire responsive stage stays inside the viewport.
export async function expectMotionFits(stage, viewport, shape) {
  await expect(stage).toHaveCount(ONE_PLAYER);
  await expect(stage).toBeVisible();
  const box = await stage.boundingBox();
  expect(box.x).toBeGreaterThanOrEqual(LEFT_EDGE);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  expect(box.height / box.width).toBeCloseTo(shape, SHAPE_DIGITS);
}
