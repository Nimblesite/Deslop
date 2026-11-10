// [DOC-FILMS-LIVE] App footage only, through SharpLsp's automatic camera.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readTrack, WIDGETS } from '../../../SharpLsp/src/website/tools/video/kit/track.mjs';
import { CAMERA, writeCamera } from '../../../SharpLsp/src/website/tools/video/kit/zoom.mjs';
import { renderCaptionTrack } from '../../../Videos/projects/deslop/videos/doc-series/caption-track.mjs';
import { stamp } from '../../../Videos/factory/film/delivery.mjs';
const root = path.resolve(import.meta.dirname, '../..');
const videos = path.resolve(root, '../Videos');
const series = path.join(videos, 'projects/deslop/videos');
const captures = path.join(videos, 'projects/deslop/captures/docs-tutorial');
const work = path.join(root, 'target/docs-tutorial/render');
const stories = JSON.parse(fs.readFileSync(path.join(series, 'doc-series/user-workflows.json'), 'utf8'));
const fps = 30;
const encode = ['-c:v', 'libx264', '-threads', '2', '-preset', 'veryfast', '-crf', '18', '-pix_fmt', 'yuv420p'];
const run = (command, args) => execFileSync(command, args, { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
const ffmpeg = args => run('ffmpeg', ['-nostdin', '-y', '-v', 'error', ...args]);
const probe = file => JSON.parse(run('ffprobe', ['-v', 'error', '-show_streams', '-show_format', '-of', 'json', file]));
const duration = file => Number(probe(file).format.duration);
const digest = file => createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const fit = 'scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=0x111111,setsar=1';

function camera(scene, directory, index) {
  const source = path.join(captures, scene.shot + '.mp4');
  assert.ok(fs.existsSync(source), `Record ${scene.shot} before producing ${scene.title}`);
  const track = readTrack(path.join(captures, scene.shot + '.track.json'));
  const sourceSeconds = duration(source);
  const seconds = scene.end - scene.start;
  const speed = Math.max(1, sourceSeconds / seconds);
  const video = probe(source).streams.find(stream => stream.codec_type === 'video');
  {
    const panel = WIDGETS.findIndex(([selector]) => selector === '.part.panel');
    track.samples = track.samples.map(sample => ({ ...sample, w: sample.w.filter(([widget]) => widget !== panel) }));
  }
  const deepest = CAMERA.deepest;
  const tighten = CAMERA.tighten;
  CAMERA.tighten = 1.05;
  const comparison = ['review-compare', 'review-pair', 'review-edited-compare', 'osprey-compare'].includes(scene.shot);
  const terminal = scene.shot.startsWith('agent-') || ['action-limit', 'action-changes', 'config-override', 'config-limit', 'osprey-config'].includes(scene.shot);
  CAMERA.deepest = comparison ? 1.12 : terminal ? 1.15 : 1.3;
  const commands = path.join(directory, `${index}.cmd`);
  const moves = writeCamera(track, { file: commands, name: 'view', fps, frames: Math.ceil(sourceSeconds * fps), scale: video.width / track.size[0], speed });
  CAMERA.deepest = deepest;
  CAMERA.tighten = tighten;
  assert.ok(moves.length > 1, `No camera movement in ${scene.shot}`);
  const output = path.join(directory, `${index}.mp4`);
  ffmpeg(['-filter_threads', '2', '-i', source, '-an', '-vf',
    `format=yuv444p,sendcmd=f='${commands}',crop@view=w=${video.width}:h=${video.height}:x=0:y=0:exact=1,setpts=PTS/${speed},fps=${fps},${fit},tpad=stop_mode=clone:stop_duration=${seconds}`,
    '-t', String(seconds), ...encode, output]);
  return { output, source, sourceSeconds, speed, cameraMoves: moves.length - 1, sha256: digest(source) };
}

function tracks(data, cues, dist, base) {
  const captions = 'WEBVTT\n\n' + cues.map(cue => `${stamp(cue.start)} --> ${stamp(cue.end)}\n${cue.text}`).join('\n\n') + '\n';
  const chapters = 'WEBVTT\n\n' + data.scenes.map(scene => `${stamp(scene.start)} --> ${stamp(scene.end)}\n${scene.title}`).join('\n\n') + '\n';
  fs.writeFileSync(path.join(dist, base + '.en.vtt'), captions);
  fs.writeFileSync(path.join(dist, base + '.chapters.vtt'), chapters);
}

function install(data, dist, base, sources, cues) {
  const media = path.join(dist, base + '.mp4');
  const result = probe(media);
  const video = result.streams.find(stream => stream.codec_type === 'video');
  const audio = result.streams.find(stream => stream.codec_type === 'audio');
  assert.deepEqual([video.codec_name, video.width, video.height, audio.codec_name], ['h264', 1920, 1080, 'aac']);
  assert.ok(Math.abs(Number(video.duration) - data.duration) < 0.18);
  const destination = path.join(root, 'site/src/assets/video/docs');
  for (const suffix of ['.mp4', '-poster.jpg', '.en.vtt', '.chapters.vtt']) fs.copyFileSync(path.join(dist, base + suffix), path.join(destination, base + suffix));
  const catalogFile = path.join(root, 'site/src/_data/docFilms.json');
  const catalog = JSON.parse(fs.readFileSync(catalogFile, 'utf8'));
  const rounded = Math.round(data.duration);
  catalog[data.slug] = { ...catalog[data.slug], title: catalog[data.slug]?.title ?? data.scenes[0].title, duration: `${Math.floor(rounded / 60)}:${String(rounded % 60).padStart(2, '0')}`,
    captionCueCount: cues.length, transcript: data.lines.map((line, index) => ({ title: data.scenes[index].title,
      seconds: Math.floor(line.start), stamp: `${Math.floor(line.start / 60)}:${String(Math.floor(line.start) % 60).padStart(2, '0')}`, text: line.text })) };
  fs.writeFileSync(catalogFile, JSON.stringify(catalog, null, 2) + '\n');
  const report = ['# ' + data.slug + ' recorded workflow verification [DOC-FILMS-LIVE]', '',
    `- Duration: ${data.duration.toFixed(3)} seconds`, '- Picture: real application footage for the entire film; captions and small step labels only',
    `- Narrator: ${data.voice}`, `- Spoken script: ${data.lines.length} current lines, matching the cached speech receipts`,
    `- Caption cues: ${cues.length}`, '- Video: H.264, 1920 x 1080; audio: AAC', `- SHA-256: ${digest(media)}`, '',
    '| Step | Recorded action | Source seconds | Playback speed | Automatic camera moves |', '| --- | --- | ---: | ---: | ---: |',
    ...sources.map((shot, index) => `| ${data.scenes[index].title} | ${data.scenes[index].shot} | ${shot.sourceSeconds.toFixed(2)} | ${shot.speed.toFixed(2)} | ${shot.cameraMoves} |`), '',
    'Narration and visible workflow must also receive finished-frame review. Encoding checks alone do not establish usefulness.', '',
  ].join('\n');
  fs.writeFileSync(path.join(root, 'reports/videos', data.slug + '-media.md'), report);
}

async function produce(slug) {
  const dist = path.join(series, 'docs-' + slug, 'dist');
  const base = 'deslop-docs-' + slug;
  const data = JSON.parse(fs.readFileSync(path.join(dist, base + '.workflow.json'), 'utf8'));
  assert.ok(stories[slug].every((scene, index) => scene.narration === data.lines[index].text), 'The soundtrack is stale');
  const directory = path.join(work, slug);
  fs.mkdirSync(directory, { recursive: true });
  try {
    const sources = data.scenes.map((scene, index) => camera(scene, directory, index));
    data.scenes.forEach((scene, index) => { scene.speed = sources[index].speed; });
    const list = path.join(directory, 'film.ffconcat');
    fs.writeFileSync(list, sources.map(shot => `file '${shot.output}'`).join('\n') + '\n');
    const overlay = path.join(directory, 'captions.mov');
    const cues = await renderCaptionTrack(data, overlay);
    const media = path.join(dist, base + '.mp4');
    ffmpeg(['-f', 'concat', '-safe', '0', '-i', list, '-i', overlay, '-i', data.soundtrack,
      '-filter_complex_threads', '2', '-filter_complex', '[0:v][1:v]overlay=0:0:format=auto[v]', '-map', '[v]', '-map', '2:a:0',
      ...encode, '-c:a', 'aac', '-b:a', '160k', '-t', String(data.duration), '-movflags', '+faststart', media]);
    ffmpeg(['-ss', String(Math.max(0, data.scenes[0].end - 1)), '-i', media, '-frames:v', '1', '-q:v', '3', path.join(dist, base + '-poster.jpg')]);
    tracks(data, cues, dist, base);
    fs.writeFileSync(path.join(dist, base + '.timeline.json'), JSON.stringify(data, null, 2) + '\n');
    if (process.argv.includes('--install')) install(data, dist, base, sources, cues);
    process.stdout.write(`${slug}: ${data.duration.toFixed(1)}s of actual app use\n`);
  } finally { fs.rmSync(directory, { recursive: true, force: true }); }
}
for (const slug of process.argv.slice(2).filter(arg => !arg.startsWith('--')).length ? process.argv.slice(2).filter(arg => !arg.startsWith('--')) : Object.keys(stories)) await produce(slug);
