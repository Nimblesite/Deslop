import {expect,test} from '@playwright/test';
import fs from 'node:fs';
const FILMS=JSON.parse(fs.readFileSync(new URL('../src/_data/docFilms.json',import.meta.url),'utf8'));
const WIDTHS=[320,390,768,1440];
const HEIGHT=900;
const SEEK_SECONDS=20;
const EXPECTED_CHAPTERS=4;
const PROBED_WIDTHS=[390,1440];
// [DOC-FILMS-DELIVERY] Actual delivered media, tracks and responsive controls.

async function assertPlayerAttributes(page,film) {
  const video=page.locator('.doc-film video');
  await expect(video).toHaveCount(1);
  await expect(video).toHaveAttribute('aria-label',film.title);
  await expect(video).toHaveAttribute('controls','');
  await expect(video).toHaveAttribute('preload','none');
  return video;
}

async function assertGeometry(page,video,width) {
  await page.setViewportSize({width,height:HEIGHT});
  await video.scrollIntoViewIfNeeded();
  const box=await video.boundingBox();
  expect(box.width).toBeGreaterThan(200);
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x+box.width).toBeLessThanOrEqual(width+1);
}

// Plays from the current position; asserting progress past `before` keeps the check
// meaningful when playback runs twice (390 and 1440) instead of only `> 0` once.
async function playAndAssertProgress(video) {
  expect(await video.evaluate(v=>v.paused)).toBe(true);
  const before=await video.evaluate(v=>v.currentTime);
  await video.evaluate(v=>{v.muted=true;return v.play();});
  await expect.poll(()=>video.evaluate(v=>v.currentTime)).toBeGreaterThan(before);
}

async function pauseAndHideTracks(video) {
  await video.evaluate(v=>{v.pause();for(const track of v.textTracks)track.mode='hidden';});
  await expect.poll(()=>video.evaluate(v=>v.textTracks[0].cues?.length)).toBe(EXPECTED_CHAPTERS);
  await expect.poll(()=>video.evaluate(v=>v.textTracks[1].cues?.length)).toBe(EXPECTED_CHAPTERS);
}

async function seekAndAssert(video) {
  await video.evaluate((v,time)=>{v.currentTime=time;},SEEK_SECONDS);
  await expect.poll(()=>video.evaluate(v=>v.seeking)).toBe(false);
  expect(await video.evaluate(v=>v.currentTime)).toBeCloseTo(SEEK_SECONDS);
  expect(await video.evaluate(v=>v.error)).toBeNull();
}

async function assertTranscript(page,film) {
  const transcript=page.locator('.doc-film-transcript');
  await transcript.locator('summary').focus();
  await page.keyboard.press('Enter');
  await expect(transcript.locator('section')).toHaveCount(EXPECTED_CHAPTERS);
  await expect(transcript.locator('p').first()).toHaveText(film.transcript[0].text);
}

async function assertDownloadLink(video,slug) {
  await expect(video.locator('a')).toHaveAttribute('href',`/assets/video/docs/deslop-docs-${slug}.mp4`);
}

test('getting started fits the reading column without prose CSS', async ({page}) => {
  await page.route('**/assets/css/prose.css', route => route.abort());
  await page.goto('/docs/');
  const video = page.locator('.doc-film video');
  const column = page.locator('.prose--docs');
  for (const width of WIDTHS) {
    await page.setViewportSize({width, height: HEIGHT});
    const columnBox = await column.boundingBox();
    const playerBox = await video.boundingBox();
    expect(playerBox.width).toBeCloseTo(columnBox.width, 0);
    expect(playerBox.x + playerBox.width).toBeLessThanOrEqual(width);
  }
});

function filmRoute(slug) {
  if (slug === 'osprey') return '/blog/osprey-first-class/';
  if (slug === 'getting-started') return '/docs/';
  return `/docs/${slug}/`;
}

for(const [slug,film] of Object.entries(FILMS)) for(const locale of ['', '/zh']) {
  test(`${locale || "en"} ${slug}: responsive player and playable delivery`,async({page},testInfo)=>{
    await page.goto(locale+filmRoute(slug));
    const video=await assertPlayerAttributes(page,film);
    for(const width of WIDTHS) {
      await assertGeometry(page,video,width);
      if(!PROBED_WIDTHS.includes(width)) continue;
      await page.screenshot({path:testInfo.outputPath(`player-${width}.png`)});
      await playAndAssertProgress(video);
      await pauseAndHideTracks(video);
      await seekAndAssert(video);
    }
    await assertTranscript(page,film);
    await assertDownloadLink(video,slug);
  });
}

// [DOC-FILMS-DELIVERY] Language launch coverage belongs in the blog.
test('retired Osprey docs open the localized launch film', async ({page}) => {
  for (const locale of ['', '/zh']) {
    await page.goto(`${locale}/docs/osprey/`);
    await expect(page).toHaveURL(`${locale}/blog/osprey-first-class/`);
    await expect(page.locator('.doc-film video')).toHaveCount(1);
    await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href',
      `https://deslop.live${locale}/blog/osprey-first-class/`);
    await page.goto(`${locale}/docs/`);
    await expect(page.locator('.docs-sidebar a[href$="/docs/osprey/"]')).toHaveCount(0);
  }
});
