import {expect,test} from '@playwright/test';
import fs from 'node:fs';
const FILMS=JSON.parse(fs.readFileSync(new URL('../src/_data/docFilms.json',import.meta.url),'utf8'));
const WIDTHS=[320,390,768,1440];
const HEIGHT=900;
const SEEK_SECONDS=20;
const EXPECTED_CHAPTERS=4;
// [DOC-FILMS-DELIVERY] Actual delivered media, tracks and responsive controls.
for(const [slug,film] of Object.entries(FILMS)) for(const locale of ['', '/zh']) {
  test(`${locale || "en"} ${slug}: responsive player and playable delivery`,async({page},testInfo)=>{
    await page.goto(locale+(slug==='getting-started'?'/docs/':`/docs/${slug}/`));
    const video=page.locator('.doc-film video');
    await expect(video).toHaveCount(1);
    await expect(video).toHaveAttribute('aria-label',film.title);
    await expect(video).toHaveAttribute('controls','');
    await expect(video).toHaveAttribute('preload','none');
    for(const width of WIDTHS) {
      await page.setViewportSize({width,height:HEIGHT});
      await video.scrollIntoViewIfNeeded();
      const box=await video.boundingBox();
      expect(box.width).toBeGreaterThan(200);
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x+box.width).toBeLessThanOrEqual(width+1);
      if(width===390 || width===1440) await page.screenshot({path:testInfo.outputPath(`player-${width}.png`)});
    }
    expect(await video.evaluate(v=>v.paused)).toBe(true);
    await video.evaluate(v=>{v.muted=true;return v.play();});
    await expect.poll(()=>video.evaluate(v=>v.currentTime)).toBeGreaterThan(0);
    await video.evaluate(v=>{v.pause();for(const track of v.textTracks)track.mode='hidden';});
    await expect.poll(()=>video.evaluate(v=>v.textTracks[0].cues?.length)).toBe(EXPECTED_CHAPTERS);
    await expect.poll(()=>video.evaluate(v=>v.textTracks[1].cues?.length)).toBe(EXPECTED_CHAPTERS);
    await video.evaluate((v,time)=>{v.currentTime=time;},SEEK_SECONDS);
    await expect.poll(()=>video.evaluate(v=>v.seeking)).toBe(false);
    expect(await video.evaluate(v=>v.currentTime)).toBeCloseTo(SEEK_SECONDS);
    expect(await video.evaluate(v=>v.error)).toBeNull();
    await expect(video.locator('a')).toHaveAttribute('href',`/assets/video/docs/deslop-docs-${slug}.mp4`);
  });
}
