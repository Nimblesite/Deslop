// [DOC-FILMS-LIVE] Account for the recorded source behind the full delivery timeline.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
const root=path.resolve(import.meta.dirname,'../..');
const source=path.resolve(root,'../Videos/projects/deslop');
const catalog=JSON.parse(fs.readFileSync(path.join(root,'site/src/_data/docFilms.json'),'utf8'));
const shots=new Set();
const rows=Object.keys(catalog).map(slug=>{
  const base=`deslop-docs-${slug}`;
  const film=JSON.parse(fs.readFileSync(path.join(source,'videos','docs-'+slug,'dist',base+'.timeline.json'),'utf8'));
  assert.equal(film.scenes.length,4);
  let covered=0;
  for(const scene of film.scenes) {
    assert.ok(Math.abs(scene.start-covered)<0.01);
    assert.ok(fs.existsSync(path.join(source,'captures/docs-tutorial',scene.shot+'.mp4')));
    covered=scene.end; shots.add(scene.shot);
  }
  assert.ok(Math.abs(covered-film.duration)<0.01);
  const seconds=Number(execFileSync('ffprobe',['-v','error','-show_entries','format=duration','-of','csv=p=0',path.join(root,'site/src/assets/video/docs',base+'.mp4')],{encoding:'utf8'}));
  assert.ok(Math.abs(seconds-covered)<0.18);
  return `| ${slug} | ${covered.toFixed(2)} | ${film.scenes.map(scene=>scene.shot).join(', ')} |`;
});
const report=['# Documentation film footage [DOC-FILMS-LIVE]','',`${rows.length} films use ${shots.size} distinct recorded application takes across their complete timelines. Only captions, step labels and playback-speed labels overlay those recordings. Finished-frame content verification is in workflow-verification.md.`,'','| Film | Recorded application timeline (seconds) | Recorded takes |','| --- | ---: | --- |',...rows,'','The timeline accounts for the complete encoded duration; this inventory supplements finished-frame review and playback checks. Narration is Qwen3-TTS Aiden.','','Regenerate with `node scripts/videos/record-footage-inventory.mjs`.',''].join('\n');
fs.writeFileSync(path.join(root,'reports/videos/live-footage.md'),report);
