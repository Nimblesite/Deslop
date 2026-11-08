import { expect, test } from '@playwright/test';
import { expectAssetType } from './helpers/assets.js';

// [SITE-HOME-VIDEO] One actual product recording shares the introduction.
const VIEWPORTS = [{width:1440,height:900},{width:820,height:1180},{width:390,height:844}];
const HEADER_HEIGHT = 72;
const ASPECT_RATIO = 1600 / 1000;
const ROUTES = [{path:'/',label:'Deslop product demo',track:'English captions'}, {path:'/zh/',label:'Deslop 产品演示',track:'英语字幕'}];

for (const route of ROUTES) {
  test(`one responsive demo belongs to the hero (${route.path})`, async ({ page }) => {
    for (const viewport of VIEWPORTS) {
      await page.setViewportSize(viewport);
      await page.goto(route.path);
      const video = page.locator('.hero video');
      await expect(video).toHaveCount(1);
      await expect(video).toHaveAttribute('aria-label', new RegExp(route.label));
      await expect(video).toHaveAttribute('playsinline', '');
      await expect(video).toHaveAttribute('preload', 'none');
      await expect(video.locator('track')).toHaveAttribute('label', route.track);
      const box = await video.boundingBox();
      const copy = await page.locator('.hero__left').boundingBox();
      expect(box.width / box.height).toBeCloseTo(ASPECT_RATIO);
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(viewport.width);
      expect(copy.y).toBeGreaterThanOrEqual(HEADER_HEIGHT);
      if (viewport.width >= 1024) {
        expect(box.x).toBeGreaterThan(copy.x + copy.width);
        expect(box.y + box.height).toBeLessThan(viewport.height);
      } else {
        expect(box.y).toBeGreaterThanOrEqual(copy.y + copy.height);
        expect(box.y - copy.y - copy.height).toBeLessThanOrEqual(32);
      }
    }
  });
}

test('the poster and captions load correctly without downloading the film on arrival', async ({ page }) => {
  const media = [];
  page.on('request', request => { if (request.url().endsWith('.mp4')) media.push(request.url()); });
  await page.goto('/');
  await expectAssetType(page, '/assets/img/deslop-demo-poster.webp', 'image/webp');
  const captions = await page.request.get('/assets/video/deslop-demo.vtt');
  expect(captions.ok()).toBe(true);
  expect(await captions.text()).toContain('WEBVTT');
  expect(media).toEqual([]);
  await page.locator('.hero__cta a[href="#demo"]').click();
  await expect(page.locator('.hero video')).toBeInViewport();
});
