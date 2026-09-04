// Contract for the zip reader and writer the deployment gates run on.
// [DEPLOY-GATE-PORTABILITY] [DEPLOY-VSIX-PACKAGE] [DEPLOY-JETBRAINS-PACKAGE]
//
// A VSIX and a JetBrains plugin are zip archives, and the verifiers that
// inspect them used to shell out to Info-ZIP. Windows ships neither `zip` nor
// `unzip`, so on that platform the package gates aborted on a missing program
// — which reads as "no gate", not as a failure.
//
// The risk in replacing them is that the reader and the writer here agree with
// each other and with nothing else, which would be a gate that proves only its
// own internal consistency. So the reader is held against an archive this
// repository did not produce — the checked-in Gradle wrapper jar, written years
// ago by a different tool — before the writer is held against the reader.
// Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path, { join, resolve } from "node:path";

import { repoRoot } from "../lib/repo-root.mjs";
import { ENTRY_SEPARATOR, openArchive, writeArchive } from "../lib/zip.mjs";

/** A zip this repository did not write, checked in and byte-stable. */
const FOREIGN_ARCHIVE = resolve(repoRoot, "clients/jetbrains/gradle/wrapper/gradle-wrapper.jar");

/** Two entries that archive is known to carry, one text and one compiled. */
const FOREIGN_MANIFEST = "META-INF/MANIFEST.MF";
const FOREIGN_CLASS = "org/gradle/wrapper/GradleWrapperMain.class";

/** What the first bytes of each must be if they were decompressed correctly. */
const MANIFEST_OPENING = "Manifest-Version:";
const CLASS_FILE_MAGIC = [0xca, 0xfe, 0xba, 0xbe];

/** The character a host path uses on Windows and an entry name never does. */
const HOST_JOINER = String.fromCharCode(0x5c);

/** A staged tree the writer is exercised against. */
const ARCHIVE_ROOT = "extension";
const NESTED_DIRECTORY = "bin";
const TEXT_ENTRY = `${ARCHIVE_ROOT}${ENTRY_SEPARATOR}package.json`;
const NESTED_ENTRY = `${ARCHIVE_ROOT}${ENTRY_SEPARATOR}${NESTED_DIRECTORY}${ENTRY_SEPARATOR}deslop`;

/** Long enough that deflating it must win, and repetitive so by a wide margin. */
const COMPRESSIBLE_TEXT = "deslop deslop deslop\n".repeat(500);

/** The mode a staged binary carries, and the one a plain file carries. */
const EXECUTABLE_MODE = 0o755;
const PLAIN_MODE = 0o644;

test("a zip written by another tool reads back entry by entry", () => {
  const archive = openArchive(FOREIGN_ARCHIVE);
  assert.ok(archive.names.includes(FOREIGN_MANIFEST), `${FOREIGN_ARCHIVE} must carry ${FOREIGN_MANIFEST}`);
  assert.ok(archive.names.includes(FOREIGN_CLASS), `${FOREIGN_ARCHIVE} must carry ${FOREIGN_CLASS}`);
  assert.ok(
    archive.readText(FOREIGN_MANIFEST).startsWith(MANIFEST_OPENING),
    "a stored or deflated text entry must come back as the text that was put in",
  );
  // A class file is deflated and checksummed; reading it proves the inflate
  // path and the CRC check against bytes nothing here produced.
  assert.deepEqual([...archive.read(FOREIGN_CLASS).subarray(0, CLASS_FILE_MAGIC.length)], CLASS_FILE_MAGIC);
});

test("entry names are archive names, never host paths", () => {
  const archive = openArchive(FOREIGN_ARCHIVE);
  for (const name of archive.names) {
    assert.equal(name.includes(HOST_JOINER), false, `${name} carries a host separator`);
    assert.equal(path.posix.isAbsolute(name), false, `${name} escapes the archive root`);
    assert.equal(name.startsWith(".."), false, `${name} escapes the archive root`);
  }
});

test("an entry the archive does not carry is named, not guessed at", () => {
  const archive = openArchive(FOREIGN_ARCHIVE);
  assert.throws(
    () => archive.read("META-INF/NOT-THERE"),
    (error) => error.message.includes("META-INF/NOT-THERE") && error.message.includes(FOREIGN_ARCHIVE),
  );
});

test("a file that is not an archive is refused by name", () => {
  const work = workDirectory();
  const notAnArchive = join(work, "plain.txt");
  writeFileSync(notAnArchive, "this is not a zip");
  assert.throws(() => openArchive(notAnArchive), /end-of-central-directory/);
  const truncated = join(work, "truncated.zip");
  writeFileSync(truncated, readFileSync(FOREIGN_ARCHIVE).subarray(0, 4096));
  assert.throws(() => openArchive(truncated), /end-of-central-directory/);
});

test("a staged tree survives the round trip through an archive", () => {
  const work = workDirectory();
  const archivePath = stageTree(work);
  const archive = openArchive(archivePath);
  assert.deepEqual(archive.names, [
    `${ARCHIVE_ROOT}${ENTRY_SEPARATOR}`,
    `${ARCHIVE_ROOT}${ENTRY_SEPARATOR}${NESTED_DIRECTORY}${ENTRY_SEPARATOR}`,
    NESTED_ENTRY,
    TEXT_ENTRY,
  ]);
  assert.equal(archive.readText(TEXT_ENTRY), COMPRESSIBLE_TEXT);
  assert.equal(archive.readText(NESTED_ENTRY), COMPRESSIBLE_TEXT);
  // The host joined those paths with its own separator; the archive must not
  // have kept it, or every entry name a verifier matches on would be wrong on
  // exactly one platform.
  for (const name of archive.names) {
    assert.equal(name.includes(HOST_JOINER), false, `${name} carries a host separator`);
  }
});

test("extraction restores the permissions the staged file had", () => {
  const work = workDirectory();
  const archivePath = stageTree(work);
  const destination = join(work, "out");
  const extracted = openArchive(archivePath).extract(NESTED_ENTRY, destination);
  assert.equal(readFileSync(extracted, "utf8"), COMPRESSIBLE_TEXT);
  // Stated against the source rather than against 0o755, because NTFS cannot
  // record an execute bit and a fixed expectation would be asserting the host.
  const staged = join(work, ...NESTED_ENTRY.split(ENTRY_SEPARATOR));
  assert.equal(statSync(extracted).mode & 0o777, statSync(staged).mode & 0o777);
});

test("entries are compressed, and the same tree always produces the same bytes", () => {
  const work = workDirectory();
  const first = readFileSync(stageTree(work));
  const second = readFileSync(stageTree(work, "second.zip"));
  assert.deepEqual(first, second, "two archives of one tree must not differ");
  assert.ok(
    first.length < COMPRESSIBLE_TEXT.length,
    `${first.length} bytes for two copies of ${COMPRESSIBLE_TEXT.length} means nothing was deflated`,
  );
});

/** A fresh temp directory, removed when the process exits. */
function workDirectory() {
  const work = mkdtempSync(join(tmpdir(), "deslop-zip-"));
  process.on("exit", () => rmSync(work, { recursive: true, force: true }));
  return work;
}

/** Stages the tree under `work` and archives it, returning the archive path. */
function stageTree(work, archiveName = "first.zip") {
  const nested = join(work, ARCHIVE_ROOT, NESTED_DIRECTORY);
  mkdirSync(nested, { recursive: true });
  writeFileSync(join(work, ...TEXT_ENTRY.split(ENTRY_SEPARATOR)), COMPRESSIBLE_TEXT);
  chmodSync(join(work, ...TEXT_ENTRY.split(ENTRY_SEPARATOR)), PLAIN_MODE);
  writeFileSync(join(work, ...NESTED_ENTRY.split(ENTRY_SEPARATOR)), COMPRESSIBLE_TEXT);
  chmodSync(join(work, ...NESTED_ENTRY.split(ENTRY_SEPARATOR)), EXECUTABLE_MODE);
  return writeArchive(join(work, archiveName), work, ARCHIVE_ROOT);
}
