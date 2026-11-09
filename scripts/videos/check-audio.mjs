// [DOC-FILMS-DELIVERY] Measure delivered narration mixes without subjective claims.
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
const root=path.resolve(import.meta.dirname,'../..');
const catalog=JSON.parse(fs.readFileSync(path.join(root,'site/src/_data/docFilms.json'),'utf8'));
const rows=[];
for(const slug of Object.keys(catalog)) {
  const file=path.join(root,'site/src/assets/video/docs',`deslop-docs-${slug}.mp4`);
  const result=spawnSync('ffmpeg',['-hide_banner','-nostats','-i',file,'-vn','-af','ebur128=peak=true','-f','null','-'],{encoding:'utf8'});
  if(result.status!==0) throw new Error(`Audio measurement failed for ${slug}: ${result.stderr}`);
  const summary=result.stderr.slice(result.stderr.lastIndexOf('Summary:'));
  const read=(pattern)=>{const match=summary.match(pattern);if(!match)throw new Error(`Missing audio measurement for ${slug}`);return Number(match[1]);};
  const loudness=read(/I:\s+(-?[\d.]+) LUFS/);
  const range=read(/LRA:\s+([\d.]+) LU/);
  const peak=read(/Peak:\s+(-?[\d.]+) dBFS/);
  rows.push({slug,loudness,range,peak});
}
const report=['# Delivered film audio measurements','','Measured from each delivered MP4 with FFmpeg EBU R128. These measurements describe level and headroom, not pronunciation or subjective voice quality.','','| Film | Integrated loudness (LUFS) | Range (LU) | True peak (dBFS) |','| --- | --- | --- | --- |',...rows.map(r=>`| ${r.slug} | ${r.loudness} | ${r.range} | ${r.peak} |`),''].join('\n');
fs.writeFileSync(path.join(root,'reports/videos/audio-levels.md'),report);
process.stdout.write(`Measured ${rows.length} delivered mixes; reports/videos/audio-levels.md\n`);
