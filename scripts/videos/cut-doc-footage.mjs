// [DOC-FILMS-LIVE] Recut the visual track without moving narration or captions.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { readTrack, WIDGETS } from '../../../SharpLsp/src/website/tools/video/kit/track.mjs';
import { CAMERA, writeCamera } from '../../../SharpLsp/src/website/tools/video/kit/zoom.mjs';

const root = path.resolve(import.meta.dirname, '../..');
const work = path.join(root, 'target/docs-live');
const clips = path.join(work, 'clips');
const site = path.join(root, 'site/src/assets/video/docs');
const series = path.join(root, '../Videos/projects/deslop/videos');
const delivery = path.join(work, 'delivery');
const fps = 30;
const size = '1920:1080';
const encode = ['-c:v', 'libx264', '-threads', '4', '-preset', 'veryfast', '-crf', '18', '-pix_fmt', 'yuv420p'];
const films = {
  'vscode-cluster-panel': ['occurrence', 'canonical', 'pair', 'numbers'],
  'getting-started': [null, 'find', 'occurrence', 'canonical'],
  'how-it-works': ['find', 'occurrence', 'canonical', 'numbers'],
  'accuracy-transparency': ['numbers', 'numbers', null, null],
  'research-background': [null, 'find', null, 'canonical'],
  configuration: ['policy', 'policy', null, 'gate'],
  'github-action': ['action', 'action', 'gate', null],
  osprey: ['osprey-native', 'osprey-python', 'osprey-manifest', 'osprey-scan'],
  'ai-integration': ['mcp-status', 'mcp-config', null, 'canonical'],
  'for-ai': ['mcp-status', 'mcp-query', 'mcp-query', 'canonical'],
};

const ffmpeg = (args) => execFileSync('ffmpeg', ['-nostdin', '-y', '-v', 'error', ...args], { stdio: 'inherit' });
const probe = (file, fields) => JSON.parse(execFileSync('ffprobe',
  ['-v', 'error', '-select_streams', 'v:0', '-show_entries', fields, '-of', 'json', file], { encoding: 'utf8' }));
const seconds = (file) => Number(probe(file, 'format=duration').format.duration);
const stamp = (value) => value.split(':').reduce((total, part) => total * 60 + Number(part), 0);
const starts = (file) => fs.readFileSync(file, 'utf8').split('\n').filter((line) => line.includes(' --> '))
  .map((line) => stamp(line.split(' --> ')[0]));
const cache = new Map();

function zoom(name) {
  if (cache.has(name)) return cache.get(name);
  const raw = path.join(clips, name + '.mp4');
  const track = readTrack(path.join(clips, name + '.track.json'));
  const cues = JSON.parse(fs.readFileSync(path.join(clips, name + '.cues.json'), 'utf8'));
  assert.ok(cues.length, `No real clicks in ${name}`);
  const [{ width, height }] = probe(raw, 'stream=width,height').streams;
  const file = path.join(work, name + '.cmd');
  const terminal = ['gate', 'osprey-scan', 'mcp-status', 'mcp-query'].includes(name);
  const panel = WIDGETS.findIndex(([selector]) => selector === '.part.panel');
  if (terminal) track.samples = track.samples.map((sample) => ({ ...sample, w: sample.w.filter(([index]) => index !== panel) }));
  const deepest = CAMERA.deepest;
  if (terminal) CAMERA.deepest = 1.35;
  const moves = writeCamera(track, { file, name: 'live', fps, frames: Math.round(seconds(raw) * fps),
    scale: width / track.size[0], speed: 1 });
  CAMERA.deepest = deepest;
  assert.ok(moves.length > 1, `No automatic camera move in ${name}`);
  const output = path.join(work, name + '-zoom.mp4');
  ffmpeg(['-filter_threads', '2', '-i', raw, '-vf',
    `format=yuv444p,sendcmd=f='${file}',crop@live=w=${width}:h=${height}:x=0:y=0:exact=1,scale=${size}:flags=lanczos,setsar=1`,
    ...encode, output]);
  const shot = { output, duration: seconds(output), clicks: cues.length, moves: moves.length - 1 };
  cache.set(name, shot);
  return shot;
}

function segment(source, start, duration, target) {
  ffmpeg(['-ss', String(start), '-i', source, '-t', duration.toFixed(4), '-an',
    '-vf', `fps=${fps},scale=${size},setsar=1`, ...encode, target]);
  return target;
}

function cut(slug, names) {
  const base = 'deslop-docs-' + slug;
  const source = original(slug);
  const duration = seconds(source);
  const boundaries = [...starts(path.join(site, base + '.chapters.vtt')), duration];
  assert.equal(boundaries.length, names.length + 1);
  const segments = [];
  const insert = (file, offset, length) => {
    if (length >= 1 / fps) segments.push(segment(file, offset, length, path.join(work, `${slug}-${segments.length}.mp4`)));
  };
  let firstLive;
  let live = 0;
  for (const [index, name] of names.entries()) {
    const start = boundaries[index];
    const length = boundaries[index + 1] - start;
    const shot = name ? zoom(name) : null;
    const shown = shot ? Math.min(length, shot.duration) : 0;
    if (shot) {
      firstLive ??= start;
      insert(shot.output, 0, shown);
      live += shown;
    }
    insert(source, start + shown, length - shown);
  }
  assert.ok(live > 0, `No live footage for ${slug}`);
  const manifest = path.join(work, slug + '.ffconcat');
  fs.writeFileSync(manifest, segments.map((file) => `file '${file}'`).join('\n') + '\n');
  const output = path.join(delivery, base + '.mp4');
  ffmpeg(['-f', 'concat', '-safe', '0', '-i', manifest, '-i', source, '-map', '0:v:0', '-map', '1:a:0',
    '-c', 'copy', '-t', String(duration), '-movflags', '+faststart', output]);
  const poster = path.join(delivery, base + '-poster.jpg');
  ffmpeg(['-ss', String(firstLive + 3), '-i', output, '-frames:v', '1', '-q:v', '3', poster]);
  assert.ok(Math.abs(seconds(output) - duration) < 0.18, `Audio or caption timing drift in ${slug}`);
  fs.rmSync(manifest);
  for (const file of segments) fs.rmSync(file);
  return [slug, live.toFixed(1), duration.toFixed(1)];
}

function original(slug) {
  return path.join(series, 'docs-' + slug, 'dist', 'deslop-docs-' + slug + '.mp4');
}

function audioHash(file) {
  return execFileSync('ffmpeg', ['-v', 'error', '-i', file, '-map', '0:a:0',
    '-c', 'copy', '-f', 'hash', '-'], { encoding: 'utf8' }).trim();
}

function verifiedDelivery(slug) {
  const base = 'deslop-docs-' + slug;
  const video = path.join(delivery, base + '.mp4');
  const poster = path.join(delivery, base + '-poster.jpg');
  const duration = seconds(video);
  assert.ok(fs.statSync(poster).size > 0);
  assert.equal(audioHash(video), audioHash(original(slug)), `Audio changed in ${slug}`);
  const [{ width, height, codec_name: codec }] = probe(video, 'stream=width,height,codec_name').streams;
  assert.deepEqual([width, height, codec], [1920, 1080, 'h264']);
  const chapters = starts(path.join(site, base + '.chapters.vtt'));
  const live = films[slug].reduce((total, name, index) => name
    ? total + Math.min((chapters[index + 1] ?? duration) - chapters[index],
      seconds(path.join(work, name + '-zoom.mp4'))) : total, 0);
  const hash = createHash('sha256').update(fs.readFileSync(video)).digest('hex');
  return { base, slug, video, poster, duration, live, hash };
}

function install(items) {
  const reports = path.join(root, 'reports/videos');
  for (const { base, slug, video, poster, duration, live, hash } of items) {
    fs.copyFileSync(video, path.join(site, base + '.mp4'));
    fs.copyFileSync(poster, path.join(site, base + '-poster.jpg'));
    fs.writeFileSync(path.join(reports, slug + '-media.md'), [
      '# ' + slug + ' media verification', '',
      `- Duration: ${duration.toFixed(3)} seconds`,
      `- Live VS Code footage: ${live.toFixed(1)} seconds`,
      '- Video: H.264, 1920 × 1080',
      '- Audio: original AAC stream, verified by stream hash',
      '- English captions and chapters: original timestamps preserved',
      `- SHA-256: ${hash}`, '',
    ].join('\n'));
  }
  fs.writeFileSync(path.join(reports, 'live-footage.md'), [
    '# Documentation film footage [DOC-FILMS-LIVE]', '',
    '| Film | Live VS Code (s) | Total (s) |', '| --- | ---: | ---: |',
    ...items.map(({ slug, live, duration }) => `| ${slug} | ${live.toFixed(1)} | ${duration.toFixed(1)} |`),
    '', 'The recut keeps the original audio, captions and chapter timings. See each media receipt for its SHA-256.', '',
  ].join('\n'));
}

const selection = process.argv.slice(2);
if (selection[0] === '--install') {
  install(Object.keys(films).map(verifiedDelivery));
} else {
  fs.mkdirSync(delivery, { recursive: true });
  for (const slug of selection.length ? selection : Object.keys(films)) {
    assert.ok(films[slug], `Unknown film ${slug}`);
    process.stdout.write(cut(slug, films[slug]).join('\t') + '\n');
  }
  for (const [name, shot] of cache) process.stdout.write(`${name}	${shot.clicks} clicks	${shot.moves} camera moves
`);
}
