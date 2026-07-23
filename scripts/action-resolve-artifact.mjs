// Resolves which published deslop release archive a runner should install,
// and which CLI version the action installs. [ACTION-RESOLVE], [ACTION-VERSION].
//
// Usage:
//   node scripts/action-resolve-artifact.mjs <runnerOs> <runnerArch> [version] [actionRef]
//
// Writes `version`, `artifact`, `archive`, `stage`, `url` and `checksum-url` to
// $GITHUB_OUTPUT.

import { pathToFileURL } from "node:url";

import { writeOutputs } from "./action-github-output.mjs";

const RELEASE_BASE = "https://github.com/Nimblesite/Deslop/releases/download";

// GitHub's `runner.os` / `runner.arch` pair mapped to the `artifact_name`
// published by .github/workflows/release.yml. Windows ships a `.zip`; every
// other target ships a `.tar.gz`. Deliberately distinct from the Shipwright
// platform ids (`darwin-arm64`, `win32-x64`) used by the manifest verifiers —
// these are release *asset* names.
const ARTIFACTS = new Map([
  ["Linux/X64", "linux-x64"],
  ["Linux/ARM64", "linux-arm64"],
  ["macOS/X64", "macos-x64"],
  ["macOS/ARM64", "macos-arm64"],
  ["Windows/X64", "windows-x64"],
]);

/**
 * Maps a runner to its release asset name.
 *
 * @param {string} runnerOs `$RUNNER_OS`
 * @param {string} runnerArch `$RUNNER_ARCH`
 * @returns {string}
 */
export function releaseArtifact(runnerOs, runnerArch) {
  const artifact = ARTIFACTS.get(`${runnerOs}/${runnerArch}`);
  if (artifact) return artifact;
  const supported = [...ARTIFACTS.keys()].join(", ");
  throw new Error(`unsupported runner ${runnerOs}/${runnerArch}; deslop publishes ${supported}`);
}

/**
 * Returns the CLI version to install: the explicit input when given, otherwise
 * the tag this action was pinned to. A commit-SHA pin carries no version, so it
 * fails loudly rather than silently resolving to "latest".
 *
 * @param {string} versionInput
 * @param {string} actionRef `github.action_ref`
 * @returns {string}
 */
export function resolveVersion(versionInput, actionRef) {
  const explicit = stripLeadingV(versionInput.trim());
  if (explicit) return explicit;
  const derived = stripLeadingV(actionRef.trim());
  if (isSemver(derived)) return derived;
  throw new Error(
    `cannot derive the deslop version from action ref "${actionRef}" — ` +
      "set the `version` input explicitly, which is required when you pin this action to a commit SHA",
  );
}

/**
 * Resolves every download coordinate for one runner in a single pass.
 *
 * @param {string} runnerOs
 * @param {string} runnerArch
 * @param {string} versionInput
 * @param {string} actionRef
 * @returns {{version: string, artifact: string, archive: string, stage: string, url: string, checksumUrl: string}}
 */
export function resolveRelease(runnerOs, runnerArch, versionInput, actionRef) {
  const version = resolveVersion(versionInput, actionRef);
  const artifact = releaseArtifact(runnerOs, runnerArch);
  const stage = `deslop-${version}-${artifact}`;
  const archive = `${stage}.${archiveExtension(artifact)}`;
  const url = `${RELEASE_BASE}/v${version}/${archive}`;
  return { version, artifact, archive, stage, url, checksumUrl: `${url}.sha256` };
}

function archiveExtension(artifact) {
  return artifact.startsWith("windows") ? "zip" : "tar.gz";
}

function stripLeadingV(value) {
  return value.startsWith("v") ? value.slice(1) : value;
}

// Deliberately hand-parsed rather than regex-matched: this repo prohibits regex
// over structured data, and a MAJOR.MINOR.PATCH check needs no more than this.
function isSemver(candidate) {
  const [core] = candidate.split("-", 1);
  const parts = core.split(".");
  return parts.length === 3 && parts.every(isNonNegativeInteger);
}

function isNonNegativeInteger(part) {
  return part.length > 0 && [...part].every((character) => character >= "0" && character <= "9");
}

function main(argv) {
  const [runnerOs, runnerArch, versionInput = "", actionRef = ""] = argv;
  if (!runnerOs || !runnerArch) {
    throw new Error("usage: action-resolve-artifact.mjs <runnerOs> <runnerArch> [version] [actionRef]");
  }
  const release = resolveRelease(runnerOs, runnerArch, versionInput, actionRef);
  writeOutputs({
    version: release.version,
    artifact: release.artifact,
    archive: release.archive,
    stage: release.stage,
    url: release.url,
    "checksum-url": release.checksumUrl,
  });
  console.log(`deslop ${release.version} for ${release.artifact}: ${release.archive}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
