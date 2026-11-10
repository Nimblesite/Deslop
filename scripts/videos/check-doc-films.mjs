// [DOC-FILMS-COVERAGE] Mechanically account for every film destination.
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
const root=path.resolve(import.meta.dirname,'../..');
const pages=path.join(root,'site/src/docs');
const catalog=JSON.parse(fs.readFileSync(path.join(root,'site/src/_data/docFilms.json'),'utf8'));
const suffixes=['.mp4','-poster.jpg','.en.vtt','.chapters.vtt'];
const docFilms=fs.readdirSync(pages).filter(name=>name.endsWith('.md')).sort().map(name=>({
  name, slug:name==='index.md'?'getting-started':name.slice(0,-3),
}));
const destinations=[...docFilms,{name:'blog/osprey-first-class.md',slug:'osprey'}];
const rows=destinations.map(({name,slug})=>{
  const prefix=`site/src/assets/video/docs/deslop-docs-${slug}`;
  const present=suffixes.every(suffix=>fs.existsSync(path.join(root,prefix+suffix)));
  const media=prefix+'.mp4';
  const attributes=present?execFileSync('git',['check-attr','filter','--',media],{cwd:root,encoding:'utf8'}):'';
  const lfs=attributes.trim().endsWith(': lfs');
  const receipt=fs.existsSync(path.join(root,'reports/videos',`${slug}-media.md`));
  return {page:name,slug,delivered:Boolean(catalog[slug]&&present&&lfs&&receipt),duration:catalog[slug]?.duration||'Pending',lfs};
});
const delivered=rows.filter(row=>row.delivered).length;
const report=['# Documentation film delivery status','',`${delivered} of ${rows.length} film destinations have imported media, captions, chapters, poster, LFS rules and a verification receipt.`,'','| Page | Delivery | Duration | LFS rule |','| --- | --- | --- | --- |',...rows.map(row=>`| ${row.page} | ${row.delivered?'Imported':'Incomplete'} | ${row.duration} | ${row.lfs?'Yes':'Pending'} |`),'','This inventory does not replace browser playback, source review or finished-frame inspection.',''].join('\n');
fs.writeFileSync(path.join(root,'reports/videos/series-status.md'),report);
process.stdout.write(`${delivered}/${rows.length} imported; reports/videos/series-status.md\n`);
if(process.argv.includes('--complete')&&delivered!==rows.length) process.exitCode=1;
