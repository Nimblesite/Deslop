import { expect, test } from "@playwright/test";
import { expectAssetType } from "./helpers/assets.js";
import { advanceTo, expectMotionFits, expectScene, openMotion, SCENE_SECONDS } from "./helpers/motion.js";

// [SITE-HOME-VIDEO] [SITE-ASSET-TYPE] The reel opens the homepage on every screen, built from one script and one stylesheet.
const DESKTOP = { width: 1440, height: 900 };
const PHONE = { width: 390, height: 844 };
const TABLET = { width: 820, height: 1180 };
const WIDE_SHAPE = 9 / 16;
const SQUARE_SHAPE = 1;
const STAGE = ".motion-reel__stage";
const HEADLINE = ".hero h1";
const FULLY_VISIBLE = { ratio: 1 };
const TOP_OF_PAGE = 0;
const SCRIPT = "/assets/js/reel.js";
const STYLES = "/assets/css/reel.css";
const ASSETS = [[SCRIPT, "javascript"], [STYLES, "text/css"]];
const KIBIBYTE = 1024;
const MAX_REEL_BYTES = 32 * KIBIBYTE;
const NO_BYTES = 0;
const NO_MEDIA_REQUESTS = [];
const CHINESE_HOME = "/zh/";
const CHINESE_NAME = "Deslop 能做什么";
const CHINESE_CHAT = ["智能体 我来写一个 total() 辅助函数。", "Deslop 相似代码已经存在。", "智能体 那我就复用它。"];
const CHINESE_SCENES = [
  { id: "find", at: SCENE_SECONDS.find, headline: "查找重复代码。" },
  { id: "languages", at: SCENE_SECONDS.languages, headline: "覆盖九种语言。" },
  { id: "rank", at: SCENE_SECONDS.rank, headline: "最严重的重复排在最前。" },
  { id: "agents", at: SCENE_SECONDS.agents, headline: "阻止编码智能体制造更多重复。", items: [[".motion-reel__chat p", CHINESE_CHAT]] },
  { id: "brand", at: SCENE_SECONDS.brand, headline: "在 VS Code 中实时分析。" },
];
const CHINESE_CONTROLS = { play: "播放动画", pause: "暂停动画" };

function recordMedia(page) {
  const requests = new Set();
  page.on("request", (request) => {
    if (request.resourceType() === "media") requests.add(new URL(request.url()).pathname);
  });
  return requests;
}

async function expectAssets(page) {
  let totalBytes = NO_BYTES;
  for (const [path, type] of ASSETS) {
    await expectAssetType(page, path, type);
    totalBytes += (await (await page.request.get(path)).body()).byteLength;
  }
  expect(totalBytes).toBeLessThanOrEqual(MAX_REEL_BYTES);
}

async function boxes(page, reel) {
  return { stage: await reel.locator(STAGE).boundingBox(), headline: await page.locator(HEADLINE).boundingBox() };
}

async function expectFirstScreen(page, reel, viewport, shape) {
  await expectMotionFits(reel.locator(STAGE), viewport, shape);
  await expect(reel.locator(STAGE)).toBeInViewport(FULLY_VISIBLE);
  expect(await page.evaluate(() => window.scrollY)).toBe(TOP_OF_PAGE);
  await expect(reel).toHaveAttribute("data-playing", "true");
}

test("a desktop visitor sees the playing reel beside the headline in the first screen, without media downloads", async ({ page }) => {
  const media = recordMedia(page);
  await page.setViewportSize(DESKTOP);
  const reel = await openMotion(page);
  await expectFirstScreen(page, reel, DESKTOP, WIDE_SHAPE);
  const { stage, headline } = await boxes(page, reel);
  expect(stage.x).toBeGreaterThan(headline.x + headline.width);
  await expectAssets(page);
  expect([...media]).toEqual(NO_MEDIA_REQUESTS);
});

test("a phone visitor sees the square reel above the headline, and a portrait tablet keeps the wide reel on top", async ({ page }) => {
  await page.setViewportSize(PHONE);
  const reel = await openMotion(page);
  for (const [viewport, shape] of [[PHONE, SQUARE_SHAPE], [TABLET, WIDE_SHAPE]]) {
    await page.setViewportSize(viewport);
    await expectFirstScreen(page, reel, viewport, shape);
    const { stage, headline } = await boxes(page, reel);
    expect(stage.y + stage.height).toBeLessThan(headline.y);
  }
});

test("a Chinese-reading visitor gets the same reel with translated headlines and controls", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  const reel = await openMotion(page, CHINESE_HOME, CHINESE_NAME);
  const control = reel.getByRole("button");
  await expectFirstScreen(page, reel, DESKTOP, WIDE_SHAPE);
  await expect(control).toHaveAccessibleName(CHINESE_CONTROLS.pause);
  for (const scene of CHINESE_SCENES) {
    await advanceTo(page, reel, scene.at);
    await expectScene(reel, scene);
  }
  await control.click();
  await expect(control).toHaveAccessibleName(CHINESE_CONTROLS.play);
});
