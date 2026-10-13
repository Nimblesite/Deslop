import { expect } from "@playwright/test";

const PLAY = "Play animation";
const PAUSE = "Pause animation";
const START_SECONDS = 0;
const REEL_SECONDS = 21;
const ONE_PLAYER = 1;
const SHAPE_DIGITS = 2;
const HOME_ROUTE = "/";
const REEL_TITLE = "Now it finds the whole copy.";
const NAVIGATION = { waitUntil: "domcontentloaded" };

export async function openMotion(page, route = HOME_ROUTE, title = REEL_TITLE) {
  await page.goto(route, NAVIGATION);
  return page.getByRole("region", { name: title });
}

export async function expectAudioProperties(reel, properties) {
  for (const [property, value] of Object.entries(properties)) {
    await expect(reel.locator("audio")).toHaveJSProperty(property, value);
  }
}

export async function expectOpeningScene(reel) {
  await expectAudioProperties(reel, { paused: true, currentTime: START_SECONDS });
  await expect(reel.locator('[data-scene="whole"]')).toBeVisible();
}

async function expectPlaybackState(reel, audio, label, paused) {
  await reel.getByRole("button", { name: label, exact: true }).click();
  await expect(audio).toHaveJSProperty("paused", paused);
}

// [SITE-HOME-VIDEO] Exercise the real soundtrack and JavaScript playback controls.
export async function expectPlayback(reel, labels = { play: PLAY, pause: PAUSE }) {
  const audio = reel.locator("audio");
  await expectPlaybackState(reel, audio, labels.play, false);
  await expect(audio).toHaveJSProperty("muted", false);
  await expect.poll(() => audio.evaluate((element) => element.currentTime)).toBeGreaterThan(START_SECONDS);
  await expectPlaybackState(reel, audio, labels.pause, true);
  const pausedAt = await audio.evaluate((element) => element.currentTime);
  await expect(audio).toHaveJSProperty("currentTime", pausedAt);
  expect(await audio.evaluate((element) => element.duration)).toBeCloseTo(REEL_SECONDS, START_SECONDS);
}

// [SITE-HOME-VIDEO] Verify the entire responsive stage stays inside the viewport.
export async function expectMotionFits(stage, viewport, shape) {
  await expect(stage).toHaveCount(ONE_PLAYER);
  await expect(stage).toBeVisible();
  const box = await stage.boundingBox();
  expect(box.x).toBeGreaterThanOrEqual(START_SECONDS);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  expect(box.height / box.width).toBeCloseTo(shape, SHAPE_DIGITS);
}
