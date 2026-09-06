// Verifies a downloaded release archive against its published `.sha256`
// sidecar before anything is extracted. [ACTION-VERIFY].
//
// Node computes the digest so the action needs no per-OS branch between
// `sha256sum`, `shasum -a 256`, and whatever Windows offers.
//
// Usage: node scripts/action-verify-checksum.mjs <archivePath> <checksumPath>

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

/**
 * Returns the digest recorded in a `<hash>  <filename>` sidecar.
 *
 * @param {string} checksumText
 * @returns {string}
 */
export function expectedDigest(checksumText) {
  const trimmed = checksumText.trim();
  const end = [...trimmed].findIndex((character) => character === " " || character === "\t");
  const digest = end === -1 ? trimmed : trimmed.slice(0, end);
  if (!digest) throw new Error("checksum file is empty");
  return digest.toLowerCase();
}

/**
 * Throws unless `archivePath` hashes to the digest recorded in `checksumPath`.
 *
 * @param {string} archivePath
 * @param {string} checksumPath
 * @returns {string} the verified digest
 */
export function verifyChecksum(archivePath, checksumPath) {
  const expected = expectedDigest(readFileSync(checksumPath, "utf8"));
  const actual = createHash("sha256").update(readFileSync(archivePath)).digest("hex");
  if (actual !== expected) {
    throw new Error(`checksum mismatch for ${archivePath}: expected ${expected}, computed ${actual}`);
  }
  return actual;
}

function main(argv) {
  const [archivePath, checksumPath] = argv;
  if (!archivePath || !checksumPath) {
    throw new Error("usage: action-verify-checksum.mjs <archivePath> <checksumPath>");
  }
  console.log(`Verified sha256 ${verifyChecksum(archivePath, checksumPath)} for ${archivePath}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
