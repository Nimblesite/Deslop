import { expect, test } from '@playwright/test';

// [SITE-HOME-VIDEO] The narrated demo replaces the automatic marketing reel.
const DEMO = '.hero video';
const SEEK_SECONDS = 12;
const END_MARGIN_SECONDS = 0.2;
const CAPTION_COUNT = 3;

test('the real demo plays, pauses, seeks, exposes captions and stops at its end', async ({ page }) => {
  await page.goto('/');
  const video = page.locator(DEMO);
  expect(await video.evaluate(element => element.paused)).toBe(true);
  await video.evaluate(element => { element.muted = true; return element.play(); });
  await expect.poll(() => video.evaluate(element => element.currentTime)).toBeGreaterThan(0);
  await video.evaluate(element => element.pause());
  expect(await video.evaluate(element => element.paused)).toBe(true);
  await expect.poll(() => video.evaluate(element => element.buffered.length ? element.buffered.end(element.buffered.length - 1) : 0)).toBeGreaterThan(SEEK_SECONDS);
  await video.evaluate((element, seconds) => new Promise(resolve => {
    element.textTracks[0].mode = 'showing';
    element.addEventListener('seeked', resolve, { once: true });
    element.currentTime = seconds;
  }), SEEK_SECONDS);
  await expect.poll(() => video.evaluate(element => element.seeking)).toBe(false);
  await expect.poll(() => video.evaluate(element => element.textTracks[0].cues?.length)).toBe(CAPTION_COUNT);
  const captions = await video.evaluate(element => Array.from(element.textTracks[0].cues, cue => cue.text));
  expect(captions[0]).toContain('Find duplicate code');
  expect(captions[1]).toContain('canonical');
  expect(captions[2]).toContain('Compare the copies side by side');
  expect(await video.evaluate(element => element.currentTime)).toBeCloseTo(SEEK_SECONDS);
  await video.evaluate(element => element.play());
  await expect.poll(() => video.evaluate(element => element.currentTime)).toBeGreaterThan(SEEK_SECONDS);
  await video.evaluate((element, margin) => { element.currentTime = element.duration - margin; }, END_MARGIN_SECONDS);
  await expect.poll(() => video.evaluate(element => element.ended)).toBe(true);
  expect(await video.evaluate(element => element.loop)).toBe(false);
});

test('reduced motion keeps both themes still until playback is requested', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const colorScheme of ['dark', 'light']) {
    await page.emulateMedia({ colorScheme });
    await page.goto('/');
    const video = page.locator(DEMO);
    expect(await video.evaluate(element => ({paused:element.paused,time:element.currentTime,autoplay:element.autoplay}))).toEqual({paused:true,time:0,autoplay:false});
    await expect(video).toHaveAttribute('poster', '/assets/img/deslop-demo-poster.webp');
    await expect(video).toHaveAttribute('controls', '');
    await expect(page.locator('.hero h1')).toBeVisible();
  }
});

test.describe('without JavaScript', () => {
  test.use({ javaScriptEnabled: false });
  test('the actual demo retains native controls, captions and a download fallback', async ({ page }) => {
    await page.goto('/');
    const video = page.locator(DEMO);
    await expect(video).toBeVisible();
    await expect(video).toHaveAttribute('controls', '');
    await expect(video.locator('source')).toHaveAttribute('src', '/assets/video/deslop-demo.mp4');
    await expect(video.locator('track')).toHaveAttribute('src', '/assets/video/deslop-demo.vtt');
    await expect(video.locator('a')).toHaveAttribute('href', '/assets/video/deslop-demo.mp4');
    await expect(page.locator('.hero__cta a').first()).toHaveAttribute('href', 'vscode:extension/Nimblesite.deslop-live');
  });
});
