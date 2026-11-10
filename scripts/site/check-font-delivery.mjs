// [SITE-NAV-FONTS] Compare delivered font bytes with the original bundled assets.
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
const root=path.resolve(import.meta.dirname,'../..');
const baseline='5a8ade06';
const prefix='site/src/assets/fonts/';
const fonts=[['Material Symbols','material-symbols-outlined.woff2','material-symbols-outlined.woff2'],['Inter','inter-variable.ttf','inter-variable.woff2'],['JetBrains Mono','jetbrains-mono-variable.ttf','jetbrains-mono-variable.woff2']];
const rows=fonts.map(([name,before,after])=>{
  const original=Number(execFileSync('git',['cat-file','-s',`${baseline}:${prefix}${before}`],{cwd:root,encoding:'utf8'}));
  const delivered=fs.readFileSync(path.join(root,prefix+after));
  const pointer=execFileSync('git',['show',`HEAD:${prefix}${after}`],{cwd:root,encoding:'utf8'});
  if(!pointer.startsWith('version https://git-lfs.github.com/spec/v1')) throw new Error(`${name} is not committed as LFS`);
  return {name,before:original,after:delivered.length,saved:original-delivered.length};
});
const totals=rows.reduce((sum,row)=>({before:sum.before+row.before,after:sum.after+row.after,saved:sum.saved+row.saved}),{before:0,after:0,saved:0});
const report=['# Website font delivery','',`Baseline: bundled assets in commit ${baseline}. Current sizes are read from the delivered files; every current font is committed as a Git LFS pointer.`,'','| Font | Before (bytes) | Delivered (bytes) | Saved (bytes) |','| --- | ---: | ---: | ---: |',...rows.map(row=>`| ${row.name} | ${row.before} | ${row.after} | ${row.saved} |`),`| Total | ${totals.before} | ${totals.after} | ${totals.saved} |`,'','Regenerate with `node scripts/site/check-font-delivery.mjs`. Text-font glyph coverage is retained; the symbols font is subset by `scripts/site/subset-icons.py`. Browser checks verify local loading and every published icon ligature.',''].join('\n');
fs.mkdirSync(path.join(root,'reports/website'),{recursive:true});
fs.writeFileSync(path.join(root,'reports/website/font-delivery.md'),report);
process.stdout.write(`${totals.before} -> ${totals.after} font bytes; ${totals.saved} bytes saved\n`);
