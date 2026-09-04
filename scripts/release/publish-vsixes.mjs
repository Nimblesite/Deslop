// [DEPLOY-PUBLISH-COMPLETE] Publishes every platform VSIX under artifacts/ to
// one registry, attempting all of them and failing afterwards — naming the
// platforms that never reached it. Shared by publish-marketplace and
// publish-openvsx so the two registries cannot drift apart.
//
// Issue #348: a `set -e` loop aborted after the first Marketplace timeout, so
// v0.31.0 reached darwin-arm64 only and nothing in the run named the four
// platforms it skipped. Aborting does not prevent a partial release — the
// earlier platforms are already live — it only hides which ones are missing.
// See docs/specs/release.md; pinned by test-release-publish-contract.mjs.

import { spawnSync } from "node:child_process";
import { readdirSync } from "node:fs";
import { join } from "node:path";

import { spawnableCommand } from "../lib/posix-shell.mjs";
import { VSIX_ARTIFACT_PREFIX, VSIX_PLATFORMS } from "./vsix-platforms.mjs";

/** Where the workflow's download-artifact step puts every uploaded artifact. */
const ARTIFACTS_DIR = "artifacts";

/** Registry-specific publish details; everything else is identical. */
const TOOLS = {
  vsce: {
    npxPackage: "@vscode/vsce@3.9.2",
    tokenVariable: "VSCE_PAT",
    registry: "the VS Code Marketplace",
  },
  ovsx: {
    npxPackage: "ovsx@1.0.0",
    tokenVariable: "OVSX_PAT",
    registry: "Open VSX",
  },
};

const tool = parseTool(process.argv.slice(2));
requireToken(tool);
const packages = discoverPlatformVsixes();
const failed = [...packages]
  .filter(([, vsix]) => !publish(tool, vsix))
  .map(([platform]) => platform);
report(tool, failed);

/**
 * Publishes one platform's VSIX. --skip-duplicate keeps the call idempotent,
 * so a re-run of a partially published tag republishes nothing.
 *
 * @returns {boolean} whether the VSIX reached the registry
 */
function publish(tool, vsix) {
  console.log(`Publishing ${vsix} to ${tool.registry}`);
  const [file, argv] = spawnableCommand("npx", [
    "--yes", tool.npxPackage, "publish", "--skip-duplicate", ...prereleaseFlag(), "--packagePath", vsix,
  ]);
  const result = spawnSync(file, argv, { stdio: "inherit" });
  if (result.status !== 0) console.error(`::warning::publish failed for ${vsix}`);
  return result.status === 0;
}

/** Names the platforms that never reached the registry, or reports success. */
function report(tool, failed) {
  if (failed.length > 0) {
    console.error(
      `::error::partial publish: these platforms are NOT on ${tool.registry}: ${failed.join(", ")}`,
    );
    process.exit(1);
  }
  console.log(
    `Published ${VSIX_PLATFORMS.length} platform VSIXes to ${tool.registry}: ${VSIX_PLATFORMS.join(", ")}`,
  );
}

/**
 * The Marketplace forbids a SemVer prerelease suffix in the version field, so
 * the VSIX carries the core MAJOR.MINOR.PATCH (see stamp-release-version.mjs)
 * and a hyphenated tag conveys prerelease status through the flag instead.
 */
function prereleaseFlag() {
  return (process.env.GITHUB_REF_NAME ?? "").includes("-") ? ["--pre-release"] : [];
}

/**
 * Every platform's VSIX, keyed by the platform its artifact directory names.
 * Identity, not count: five VSIXes that are not the five expected platforms
 * is the partial release this script exists to prevent.
 *
 * @returns {Map<string, string>} platform to VSIX path
 */
function discoverPlatformVsixes() {
  const found = new Map(
    artifactDirectories().map((name) => [
      name.slice(VSIX_ARTIFACT_PREFIX.length),
      vsixWithin(join(ARTIFACTS_DIR, name)),
    ]),
  );
  requireEveryPlatform(found);
  return new Map([...found].sort(([left], [right]) => left.localeCompare(right)));
}

/** The `vsix-<platform>` directories download-artifact created, or none. */
function artifactDirectories() {
  try {
    return readdirSync(ARTIFACTS_DIR, { withFileTypes: true })
      .filter((entry) => entry.isDirectory() && entry.name.startsWith(VSIX_ARTIFACT_PREFIX))
      .map((entry) => entry.name);
  } catch {
    return [];
  }
}

/** The single VSIX inside one artifact directory; more than one is ambiguous. */
function vsixWithin(directory) {
  const vsixes = readdirSync(directory).filter((entry) => entry.endsWith(".vsix"));
  if (vsixes.length !== 1) {
    console.error(`::error::${directory} holds ${vsixes.length} VSIX files, expected exactly 1`);
    process.exit(1);
  }
  return join(directory, vsixes[0]);
}

/**
 * Refuses to publish anything unless the discovered platforms are exactly the
 * expected set — a missing artifact upload must not become a partial release.
 */
function requireEveryPlatform(found) {
  const missing = VSIX_PLATFORMS.filter((platform) => !found.has(platform));
  const unexpected = [...found.keys()].filter((platform) => !VSIX_PLATFORMS.includes(platform));
  if (missing.length === 0 && unexpected.length === 0) return;
  console.error(
    `::error::refusing to publish an incomplete set of platform VSIXes` +
      `${missing.length > 0 ? `; missing: ${missing.join(", ")}` : ""}` +
      `${unexpected.length > 0 ? `; unexpected: ${unexpected.join(", ")}` : ""}`,
  );
  process.exit(1);
}

/** Fails loudly on an unknown or absent registry token. */
function requireToken(tool) {
  if (!process.env[tool.tokenVariable]) {
    console.error(`::error::${tool.tokenVariable} is not set; cannot publish to ${tool.registry}`);
    process.exit(1);
  }
}

/** Reads `--tool <vsce|ovsx>`; the platform set is not a caller's choice. */
function parseTool(argv) {
  const name = argv[1];
  if (argv[0] !== "--tool" || !Object.hasOwn(TOOLS, name)) {
    console.error("usage: node scripts/release/publish-vsixes.mjs --tool <vsce|ovsx>");
    process.exit(1);
  }
  return TOOLS[name];
}
