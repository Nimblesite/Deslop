import { expect, test } from "@playwright/test";
import { expectPlayback, expectMotionFits, openMotion, expectOpeningScene } from "./helpers/motion.js";

// [SITE-HOME-VIDEO] The release reel runs in the page, without encoded video.
const PLAYER = ".motion-reel";
const STAGE = ".motion-reel__stage";
const NO_VIDEO_PLAYERS = 0;
const ONE_PLAYER = 1;
const REEL_SECONDS = 21;
const GRADED_SECONDS = 10;
const START_SECONDS = 0;
const SOURCE_SCRIPT = "/assets/js/reel.js";
const REPLAY = "Restart animation";
const SEEK = "Animation position";
const DESKTOP = { width: 1440, height: 900 };
const PHONE = { width: 390, height: 844 };
const DESKTOP_SHAPE = 720 / 1280;
const PHONE_SHAPE = 1280 / 720;
const GRADED_ENTRY_SECONDS = 7.1;
const IDENTITY_TRANSFORM = "matrix(1, 0, 0, 1, 0, 0)";
const AUDIO_ROUTE = "**/assets/audio/whole-copy.m4a";
const PLAY = "Play animation";
const MEDIA_ERROR = "The soundtrack could not play.";
const NO_SCRIPT_MESSAGE = "Enable JavaScript to play the animation. The opening frame and accuracy article are available here.";
const REEL_TEXT = "rgb(238, 238, 238)";
const TRANSPARENT = "rgba(0, 0, 0, 0)";
const COMPARE_SECONDS = 16;
const THEME_CONTROL = "Toggle dark mode";
const NO_PLAYBACK_ERROR = "";

test("the homepage plays, pauses and seeks its JavaScript motion reel without video files", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  const reel = await openMotion(page);
  await expect(reel.locator("video")).toHaveCount(NO_VIDEO_PLAYERS);
  await expect(reel.locator(PLAYER)).toHaveCount(ONE_PLAYER);
  await expect(reel.locator(STAGE)).toBeVisible();
  await expect(page.locator(`script[src="${SOURCE_SCRIPT}"]`)).toHaveCount(ONE_PLAYER);
  await expect(reel.locator("audio")).toHaveJSProperty("paused", true);
  await expect(reel.getByRole("slider", { name: SEEK })).toHaveAttribute("max", String(REEL_SECONDS));
  await expectPlayback(reel);
  await expectPosition(reel, GRADED_SECONDS);
  await expect(reel.locator('[data-scene="graded"]')).toBeVisible();
  await reel.getByRole("button", { name: REPLAY, exact: true }).click();
  await expectOpeningScene(reel);
});

async function expectPosition(reel, seconds) {
  const slider = reel.getByRole("slider", { name: SEEK });
  await slider.fill(String(seconds));
  await expect(reel.locator("audio")).toHaveJSProperty("currentTime", seconds);
  await expect(reel.locator(PLAYER)).toHaveAttribute("data-position", String(seconds));
}

async function holdAudio(page) {
  const held = Promise.withResolvers();
  const observed = Promise.withResolvers();
  await page.route(AUDIO_ROUTE, async (route) => {
    observed.resolve();
    await held.promise;
    await route.continue();
  });
  return { started: observed.promise, release: held.resolve };
}

test("the same motion player fits desktop and phone screens without cropping", async ({ page }) => {
  await openMotion(page);
  for (const [viewport, shape] of [[DESKTOP, DESKTOP_SHAPE], [PHONE, PHONE_SHAPE]]) {
    await page.setViewportSize(viewport);
    await expectMotionFits(page.locator(STAGE), viewport, shape);
  }
});

test("reduced motion keeps scene changes still and playback waits for the visitor", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  const reel = await openMotion(page);
  await expect(reel.locator("audio")).toHaveJSProperty("paused", true);
  await expectPosition(reel, GRADED_ENTRY_SECONDS);
  const graded = reel.locator('[data-scene="graded"]');
  await expect(graded).toBeVisible();
  await expect(graded).toHaveCSS("transform", IDENTITY_TRANSFORM);
});

test("the dark reel remains readable in both website themes", async ({ page }) => {
  const reel = await openMotion(page);
  await expectPosition(reel, COMPARE_SECONDS);
  const heading = reel.locator('[data-scene="compare"] h3');
  const terminal = reel.locator('[data-scene="compare"] pre');
  for (const theme of ["initial", "toggled"]) {
    await expect(heading, theme).toHaveCSS("color", REEL_TEXT);
    await expect(terminal, theme).toHaveCSS("color", REEL_TEXT);
    await expect(terminal, theme).toHaveCSS("background-color", TRANSPARENT);
    await page.getByRole("button", { name: THEME_CONTROL }).click();
  }
});

test("an unavailable soundtrack reports the error and the scenes remain seekable", async ({ page }) => {
  await page.route(AUDIO_ROUTE, (route) => route.abort());
  const reel = await openMotion(page);
  await reel.getByRole("button", { name: PLAY, exact: true }).click();
  await expect(reel.locator("output")).toContainText(MEDIA_ERROR);
  await expectPosition(reel, GRADED_SECONDS);
  await expect(reel.locator('[data-scene="graded"]')).toBeVisible();
  await expect(reel.locator("audio")).toHaveJSProperty("paused", true);
});

test("restarting pending playback returns to the opening scene without a false audio error", async ({ page }) => {
  const audioRequest = await holdAudio(page);
  const reel = await openMotion(page);
  await reel.getByRole("button", { name: PLAY, exact: true }).click();
  await audioRequest.started;
  await reel.getByRole("button", { name: REPLAY, exact: true }).click();
  audioRequest.release();
  await expectOpeningScene(reel);
  await expect(reel.locator("output")).toHaveText(NO_PLAYBACK_ERROR);
});

test.describe("without JavaScript", () => {
  test.use({ javaScriptEnabled: false });
  test("the opening artwork and accuracy link remain available", async ({ page }) => {
    const reel = await openMotion(page);
    await expect(reel.locator('[data-scene="whole"]')).toBeVisible();
    await expect(reel.getByRole("link", { name: "How we measure accuracy" })).toHaveAttribute("href", "/blog/duplicate-code-detection-accuracy/");
    await expect(reel.getByRole("button", { name: PLAY, exact: true })).toBeDisabled();
    await expect(reel.locator("noscript")).toBeVisible();
    await expect(reel.locator("noscript")).toHaveJSProperty("textContent", NO_SCRIPT_MESSAGE);
  });
});
