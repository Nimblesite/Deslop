import { expect, test } from "@playwright/test";

/* The 0.35.0 reel ships in two cuts: the wide one on a desktop, the tall one on
   a phone. Each arrives as AV1 where the browser decodes it and H.264 where it
   does not, fetches nothing before play, and plays with sound. The expected
   files are written out here rather than read from _data/reel.json so the test
   states them independently of the page that uses them. */
const REEL_TITLE = "Now it finds the whole copy.";
const REEL_NAME = "Deslop 0.35.0: now it finds the whole copy";
const ACCURACY_LINK = "How we measure accuracy";
const ACCURACY_POST = "/blog/duplicate-code-detection-accuracy/";
const CHINESE_HOME = "/zh/";
const CHINESE_REEL_TITLE = "现在它能找到完整副本。";
const CHINESE_ACCURACY_LINK = "我们如何衡量准确性";
const CHINESE_ACCURACY_POST = "/zh/blog/duplicate-code-detection-accuracy/";
const AV1_TYPE = "video/mp4; codecs=av01.0.08M.08";
const VIDEO_MIME_TYPE = "video/mp4";
const POSTER_MIME_TYPE = "image/jpeg";
const REEL_SECONDS = 21;
const REEL_ISO_DURATION = "PT21S";
const WHOLE_SECOND_DIGITS = 0;
const HAVE_NOTHING = 0;
const SHAPE_DIGITS = 2;
const SITE_ORIGIN = "https://deslop.live";

const DESKTOP_VIEWPORT = { width: 1440, height: 900 };
const PHONE_VIEWPORT = { width: 390, height: 844 };
const LANDSCAPE_TABLET_VIEWPORT = { width: 1024, height: 768 };

const CUTS = {
  desktop: {
    video: ".reel--desktop video",
    width: 1280,
    height: 720,
    av1: "/assets/video/whole-copy-desktop-av1.mp4",
    h264: "/assets/video/whole-copy-desktop.mp4",
    poster: "/assets/video/whole-copy-desktop-poster.jpg",
  },
  mobile: {
    video: ".reel--mobile video",
    width: 720,
    height: 1280,
    av1: "/assets/video/whole-copy-mobile-av1.mp4",
    h264: "/assets/video/whole-copy-mobile.mp4",
    poster: "/assets/video/whole-copy-mobile-poster.jpg",
  },
};

const readProperty = (locator, name) => locator.evaluate((element, property) => element[property], name);

const openReel = async (page, viewport, { route = "/", title = REEL_TITLE } = {}) => {
  await page.setViewportSize(viewport);
  await page.goto(route);
  const reel = page.getByRole("region", { name: title });
  return {
    reel,
    desktop: reel.locator(CUTS.desktop.video),
    mobile: reel.locator(CUTS.mobile.video),
  };
};

// Nothing is fetched until the visitor asks: the poster stands in for the film.
const expectWaiting = async (video, cut) => {
  await video.scrollIntoViewIfNeeded();
  await expect(video).toBeInViewport();
  await expect(video).toHaveAttribute("poster", cut.poster);
  await expect(video).toHaveAttribute("preload", "none");
  await expect(video).toHaveJSProperty("paused", true);
  await expect(video).toHaveJSProperty("readyState", HAVE_NOTHING);
};

// Every file the cut names is served with the type the browser needs.
const expectServed = async (page, cut) => {
  for (const [path, type] of [[cut.av1, VIDEO_MIME_TYPE], [cut.h264, VIDEO_MIME_TYPE], [cut.poster, POSTER_MIME_TYPE]]) {
    const response = await page.request.get(path);
    expect(response.ok(), path).toBeTruthy();
    expect(response.headers()["content-type"], path).toContain(type);
  }
};

// The visitor presses play: the lightest file this browser decodes plays, with sound, at the cut's size.
const expectPlays = async (page, video, cut) => {
  await video.focus();
  await page.keyboard.press("Space");
  await expect(video).toHaveJSProperty("paused", false);
  await expect(video).toHaveJSProperty("muted", false);
  const decodesAv1 = await video.evaluate((element, type) => element.canPlayType(type) !== "", AV1_TYPE);
  await expect.poll(() => readProperty(video, "currentSrc")).toContain(decodesAv1 ? cut.av1 : cut.h264);
  await expect.poll(() => readProperty(video, "currentTime")).toBeGreaterThan(0);
  await expect(video).toHaveJSProperty("videoWidth", cut.width);
  await expect(video).toHaveJSProperty("videoHeight", cut.height);
  expect(await readProperty(video, "duration")).toBeCloseTo(REEL_SECONDS, WHOLE_SECOND_DIGITS);
};

// A second press pauses it where it was.
const expectPauses = async (page, video) => {
  await page.keyboard.press("Space");
  await expect(video).toHaveJSProperty("paused", true);
  const pausedAt = await readProperty(video, "currentTime");
  expect(pausedAt).toBeGreaterThan(0);
  await expect(video).toHaveJSProperty("currentTime", pausedAt);
};

// The player sits inside the screen at the cut's own shape, so nothing is cropped or letterboxed.
const expectFits = async (video, cut, viewport) => {
  const box = await video.boundingBox();
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
  expect(box.height / box.width).toBeCloseTo(cut.height / cut.width, SHAPE_DIGITS);
};

test("a desktop visitor plays the wide cut with sound, pauses it and finds the accuracy post", async ({ page }) => {
  const { reel, desktop, mobile } = await openReel(page, DESKTOP_VIEWPORT);
  await expectWaiting(desktop, CUTS.desktop);
  await expect(mobile).toBeHidden();
  await expectFits(desktop, CUTS.desktop, DESKTOP_VIEWPORT);
  await expectServed(page, CUTS.desktop);
  await expectPlays(page, desktop, CUTS.desktop);
  await expectPauses(page, desktop);

  await expect(reel.getByRole("link", { name: ACCURACY_LINK })).toHaveAttribute("href", ACCURACY_POST);
  const video = await page.evaluate(() =>
    [...document.querySelectorAll('script[type="application/ld+json"]')]
      .map((node) => JSON.parse(node.textContent))
      .find((entity) => entity["@type"] === "VideoObject"),
  );
  expect(video.name).toBe(REEL_NAME);
  expect(video.contentUrl).toBe(`${SITE_ORIGIN}${CUTS.desktop.h264}`);
  expect(video.thumbnailUrl).toBe(`${SITE_ORIGIN}${CUTS.desktop.poster}`);
  expect(video.duration).toBe(REEL_ISO_DURATION);
});

test("a phone visitor plays the tall cut, and turning to a wide screen swaps in the wide cut", async ({ page }) => {
  const { desktop, mobile } = await openReel(page, PHONE_VIEWPORT);
  await expectWaiting(mobile, CUTS.mobile);
  await expect(desktop).toBeHidden();
  await expectServed(page, CUTS.mobile);
  await expectPlays(page, mobile, CUTS.mobile);
  await expectPauses(page, mobile);
  await expectFits(mobile, CUTS.mobile, PHONE_VIEWPORT);

  await page.setViewportSize(LANDSCAPE_TABLET_VIEWPORT);
  await expectWaiting(desktop, CUTS.desktop);
  await expect(mobile).toBeHidden();
  await expectFits(desktop, CUTS.desktop, LANDSCAPE_TABLET_VIEWPORT);
});

test("a Chinese-reading visitor gets the same reel under a Chinese heading, linked to the Chinese accuracy post", async ({ page }) => {
  const { reel, desktop, mobile } = await openReel(page, DESKTOP_VIEWPORT, { route: CHINESE_HOME, title: CHINESE_REEL_TITLE });
  await expectWaiting(desktop, CUTS.desktop);
  await expect(mobile).toBeHidden();
  await expect(reel.getByRole("link", { name: CHINESE_ACCURACY_LINK })).toHaveAttribute("href", CHINESE_ACCURACY_POST);
  await expectPlays(page, desktop, CUTS.desktop);
  await expectPauses(page, desktop);
});
