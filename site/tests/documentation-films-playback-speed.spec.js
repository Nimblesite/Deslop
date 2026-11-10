import {expect,test} from '@playwright/test';
import {FILMS} from './helpers/doc-films.js';
import {keyframeTimes} from './helpers/mp4-keyframes.js';
// [DOC-FILMS-DELIVERY] Safari restarts decoding at the previous keyframe on every playback-speed
// change, so a film whose keyframes sit far apart freezes when the speed menu is used.
const MAX_KEYFRAME_GAP_SECONDS=1;
const FLOAT_SLACK_SECONDS=0.001;
const MIN_FILM_SECONDS=30;
const filmPath=slug=>new URL(`../src/assets/video/docs/deslop-docs-${slug}.mp4`,import.meta.url);

for(const slug of Object.keys(FILMS)) {
  test(`${slug}: a keyframe at least every second, from the first frame to the last`,()=>{
    const {times,duration}=keyframeTimes(filmPath(slug));
    expect(duration,'the delivery is a whole film').toBeGreaterThan(MIN_FILM_SECONDS);
    expect(times[0],'playback can start without a seek').toBe(0);
    expect(times.length,'one keyframe per second of film').toBeGreaterThanOrEqual(Math.floor(duration/MAX_KEYFRAME_GAP_SECONDS));
    const gaps=[...times.slice(1),duration].map((time,index)=>time-times[index]);
    const widest=Math.max(...gaps);
    expect(widest,`widest keyframe gap in ${slug}`).toBeLessThanOrEqual(MAX_KEYFRAME_GAP_SECONDS+FLOAT_SLACK_SECONDS);
    expect(gaps.every(gap=>gap>0),'keyframes advance through the film').toBe(true);
  });
}
