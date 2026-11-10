// [DOC-FILMS-MOTION] Inspect delivered application frames, not caption text.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
const root=path.resolve(import.meta.dirname,'../..');
const videos=path.resolve(root,'../Videos/projects/deslop/videos');
const catalog=JSON.parse(fs.readFileSync(path.join(root,'site/src/_data/docFilms.json'),'utf8'));
const expected={
  'action-create':['checkout'], 'action-version':['v0.41.0'], 'action-limit':['exit status: 3'], 'action-changes':['changes.patch','only-changed','exit status: 3'],
  'start-extension':['Deslop'], 'review-finding':['Eta.cs'], 'review-sources':['Zeta.cs'],
  'review-compare':['Tally','step'], 'review-pair':['Tally','step'], 'review-counts':['Duplication'],
  'review-refresh':['SharedCounter'], 'review-edit':['limit'], 'review-edited-compare':['limit'],
  'config-create':['threshold'], 'config-exclude':['archive','reference'], 'config-override':['exit status: 0'], 'config-limit':['exit status: 3'],
  'osprey-finding':['beta.osp'], 'osprey-compare':['alpha.osp','beta.osp'], 'osprey-python':['osprey'], 'osprey-config':['osprey-python-project','exit status: 0'],
  'agent-duplicates':['deslop/duplicates','Eta.cs','Zeta.cs'], 'agent-details':['Eta.cs','Zeta.cs'],
  'agent-ask':['find-similar','Eta.cs'], 'agent-search':['find-similar','Eta.cs'], 'agent-results':['Eta.cs','Zeta.cs'],
};
const run=(program,args,input)=>execFileSync(program,args,{input, maxBuffer:16*1024*1024,stdio:['pipe','pipe','ignore']});
const normalize=text=>text.toLowerCase().replace(/\bv[dqo@](?=\.\d+\.)/g,'v0').replace(/exit\s*status:\s*(?:@|q|o|\(\))/g,'exit status: 0').replaceAll(/[^a-z0-9]/g,'');
function inspect(media,scene,cache) {
  const fractions=scene.shot==='action-changes' ? [0.04,0.2,0.55,0.9] : [0.2,0.55,0.9];
  const terminal=['action-limit','action-changes','config-override','config-limit','osprey-config'].includes(scene.shot);
  const crop=terminal ? 'crop=iw:ih*0.91:0:0' : 'crop=iw:ih*0.82:0:0';
  const text=cache.data[scene.shot] ?? fractions.map(fraction=>{
    const time=scene.start+(scene.end-scene.start)*fraction;
    const frame=run('ffmpeg',['-nostdin','-v','error','-ss',String(time),'-i',media,'-vf',crop,'-frames:v','1','-f','image2pipe','-vcodec','png','-']);
    return (terminal ? ['6','11'] : ['11']).map(mode=>run('tesseract',['stdin','stdout','--psm',mode],frame).toString()).join('\n');
  }).join('\n');
  cache.data[scene.shot]=text;
  fs.writeFileSync(cache.file,JSON.stringify(cache,null,2)+'\n');
  for(const word of expected[scene.shot]) assert.ok(normalize(text).includes(normalize(word)),`${scene.shot}: delivered app frames did not show ${word}`);
  return expected[scene.shot].join(', ');
}
const events=fs.readFileSync(path.resolve(videos,'../captures/docs-tutorial/pi-session.jsonl'),'utf8').trim().split('\n').map(line=>JSON.parse(line));
const calls=events.filter(row=>row.message?.role==='toolResult' && row.message.toolName.startsWith('mcp__deslop__'));
assert.equal(calls.length,4);
for(const call of calls) {
  assert.ok(!call.message.isError);
  const result=JSON.parse(call.message.content[0].text);
  const finding=result.clusters?.[0] ?? result;
  assert.equal(finding.occurrence_count,2);
  assert.deepEqual(finding.occurrences.map(item=>item.path).sort(),['Eta.cs','Zeta.cs']);
  assert.ok(finding.occurrences.every(item=>item.start_line===1 && item.end_line===19));
}
const rows=[];
const requested=process.argv.slice(2);
for(const slug of requested.length ? requested : Object.keys(catalog)) {
  const base=`deslop-docs-${slug}`;
  const media=path.join(root,'site/src/assets/video/docs',base+'.mp4');
  const timeline=JSON.parse(fs.readFileSync(path.join(videos,'docs-'+slug,'dist',base+'.timeline.json'),'utf8'));
  const file=path.join(videos,'docs-'+slug,'dist',base+'.ocr.json');
  const sha256=createHash('sha256').update(fs.readFileSync(media)).digest('hex');
  const saved=fs.existsSync(file) ? JSON.parse(fs.readFileSync(file,'utf8')) : {};
  const cache={file,sha256,data:saved.sha256===sha256 ? saved.data : {}};
  for(const scene of timeline.scenes) rows.push(`| ${slug} | ${scene.shot} | ${inspect(media,scene,cache)} |`);
  process.stdout.write(`Verified finished application frames: ${slug}\n`);
}
const speech=JSON.parse(fs.readFileSync(path.join(videos,'doc-series/narration/check.json'),'utf8'));
assert.equal(speech.length,40);
assert.ok(speech.every(row=>row.word_match>=0.85));
const report=['# Finished workflow verification [DOC-FILMS-MOTION]','','All checks below read the encoded website MP4s. OCR excludes captions and samples at least three frames from each step. Editor checks also exclude the step-label band; terminal checks retain the result lines and require explicit exit-status values absent from the labels. These content checks supplement visual review; they do not measure editing quality.','','| Film | Recorded step | Visible application content |','| --- | --- | --- |',...rows,'','## Real agent evidence','','The saved pi session contains four successful Deslop MCP results: duplicates, finding details and two similarity requests. Every result identifies Eta.cs and Zeta.cs at lines 1–19, with exactly two copies.','','## Narration','','Independent local Whisper transcription checked all 40 Qwen3-TTS Aiden speech files against the current scripts.','','| Film | Step | Word-sequence match |','| --- | ---: | ---: |',...speech.map(row=>`| ${row.film} | ${row.step} | ${(row.word_match*100).toFixed(1)}% |`),'','Regenerate with `node scripts/videos/verify-user-workflows.mjs`.',''].join('\n');
if (!requested.length) fs.writeFileSync(path.join(root,'reports/videos/workflow-verification.md'),report);
