// [DOC-FILMS-MOTION] Inspect delivered frames, not just filenames or recipes.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
const root = path.resolve(import.meta.dirname, '../..');
const work = path.join(root, 'target/docs-live');
const delivery = path.join(work, 'delivery');
const run = (command, args, options = {}) => execFileSync(command, args, { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024, ...options });
const edits = fs.readdirSync(delivery).filter(file => file.endsWith('.edit.json'))
  .map(file => JSON.parse(fs.readFileSync(path.join(delivery, file), 'utf8')));
assert.equal(edits.length, 10, 'Every delivered film needs its actual edit timeline');
const motionHashes = new Map();

function verifyMotion(file) {
  if (motionHashes.has(file)) return motionHashes.get(file);
  const output = run('ffmpeg', ['-v', 'error', '-ss', '1', '-i', file, '-vf',
    'crop=1600:600:160:240,fps=1,scale=160:60', '-frames:v', '6', '-f', 'framemd5', '-']);
  const hashes = output.split('\n').filter(line => line && !line.startsWith('#'))
    .map(line => line.split(',').at(-1).trim());
  const unique = new Set(hashes).size;
  assert.ok(unique >= 4, `Explanatory graphic is static: ${file}`);
  motionHashes.set(file, unique);
  return unique;
}

function verifyAgent(edit) {
  const scene = edit.edits.find(item => item.footage === 'pi-find-similar');
  if (!scene) return null;
  const at = scene.footageStart + scene.footageSeconds - 1;
  const frame = execFileSync('ffmpeg', ['-v', 'error', '-ss', String(at), '-i',
    path.join(delivery, `deslop-docs-${edit.slug}.mp4`), '-frames:v', '1', '-f', 'image2pipe', '-c:v', 'png', '-']);
  const visible = run('tesseract', ['stdin', 'stdout', '--psm', '11'], { input: frame, stdio: ['pipe', 'pipe', 'ignore'] });
  for (const marker of ['deslop/find-similar', 'Eta.cs', 'Zeta.cs']) {
    assert.ok(visible.includes(marker), `${marker} is missing from the encoded ${edit.slug} frame at ${at}`);
  }
  const camera = fs.readFileSync(path.join(work, 'pi-find-similar.cmd'), 'utf8');
  const widths = new Set(camera.split('\n').filter(Boolean).map(line => line.split(' w ')[1].split(',')[0]));
  assert.ok(widths.size > 1, 'The agent camera never zooms');
  return { at, widths: widths.size };
}

const rows = edits.map(edit => {
  assert.equal(edit.edits.length, 4);
  for (const scene of edit.edits) {
    assert.ok(scene.animatedSeconds > 0);
    verifyMotion(scene.motion);
  }
  const agent = verifyAgent(edit);
  return { edit, agent };
});
assert.equal(rows.filter(row => row.agent).length, 2, 'Both AI films must visibly show pi MCP findings');
const report = ['# Finished motion and pi verification [DOC-FILMS-MOTION]', '',
  `- Delivered films inspected: ${rows.length}`,
  `- Animated chapter graphics checked: ${motionHashes.size}`,
  '- Graphic checks sample six encoded frames inside the illustration, excluding headings and the progress bar.',
  '- Every illustration has at least four distinct sampled frames.',
  '- Encoded agent frames must visibly contain `deslop/find-similar`, `Eta.cs` and `Zeta.cs`.', '',
  '| Film | Chapters with motion | Agent result frame (s) | Distinct camera widths |',
  '| --- | ---: | ---: | ---: |',
  ...rows.map(({ edit, agent }) => `| ${edit.slug} | ${edit.edits.length} | ${agent ? agent.at.toFixed(3) : '-'} | ${agent?.widths ?? '-'} |`), '',
  'Regenerate with `node scripts/videos/verify-motion-films.mjs` after rendering and cutting through Videos. Audio stream hashes and media hashes are verified by `cut-doc-footage.mjs --install`.', '',
].join('\n');
fs.writeFileSync(path.join(root, 'reports/videos/motion-verification.md'), report);
process.stdout.write(`${rows.length} films; ${motionHashes.size} moving illustrations; both encoded pi results visible\n`);
