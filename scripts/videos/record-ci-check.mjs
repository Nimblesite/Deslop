// [DOC-FILMS-VALIDATION] Record the latest captured full CI result without claiming a green gate.
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
const root=path.resolve(import.meta.dirname,'../..');
const source=process.argv[2];
const log=fs.readFileSync(source,'utf8');
const summary=log.match(/test result: FAILED\. ([^\n]+)/)?.[0];
const failed=[...log.matchAll(/^test (.+) \.\.\. FAILED$/gm)].map(match=>match[1]);
if(!summary||!failed.length) throw new Error('Expected a complete failing test summary');
const details=log.slice(log.indexOf('\nfailures:\n')).trim();
const report=['# Full CI gate [DOC-FILMS-VALIDATION]','','- Command: `make ci`',`- Captured log modified: ${fs.statSync(source).mtime.toISOString()}`,`- Log SHA-256: ${createHash('sha256').update(log).digest('hex')}`,`- Result: ${summary}`,`- Failing tests: ${failed.length}`,'','This is the captured full CI run, not a green gate for the film changes. Website build and browser validation are recorded separately. The PR submission skill requires the full gate to pass.','','## Failure output','','```text',details,'```',''].join('\n');
fs.writeFileSync(path.join(root,'reports/videos/ci-status.md'),report);
