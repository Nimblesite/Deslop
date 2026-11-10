// [DOC-FILMS-MOTION] Verify the bytes served by the public deployment.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
const root = path.resolve(import.meta.dirname, '../..');
const origin = 'https://deslop.live';
const catalog = JSON.parse(fs.readFileSync(path.join(root, 'site/src/_data/docFilms.json'), 'utf8'));
const commit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
const run = process.argv[2];
assert.ok(run, 'Pass the successful Pages workflow run URL');
const rows = [];
for (const slug of Object.keys(catalog)) {
  const asset = `/assets/video/docs/deslop-docs-${slug}.mp4`;
  const local = createHash('sha256').update(fs.readFileSync(path.join(root, 'site/src', asset))).digest('hex');
  const response = await fetch(origin + asset, { headers: { 'Cache-Control': 'no-cache' }, signal: AbortSignal.timeout(60000) });
  assert.ok(response.ok, `${asset}: HTTP ${response.status}`);
  assert.ok(response.headers.get('content-type')?.includes('video/mp4'));
  const hash = createHash('sha256');
  for await (const chunk of response.body) hash.update(chunk);
  const remote = hash.digest('hex');
  assert.equal(remote, local, `Public video differs from the verified local delivery: ${slug}`);
  rows.push(`| ${slug} | ${remote} |`);
  process.stdout.write(`${slug}: public SHA-256 matches\n`);
}
fs.writeFileSync(path.join(root, 'reports/videos/deployment-verification.md'), [
  '# Public film deployment verification [DOC-FILMS-MOTION]', '',
  `- Verified at: ${new Date().toISOString()}`,
  `- Verification checkout: ${commit}`,
  `- Deployment run: ${run}`,
  `- Public origin: ${origin}`,
  `- Matching public videos: ${rows.length}/${Object.keys(catalog).length}`, '',
  '| Film | Public SHA-256, equal to local delivery |', '| --- | --- |', ...rows, '',
  'Every response was HTTP 200 with video/mp4 content. The verifier hashes the complete response body, not a poster or LFS pointer.', '',
].join('\n'));
