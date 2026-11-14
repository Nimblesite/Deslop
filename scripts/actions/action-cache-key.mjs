// Names the parse store the composite action restores and saves. [ACTION-CACHE]
//
// One workflow run may call the action from several jobs, and one job may scan
// several roots. Each needs its own store: with a shared name only the first
// job to save keeps its store, and the next run restores a store built for a
// different scan (gh #619).
//
// Usage: node scripts/actions/action-cache-key.mjs <version> <runnerOs> <scanPath> <scope> <runId>

import { createHash } from "node:crypto";
import { pathToFileURL } from "node:url";

import { writeOutputs } from "./action-github-output.mjs";

/** Hex characters of the scan-root digest kept in the key. */
const ROOT_DIGEST_LENGTH = 16;

/** Arguments the command line takes, in order. */
const ARGUMENT_NAMES = ["version", "runnerOs", "scanPath", "scope", "runId"];

/**
 * A short, key-safe name for a scan root: a digest, because a path holds
 * separators and may be long, and the key only needs it to be distinct.
 *
 * @param {string} scanPath
 * @returns {string}
 */
export function scanRootDigest(scanPath) {
  return createHash("sha256").update(scanPath).digest("hex").slice(0, ROOT_DIGEST_LENGTH);
}

/**
 * The store's key and its restore prefix. The prefix is the key without the
 * run id, so a run restores the newest store of the same version, runner OS,
 * scan root and scope, and nothing else.
 *
 * @param {{version: string, runnerOs: string, scanPath: string, scope: string, runId: string}} parts
 * @returns {{key: string, "restore-prefix": string}}
 */
export function cacheKey({ version, runnerOs, scanPath, scope, runId }) {
  for (const [name, value] of Object.entries({ version, runnerOs, scanPath, scope, runId })) {
    if (!value) throw new Error(`cache key needs a ${name}`);
  }
  const prefix = `deslop-${version}-${runnerOs}-${scanRootDigest(scanPath)}-${scope}-`;
  return { key: `${prefix}${runId}`, "restore-prefix": prefix };
}

function main(argv) {
  if (argv.length !== ARGUMENT_NAMES.length) {
    throw new Error(`usage: action-cache-key.mjs <${ARGUMENT_NAMES.join("> <")}>`);
  }
  const [version, runnerOs, scanPath, scope, runId] = argv;
  writeOutputs(cacheKey({ version, runnerOs, scanPath, scope, runId }));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
