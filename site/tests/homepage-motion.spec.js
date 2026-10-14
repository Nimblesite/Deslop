import { expect, test } from "@playwright/test";
import { advanceTo, expectScene, openMotion, reelPosition, OPENING_SECONDS, REEL_NAME, SCENE_SECONDS } from "./helpers/motion.js";

// [SITE-HOME-VIDEO] The homepage reel loops Deslop's headlines in the page, behind one faint pause icon.
const DESKTOP = { width: 1440, height: 900 };
const LANGUAGES = ["C#", "Rust", "Python", "Dart", "JavaScript", "TypeScript", "PHP", "F#", "Go"];
const RANKS = ["1", "2", "3", "4"];
const CHAT = ["Agent I'll write a total() helper.", "Deslop Similar code already exists.", "Agent Then I'll reuse it."];
const TWO_MATCHES = ["", ""];
const FOUR_BARS = ["", "", "", ""];
const SCENES = [
  { id: "find", at: SCENE_SECONDS.find, headline: "Find duplicate code.", items: [[".motion-reel__copy b", TWO_MATCHES]] },
  { id: "languages", at: SCENE_SECONDS.languages, headline: "Nine languages.", items: [["li", LANGUAGES]] },
  { id: "rank", at: SCENE_SECONDS.rank, headline: "Worst offenders first.", items: [["li", RANKS], ["li i", FOUR_BARS]] },
  { id: "agents", at: SCENE_SECONDS.agents, headline: "Stop coding agents from creating more.", items: [[".motion-reel__chat p", CHAT]] },
  { id: "brand", at: SCENE_SECONDS.brand, headline: "Live in VS Code.", items: [[".motion-reel__wordmark", ["Deslop"]]] },
];
const FIRST = 0;
const LANGUAGES_SCENE = '[data-scene="languages"]';
const ENTERING_SECONDS = 5.2;
const PLAY = "Play animation";
const PAUSE = "Pause animation";
const ONE_CONTROL = 1;
const NO_MEDIA = 0;
const NO_TEXT = "";
const FAINT = "0.4";
const HOLD_SECONDS = 5;
const HOLD_MILLISECONDS = HOLD_SECONDS * 1000;
const FRAME_DIGITS = 1;
const STILL_DIGITS = 6;
const HIDDEN = 0;
const VISIBLE = 1;
const SETTLED = 0;
const REEL_TEXT = "rgb(238, 238, 238)";
const REEL_SURFACE = "rgb(23, 23, 23)";
const THEME_CONTROL = "Toggle dark mode";

function entry(scene) {
  return scene.evaluate((element) => {
    const style = getComputedStyle(element);
    return { opacity: Number(style.opacity), lift: new DOMMatrixReadOnly(style.transform).m42 };
  });
}

async function setTabHidden(page, hidden) {
  await page.evaluate((value) => {
    Object.defineProperty(document, "hidden", { configurable: true, get: () => value });
    document.dispatchEvent(new Event("visibilitychange"));
  }, hidden);
}

async function expectHeld(page, reel, seconds) {
  await expect(reel).toHaveAttribute("data-playing", "false");
  await page.clock.runFor(HOLD_MILLISECONDS);
  expect(await reelPosition(reel)).toBeCloseTo(seconds, STILL_DIGITS);
}

async function expectRunning(page, reel, seconds) {
  await expect(reel).toHaveAttribute("data-playing", "true");
  await page.clock.runFor(HOLD_MILLISECONDS);
  expect(await reelPosition(reel)).toBeCloseTo(seconds + HOLD_SECONDS, FRAME_DIGITS);
}

async function expectPauseAndResume(page, reel, control) {
  await control.click();
  await expect(control).toHaveAccessibleName(PLAY);
  const pausedAt = await reelPosition(reel);
  await expectHeld(page, reel, pausedAt);
  await control.click();
  await expect(control).toHaveAccessibleName(PAUSE);
  await expectRunning(page, reel, pausedAt);
}

test("the homepage reel plays every headline in order, loops and pauses from one faint icon", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  const reel = await openMotion(page);
  const control = reel.getByRole("button");
  await expect(reel).toHaveAttribute("data-playing", "true");
  expect(await reelPosition(reel)).toBe(OPENING_SECONDS);
  await expect(reel.locator("video, audio, input")).toHaveCount(NO_MEDIA);
  await expect(control).toHaveCount(ONE_CONTROL);
  await expect(control).toHaveAccessibleName(PAUSE);
  await expect(control).toHaveText(NO_TEXT);
  await expect(control).toHaveCSS("opacity", FAINT);
  for (const scene of [...SCENES, SCENES[FIRST]]) {
    await advanceTo(page, reel, scene.at);
    await expectScene(reel, scene);
  }
  await expectPauseAndResume(page, reel, control);
});

test("each scene rises into place as it fades in", async ({ page }) => {
  const reel = await openMotion(page);
  await expect(reel).toHaveAttribute("data-playing", "true");
  await advanceTo(page, reel, ENTERING_SECONDS);
  const { opacity, lift } = await entry(reel.locator(LANGUAGES_SCENE));
  expect(opacity).toBeGreaterThan(HIDDEN);
  expect(opacity).toBeLessThan(VISIBLE);
  expect(lift).toBeGreaterThan(SETTLED);
});

test("reduced motion holds the finished opening scene until the visitor presses play, then scenes only fade", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  const reel = await openMotion(page);
  const control = reel.getByRole("button");
  await expect(control).toHaveAccessibleName(PLAY);
  await expectHeld(page, reel, OPENING_SECONDS);
  await expectScene(reel, SCENES[FIRST]);
  await control.click();
  await expect(reel).toHaveAttribute("data-playing", "true");
  await advanceTo(page, reel, ENTERING_SECONDS);
  const { opacity, lift } = await entry(reel.locator(LANGUAGES_SCENE));
  expect(opacity).toBeGreaterThan(HIDDEN);
  expect(opacity).toBeLessThan(VISIBLE);
  expect(lift).toBe(SETTLED);
});

test("the reel stops off screen and in a hidden tab, then resumes where it stopped", async ({ page }) => {
  const reel = await openMotion(page);
  await expect(reel).toHaveAttribute("data-playing", "true");
  await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
  const parkedAt = await reelPosition(reel);
  await expectHeld(page, reel, parkedAt);
  await page.evaluate(() => window.scrollTo(0, 0));
  await expect(reel).toHaveAttribute("data-playing", "true");
  await setTabHidden(page, true);
  await expectHeld(page, reel, parkedAt);
  await setTabHidden(page, false);
  await expectRunning(page, reel, parkedAt);
});

test("the dark reel remains readable in both website themes", async ({ page }) => {
  const reel = await openMotion(page);
  await advanceTo(page, reel, SCENE_SECONDS.agents);
  const headline = reel.locator('[data-scene="agents"] .motion-reel__headline');
  const message = reel.locator('[data-scene="agents"] .motion-reel__chat p').first();
  for (const theme of ["initial", "toggled"]) {
    await expect(headline, theme).toHaveCSS("color", REEL_TEXT);
    await expect(message, theme).toHaveCSS("color", REEL_TEXT);
    await expect(message, theme).toHaveCSS("background-color", REEL_SURFACE);
    await page.getByRole("button", { name: THEME_CONTROL }).click();
  }
});

test.describe("without JavaScript", () => {
  test.use({ javaScriptEnabled: false });
  test("the finished opening scene stands still with no dead control", async ({ page }) => {
    await page.goto("/");
    const reel = page.getByRole("figure", { name: REEL_NAME });
    await expectScene(reel, SCENES[FIRST]);
    await expect(reel.locator("button")).toBeHidden();
  });
});
