// [DOC-FILMS-VALIDATION] Preserve a completed browser run as reproducible evidence.
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
const root=path.resolve(import.meta.dirname,'../..');
const log=fs.readFileSync(process.argv[2],'utf8');
const announced=Number(log.match(/Running (\d+) tests using/)?.[1]);
const summary=log.match(/(\d+) passed \(([^)]+)\)/);
const argumentsUsed=log.match(/^> playwright test (.+)$/m)?.[1];
const passed=log.split('\n').filter(line=>line.trim().startsWith('✓'));
if(!argumentsUsed || !summary || Number(summary[1])!==announced || passed.length!==announced || /\d+ failed/.test(log)) throw new Error('Browser run is incomplete or failing');
const catalog=JSON.parse(fs.readFileSync(path.join(root,'site/src/_data/docFilms.json'),'utf8'));
for(const slug of Object.keys(catalog)) for(const locale of ['en','/zh']) {
  if(!passed.some(line=>line.includes(`› ${locale} ${slug}: responsive player and playable delivery`))) throw new Error(`Missing ${locale} ${slug}`);
}
const commit=execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim();
const report=['# Website and film browser validation','',`- Source commit: ${commit}`,`- Result: ${summary[1]} passed in ${summary[2]}`,`- Documentation routes: ${Object.keys(catalog).length*2}`,'- Playback and seeking checked at phone and desktop widths.',`- Command: \`DESLOP_SITE_TEST_PORT=8097 npm test -- ${argumentsUsed}\``,'- Server: installed `@11ty/eleventy-dev-server`, serving an immutable completed website build on port 8097.','','Regenerate after a successful run with `node scripts/videos/record-site-check.mjs <captured-test-log>`. This receipt is emitted from the completed Playwright output; it does not claim subjective audio review.','','## Passed checks','','```text',...passed.map(line=>line.trim()),'```',''].join('\n');
fs.writeFileSync(path.join(root,'reports/videos/site-validation.md'),report);
process.stdout.write(`Recorded ${summary[1]} browser checks\n`);
