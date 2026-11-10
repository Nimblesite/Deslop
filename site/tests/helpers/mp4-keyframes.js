import fs from 'node:fs';
// [DOC-FILMS-DELIVERY] Reads where a delivered MP4's video keyframes fall, from the
// container's own sample tables: no decoder, so every platform measures the same file.

const BOX_HEADER = 8;
const FULL_BOX_HEADER = 4;
const CONTAINERS = new Set(['moov', 'trak', 'mdia', 'minf', 'stbl']);
const VIDEO_HANDLER = 'vide';

function* boxes(buffer, start, end) {
  for (let offset = start; offset + BOX_HEADER <= end;) {
    const size = buffer.readUInt32BE(offset);
    const type = buffer.toString('latin1', offset + 4, offset + BOX_HEADER);
    if (size < BOX_HEADER) throw new Error(`unsupported ${type} box size ${size} at ${offset}`);
    yield {type, body: offset + BOX_HEADER, end: offset + size};
    offset += size;
  }
}

function collect(buffer, start, end, found = {}) {
  for (const box of boxes(buffer, start, end)) {
    if (CONTAINERS.has(box.type)) collect(buffer, box.body, box.end, found);
    else (found[box.type] ??= box);
  }
  return found;
}

function videoTrack(buffer) {
  const [moov] = [...boxes(buffer, 0, buffer.length)].filter(box => box.type === 'moov');
  if (!moov) throw new Error('no moov box');
  for (const trak of boxes(buffer, moov.body, moov.end)) {
    if (trak.type !== 'trak') continue;
    const tables = collect(buffer, trak.body, trak.end);
    if (buffer.toString('latin1', tables.hdlr.body + 8, tables.hdlr.body + 12) === VIDEO_HANDLER) return tables;
  }
  throw new Error('no video track');
}

function entries(buffer, box, width) {
  const count = buffer.readUInt32BE(box.body + FULL_BOX_HEADER);
  const first = box.body + FULL_BOX_HEADER + 4;
  return Array.from({length: count}, (_, index) => first + index * width);
}

/** Start time, in seconds, of every video sync sample, plus the track's duration. */
export function keyframeTimes(path) {
  const buffer = fs.readFileSync(path);
  const {mdhd, stts, stss} = videoTrack(buffer);
  if (!stss) throw new Error('every sample is a keyframe: no stss box');
  const version = buffer.readUInt8(mdhd.body);
  const timescale = buffer.readUInt32BE(mdhd.body + (version === 1 ? 20 : 12));
  const starts = [0];
  for (const entry of entries(buffer, stts, 8)) {
    const [count, delta] = [buffer.readUInt32BE(entry), buffer.readUInt32BE(entry + 4)];
    for (let sample = 0; sample < count; sample += 1) starts.push(starts.at(-1) + delta);
  }
  const sync = entries(buffer, stss, 4).map(entry => buffer.readUInt32BE(entry));
  return {times: sync.map(number => starts[number - 1] / timescale), duration: starts.at(-1) / timescale};
}
