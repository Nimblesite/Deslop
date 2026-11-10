import {expect,test} from '@playwright/test';
import {FILMS,filmRoute} from './helpers/doc-films.js';
// The native Safari speed menu, visited twice so every change happens mid-playback.
const RATES=[0.5,1.25,1,1.5,2,0.5,1];
const RESPONSE_MS=1000;
const MEDIA_SECONDS_PER_CHANGE=0.25;
// The player preloads nothing, so the first frames arrive over the network before any change is timed.
const START_MS=15000;
// [DOC-FILMS-DELIVERY] Each speed change resumes promptly in WebKit, at the chosen rate.

async function assertResumesAt(video,rate) {
  const before=await video.evaluate((v,next)=>{v.playbackRate=next;return v.currentTime;},rate);
  await expect.poll(()=>video.evaluate(v=>v.currentTime),{timeout:RESPONSE_MS}).toBeGreaterThan(before+MEDIA_SECONDS_PER_CHANGE*rate);
  expect(await video.evaluate(v=>[v.playbackRate,v.paused,v.error])).toEqual([rate,false,null]);
}

for(const slug of Object.keys(FILMS)) {
  test(`${slug}: playback speed changes take effect immediately`,async({page})=>{
    await page.goto(filmRoute(slug));
    const video=page.locator('.doc-film video');
    await video.evaluate(v=>{v.muted=true;return v.play();});
    await expect.poll(()=>video.evaluate(v=>v.currentTime),{timeout:START_MS}).toBeGreaterThan(MEDIA_SECONDS_PER_CHANGE);
    for(const rate of RATES) await assertResumesAt(video,rate);
  });
}
