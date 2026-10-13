import { expect, test } from "@playwright/test";
import { expectAssetType } from "./helpers/assets.js";
import { expectPlayback, expectMotionFits, openMotion, expectAudioProperties } from "./helpers/motion.js";

// [SITE-HOME-VIDEO] [SITE-ASSET-TYPE] JS motion replaces both encoded video cuts.
const TITLE = "Now it finds the whole copy.";
const REEL_NAME = "Deslop 0.35.0: now it finds the whole copy";
const ACCURACY_LINK = "How we measure accuracy";
const ACCURACY_POST = "/blog/duplicate-code-detection-accuracy/";
const CHINESE_HOME = "/zh/";
const CHINESE_TITLE = "现在它能找到完整副本。";
const CHINESE_LINK = "我们如何衡量准确性";
const CHINESE_POST = "/zh/blog/duplicate-code-detection-accuracy/";
const CHINESE_CONTROLS = { play: "播放动画", pause: "暂停动画" };
const DESKTOP = { width: 1440, height: 900 };
const PHONE = { width: 390, height: 844 };
const TABLET = { width: 1024, height: 768 };
const DESKTOP_SHAPE = 720 / 1280;
const PHONE_SHAPE = 1280 / 720;
const STAGE = ".motion-reel__stage";
const AUDIO = "/assets/audio/whole-copy.m4a";
const DESKTOP_POSTER = "/assets/video/whole-copy-desktop-poster.jpg";
const MOBILE_POSTER = "/assets/video/whole-copy-mobile-poster.jpg";
const SCRIPT = "/assets/js/reel.js";
const STYLES = "/assets/css/reel.css";
const ASSETS = [[SCRIPT, "javascript"], [STYLES, "text/css"], [AUDIO, "audio/mp4"], [DESKTOP_POSTER, "image/jpeg"], [MOBILE_POSTER, "image/jpeg"]];
const KIBIBYTE = 1024;
const MAX_REEL_BYTES = 512 * KIBIBYTE;
const NO_MEDIA_REQUESTS = [];
const SITE_ORIGIN = "https://deslop.live";
const DURATION = "PT21S";
const HAVE_NOTHING = 0;

async function openReel(page, viewport, route = "/", title = TITLE) {
  await page.setViewportSize(viewport);
  const reel = await openMotion(page, route, title);
  await reel.locator(STAGE).scrollIntoViewIfNeeded();
  await expect(reel.locator(STAGE)).toBeInViewport();
  await expect(reel.locator("audio")).toHaveAttribute("preload", "none");
  await expectAudioProperties(reel, { paused: true, readyState: HAVE_NOTHING });
  return reel;
}

async function expectAssets(page) {
  let totalBytes = HAVE_NOTHING;
  for (const [path, type] of ASSETS) {
    await expectAssetType(page, path, type);
    totalBytes += (await (await page.request.get(path)).body()).byteLength;
  }
  expect(totalBytes).toBeLessThanOrEqual(MAX_REEL_BYTES);
}

async function expectMetadata(page) {
  const motion = await page.evaluate(() =>
    [...document.querySelectorAll('script[type="application/ld+json"]')]
      .map((node) => JSON.parse(node.textContent))
      .find((entity) => entity["@type"] === "CreativeWork"),
  );
  expect(motion.name).toBe(REEL_NAME);
  expect(motion.url).toBe(`${SITE_ORIGIN}/#release-reel`);
  expect(motion.image).toBe(`${SITE_ORIGIN}${DESKTOP_POSTER}`);
  expect(motion.timeRequired).toBe(DURATION);
}

function recordMedia(page) {
  const requests = new Set();
  page.on("request", (request) => {
    if (request.resourceType() === "media") requests.add(new URL(request.url()).pathname);
  });
  return requests;
}

async function expectCut(reel, viewport, shape, poster) {
  await expectMotionFits(reel.locator(STAGE), viewport, shape);
  await expect.poll(() => reel.locator("img").evaluate((image) => image.currentSrc)).toContain(poster);
}

test("a desktop visitor plays the wide motion reel with sound, pauses it and finds the accuracy post", async ({ page }) => {
  const media = recordMedia(page);
  const reel = await openReel(page, DESKTOP);
  expect([...media]).toEqual(NO_MEDIA_REQUESTS);
  await expectCut(reel, DESKTOP, DESKTOP_SHAPE, DESKTOP_POSTER);
  await expectAssets(page);
  await expectPlayback(reel);
  expect([...media]).toEqual([AUDIO]);
  await expect(reel.getByRole("link", { name: ACCURACY_LINK })).toHaveAttribute("href", ACCURACY_POST);
  await expectMetadata(page);
});

test("a phone visitor plays the tall motion reel and rotates to the wide layout", async ({ page }) => {
  const reel = await openReel(page, PHONE);
  await expectCut(reel, PHONE, PHONE_SHAPE, MOBILE_POSTER);
  await expectPlayback(reel);
  await page.setViewportSize(TABLET);
  await expectCut(reel, TABLET, DESKTOP_SHAPE, DESKTOP_POSTER);
});

test("a Chinese-reading visitor plays the same motion reel with translated controls and accuracy link", async ({ page }) => {
  const reel = await openReel(page, DESKTOP, CHINESE_HOME, CHINESE_TITLE);
  await expectMotionFits(reel.locator(STAGE), DESKTOP, DESKTOP_SHAPE);
  await expect(reel.getByRole("link", { name: CHINESE_LINK })).toHaveAttribute("href", CHINESE_POST);
  await expectPlayback(reel, CHINESE_CONTROLS);
});
