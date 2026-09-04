// Reading and writing zip archives, in Node, with no external tool.
// [DEPLOY-GATE-PORTABILITY] [DEPLOY-VSIX-PACKAGE] [DEPLOY-JETBRAINS-PACKAGE]
//
// A VSIX and a JetBrains plugin are both zip archives, and every gate that
// inspects one used to shell out to Info-ZIP's `unzip`. Windows ships neither
// `unzip` nor `zip`, so on the platform that a `win32-x64` VSIX is built for —
// and where a maintainer is most likely to want to check one before publishing
// — the package verifiers could not run at all. They did not report a weaker
// answer; they aborted on a missing program, which is the same as having no
// gate.
//
// The format is small enough to read directly, and doing so buys two things a
// subprocess cannot: every entry's checksum is verified as it is read, so a
// corrupt archive is a named failure rather than a confusing one, and an
// archive written here is byte-for-byte reproducible, so a fixture cannot
// differ between two runs on one machine.
//
// Only what this repository's archives actually contain is supported —
// stored and deflated entries in a single-part archive. Anything else is
// refused by name rather than guessed at.

import { chmodSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { crc32, deflateRawSync, inflateRawSync } from "node:zlib";

/** Zip entry names are always spelled with this, whatever the host uses. */
export const ENTRY_SEPARATOR = "/";

/** The three record signatures a reader meets, in the order it meets them. */
const END_OF_CENTRAL_DIRECTORY = 0x06054b50;
const CENTRAL_FILE_HEADER = 0x02014b50;
const LOCAL_FILE_HEADER = 0x04034b50;

/** Fixed record sizes, in bytes, excluding the variable-length name. */
const END_RECORD_BYTES = 22;
const CENTRAL_HEADER_BYTES = 46;
const LOCAL_HEADER_BYTES = 30;

/** An archive comment is a 16-bit length, so the end record starts within this. */
const MAX_END_RECORD_BYTES = END_RECORD_BYTES + 0xffff;

/** The two compression methods this repository's archives ever use. */
const STORED = 0;
const DEFLATED = 8;

/** What a field holds when its real value lives in a Zip64 record instead. */
const ZIP64_COUNT = 0xffff;
const ZIP64_OFFSET = 0xffffffff;

/** "Made by" a Unix system at format version 2.0, so the mode below is read. */
const VERSION_MADE_BY = (3 << 8) | 20;
const VERSION_NEEDED = 20;

/** The MS-DOS attribute bit marking a directory, and the two Unix type bits. */
const DOS_DIRECTORY = 0x10;
const UNIX_REGULAR_FILE = 0o100000;
const UNIX_DIRECTORY = 0o040000;

/** The permission bits carried across an archive and restored on extraction. */
const PERMISSION_BITS = 0o777;

/** Where each header's description of the entry begins. */
const LOCAL_DESCRIPTION_AT = 8;
const CENTRAL_DESCRIPTION_AT = 10;

/**
 * Opens `archivePath` and reads its central directory once.
 *
 * @param {string} archivePath path to a zip archive
 * @returns {{names: string[], read: (name: string) => Buffer, readText: (name: string) => string, extract: (name: string, destination: string) => string}}
 */
export function openArchive(archivePath) {
  const buffer = readFileSync(archivePath);
  const entries = readCentralDirectory(buffer, archivePath);
  const named = (name) => entryNamed(entries, name, archivePath);
  return {
    names: entries.map((entry) => entry.name),
    read: (name) => readEntry(buffer, named(name), archivePath),
    readText: (name) => readEntry(buffer, named(name), archivePath).toString("utf8"),
    extract: (name, destination) => extractEntry(buffer, named(name), destination, archivePath),
  };
}

/**
 * Writes `archivePath` holding `entryRoot` and everything beneath it, resolved
 * inside `sourceRoot`. Children are archived in sorted order, so the same tree
 * always produces the same bytes.
 *
 * @param {string} archivePath archive to create
 * @param {string} sourceRoot directory `entryRoot` is resolved against
 * @param {string} entryRoot name of the top-level entry, and its prefix
 * @returns {string} `archivePath`
 */
export function writeArchive(archivePath, sourceRoot, entryRoot) {
  const parts = [];
  const directory = [];
  let offset = 0;
  for (const staged of stagedEntries(sourceRoot, entryRoot)) {
    const record = compress(staged);
    const header = localHeader(record);
    parts.push(header, record.stored);
    directory.push(centralHeader(record, offset));
    offset += header.length + record.stored.length;
  }
  const central = Buffer.concat(directory);
  parts.push(central, endRecord(directory.length, central.length, offset));
  writeFileSync(archivePath, Buffer.concat(parts));
  return archivePath;
}

/** The entry called `name`, or a failure naming the archive that lacks it. */
function entryNamed(entries, name, archivePath) {
  const entry = entries.find((candidate) => candidate.name === name);
  if (!entry) throw new Error(`${archivePath} has no entry ${name}`);
  return entry;
}

/** Every central-directory entry, in the order the archive lists them. */
function readCentralDirectory(buffer, archivePath) {
  const end = findEndRecord(buffer, archivePath);
  const count = buffer.readUInt16LE(end + 10);
  const start = buffer.readUInt32LE(end + 16);
  if (count === ZIP64_COUNT || start === ZIP64_OFFSET) {
    throw new Error(`${archivePath} is a Zip64 archive, which this reader does not support`);
  }
  const entries = [];
  let at = start;
  while (entries.length < count) at = readCentralHeader(buffer, at, entries, archivePath);
  return entries;
}

/** Offset of the end-of-central-directory record, searching back from the end. */
function findEndRecord(buffer, archivePath) {
  const earliest = Math.max(0, buffer.length - MAX_END_RECORD_BYTES);
  for (let at = buffer.length - END_RECORD_BYTES; at >= earliest; at -= 1) {
    if (buffer.readUInt32LE(at) === END_OF_CENTRAL_DIRECTORY) return at;
  }
  throw new Error(
    `${archivePath} carries no end-of-central-directory record, so it is not a zip archive`,
  );
}

/** Appends the entry at `at` to `entries` and returns the next entry's offset. */
function readCentralHeader(buffer, at, entries, archivePath) {
  if (at + CENTRAL_HEADER_BYTES > buffer.length || buffer.readUInt32LE(at) !== CENTRAL_FILE_HEADER) {
    throw new Error(`${archivePath} central directory ends early, at byte ${at}`);
  }
  const nameLength = buffer.readUInt16LE(at + 28);
  const nameStart = at + CENTRAL_HEADER_BYTES;
  entries.push({
    name: buffer.toString("utf8", nameStart, nameStart + nameLength),
    method: buffer.readUInt16LE(at + 10),
    checksum: buffer.readUInt32LE(at + 16),
    storedSize: buffer.readUInt32LE(at + 20),
    mode: (buffer.readUInt32LE(at + 38) >>> 16) & PERMISSION_BITS,
    localHeaderOffset: buffer.readUInt32LE(at + 42),
  });
  return nameStart + nameLength + buffer.readUInt16LE(at + 30) + buffer.readUInt16LE(at + 32);
}

/** The entry's content, decompressed and checked against its recorded checksum. */
function readEntry(buffer, entry, archivePath) {
  const at = entry.localHeaderOffset;
  if (buffer.readUInt32LE(at) !== LOCAL_FILE_HEADER) {
    throw new Error(`${archivePath} entry ${entry.name} has no local header at byte ${at}`);
  }
  const start = at + LOCAL_HEADER_BYTES + buffer.readUInt16LE(at + 26) + buffer.readUInt16LE(at + 28);
  const raw = buffer.subarray(start, start + entry.storedSize);
  const bytes = decompress(entry, raw, archivePath);
  if (crc32(bytes) !== entry.checksum) {
    throw new Error(`${archivePath} entry ${entry.name} does not match the checksum recorded for it`);
  }
  return bytes;
}

/** Undoes whichever of the two supported compression methods was used. */
function decompress(entry, raw, archivePath) {
  if (entry.method === STORED) return Buffer.from(raw);
  if (entry.method === DEFLATED) return inflateRawSync(raw);
  throw new Error(
    `${archivePath} entry ${entry.name} uses compression method ${entry.method}, which this reader does not support`,
  );
}

/** Writes one entry beneath `destination`, restoring its recorded permissions. */
function extractEntry(buffer, entry, destination, archivePath) {
  const path = join(destination, ...entry.name.split(ENTRY_SEPARATOR));
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, readEntry(buffer, entry, archivePath));
  if (entry.mode !== 0) chmodSync(path, entry.mode);
  return path;
}

/** `entryRoot` and its descendants, parents before children, sorted throughout. */
function stagedEntries(sourceRoot, entryRoot) {
  const staged = [];
  stage(sourceRoot, entryRoot, staged);
  return staged;
}

/** Appends `name` — and, for a directory, everything under it — to `staged`. */
function stage(sourceRoot, name, staged) {
  const path = join(sourceRoot, ...name.split(ENTRY_SEPARATOR));
  const stats = statSync(path);
  const mode = stats.mode & PERMISSION_BITS;
  if (!stats.isDirectory()) {
    staged.push({ name, mode, bytes: readFileSync(path) });
    return;
  }
  staged.push({ name: `${name}${ENTRY_SEPARATOR}`, mode, bytes: Buffer.alloc(0) });
  for (const child of readdirSync(path).sort()) {
    stage(sourceRoot, `${name}${ENTRY_SEPARATOR}${child}`, staged);
  }
}

/** Deflates the entry, keeping it stored whenever deflating would not shrink it. */
function compress(staged) {
  const isDirectory = staged.name.endsWith(ENTRY_SEPARATOR);
  const deflated = isDirectory ? staged.bytes : deflateRawSync(staged.bytes);
  const shrank = !isDirectory && deflated.length < staged.bytes.length;
  return {
    name: staged.name,
    mode: staged.mode,
    isDirectory,
    checksum: crc32(staged.bytes),
    size: staged.bytes.length,
    method: shrank ? DEFLATED : STORED,
    stored: shrank ? deflated : staged.bytes,
  };
}

/** The header written immediately before an entry's content. */
function localHeader(record) {
  const header = startHeader(record, LOCAL_FILE_HEADER, LOCAL_HEADER_BYTES);
  header.writeUInt16LE(VERSION_NEEDED, 4);
  describeEntry(header, record, LOCAL_DESCRIPTION_AT);
  return header;
}

/** The catalogue entry, carrying the Unix mode extraction restores. */
function centralHeader(record, offset) {
  const header = startHeader(record, CENTRAL_FILE_HEADER, CENTRAL_HEADER_BYTES);
  header.writeUInt16LE(VERSION_MADE_BY, 4);
  header.writeUInt16LE(VERSION_NEEDED, 6);
  describeEntry(header, record, CENTRAL_DESCRIPTION_AT);
  header.writeUInt32LE(externalAttributes(record), 38);
  header.writeUInt32LE(offset, 42);
  return header;
}

/** A header sized for its name, with the signature and the name in place. */
function startHeader(record, signature, headerBytes) {
  const name = Buffer.from(record.name, "utf8");
  const header = Buffer.alloc(headerBytes + name.length);
  header.writeUInt32LE(signature, 0);
  name.copy(header, headerBytes);
  return header;
}

/**
 * The five fields that describe the entry itself, identical in both headers
 * and in the same order — the central directory's copy simply starts two
 * bytes later, because it carries a "version made by" the local one does not.
 */
function describeEntry(header, record, at) {
  header.writeUInt16LE(record.method, at);
  header.writeUInt32LE(record.checksum, at + 6);
  header.writeUInt32LE(record.stored.length, at + 10);
  header.writeUInt32LE(record.size, at + 14);
  header.writeUInt16LE(Buffer.byteLength(record.name, "utf8"), at + 18);
}

/** Unix mode in the high half, MS-DOS attributes in the low half. */
function externalAttributes(record) {
  const type = record.isDirectory ? UNIX_DIRECTORY : UNIX_REGULAR_FILE;
  const dos = record.isDirectory ? DOS_DIRECTORY : 0;
  // Unsigned last: `|` yields a signed 32-bit result, so shifting the mode
  // into the high half and then OR-ing anything at all turns it negative.
  return ((((type | record.mode) << 16) | dos) >>> 0);
}

/** The trailing record naming where the catalogue is and how big it is. */
function endRecord(count, size, offset) {
  const record = Buffer.alloc(END_RECORD_BYTES);
  record.writeUInt32LE(END_OF_CENTRAL_DIRECTORY, 0);
  record.writeUInt16LE(count, 8);
  record.writeUInt16LE(count, 10);
  record.writeUInt32LE(size, 12);
  record.writeUInt32LE(offset, 16);
  return record;
}
