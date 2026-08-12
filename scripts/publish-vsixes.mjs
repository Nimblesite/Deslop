// [DEPLOY-PUBLISH-COMPLETE] Publishes every platform VSIX under artifacts/ to
// one registry, retrying each platform independently and failing only after
// all platforms have been attempted — naming the ones that never reached the
// registry. Shared by publish-marketplace and publish-openvsx so the two
// registries cannot drift apart (issue #348: one transient Marketplace
// timeout aborted the publish loop and left v0.31.0 published for
// darwin-arm64 only, with no signal naming the missing platforms).
//
// Pinned by scripts/test-release-publish-contract.mjs, which executes the
// workflow's publish steps against a scripted registry timeout.

import { spawnSync } from "node:child_process";
import { readdirSync } from "node:fs";
import { join } from "node:path";

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
const MAX_ATTEMPTS = 3;

const { tool, expected } = parseArguments(process.argv.slice(2));
requireToken(tool);
const vsixes = discoverVsixes(expected);
const failed = [];
for (const vsix of vsixes) {
  if (!(await publishWithRetry(tool, vsix))) failed.push(vsix);
}
if (failed.length > 0) {
  console.error(
    `::error::partial publish: these platform VSIXes are NOT on ${tool.registry}: ${failed.join(", ")}`,
  );
  process.exit(1);
}
console.log(`Published ${vsixes.length} VSIX(es) to ${tool.registry}`);

// One platform, up to MAX_ATTEMPTS publishes. --skip-duplicate keeps every
// attempt (and any re-run of a partially published tag) idempotent. Backoff
// grows linearly; DESLOP_PUBLISH_BACKOFF_SECONDS lets the contract suite run
// without sleeping.
async function publishWithRetry(tool, vsix) {
  for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
    console.log(`Publishing ${vsix} to ${tool.registry} (attempt ${attempt}/${MAX_ATTEMPTS})`);
    const result = spawnSync(
      "npx",
      ["--yes", tool.npxPackage, "publish", "--skip-duplicate", ...prereleaseFlag(), "--packagePath", vsix],
      { stdio: "inherit" },
    );
    if (result.status === 0) return true;
    console.error(`::warning::publish attempt ${attempt}/${MAX_ATTEMPTS} failed for ${vsix}`);
    if (attempt < MAX_ATTEMPTS) await backoff(attempt);
  }
  return false;
}

// The Marketplace forbids a SemVer prerelease suffix in the version field, so
// the VSIX carries the core MAJOR.MINOR.PATCH (see stamp-release-version.mjs)
// and a hyphenated tag conveys prerelease status through the flag instead.
function prereleaseFlag() {
  const refName = process.env.GITHUB_REF_NAME ?? "";
  return refName.includes("-") ? ["--pre-release"] : [];
}

async function backoff(attempt) {
  const configured = Number(process.env.DESLOP_PUBLISH_BACKOFF_SECONDS);
  const seconds = Number.isFinite(configured) && configured >= 0 ? configured : 20;
  await new Promise((resolve) => setTimeout(resolve, attempt * seconds * 1000));
}

// Completeness is asserted before anything publishes: finding fewer VSIXes
// than the build matrix produced means an artifact never uploaded, and
// publishing the remainder would manufacture exactly the partial release this
// script exists to prevent.
function discoverVsixes(expected) {
  const entries = (() => {
    try {
      return readdirSync("artifacts", { recursive: true });
    } catch {
      return [];
    }
  })();
  const vsixes = entries
    .filter((entry) => entry.endsWith(".vsix"))
    .map((entry) => join("artifacts", entry))
    .sort();
  if (vsixes.length !== expected) {
    console.error(
      `::error::expected ${expected} VSIX artifacts, found ${vsixes.length}: ${vsixes.join(", ") || "none"}`,
    );
    process.exit(1);
  }
  return vsixes;
}

function requireToken(tool) {
  if (!process.env[tool.tokenVariable]) {
    console.error(`::error::${tool.tokenVariable} is not set; cannot publish to ${tool.registry}`);
    process.exit(1);
  }
}

function parseArguments(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    values.set(argv[index], argv[index + 1]);
  }
  const tool = TOOLS[values.get("--tool")];
  const expected = Number(values.get("--expected"));
  if (tool === undefined || !Number.isInteger(expected) || expected < 1) {
    console.error("usage: node scripts/publish-vsixes.mjs --tool <vsce|ovsx> --expected <count>");
    process.exit(1);
  }
  return { tool, expected };
}
