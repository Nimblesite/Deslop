// Tests for first-class release/test version stamping.

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { runContractSuite } from "../lib/contract-harness.mjs";
import { repoRoot } from "../lib/repo-root.mjs";
import { copyFileAt } from "../lib/write-file.mjs";

const stamper = join(repoRoot, "scripts/release/stamp-release-version.mjs");
const version = "9.8.7-test.1";
// Mirrors stamp-release-version.mjs. The doc pages come after README.md so the
// SHA-example assertion can address them as `slice(1)` — only they carry it.
const actionPinPrefix = "uses: Nimblesite/Deslop@v";
const actionPinDocs = [
  "README.md",
  "site/src/docs/github-action.md",
  "site/src/zh/docs/github-action.md",
];

const tests = [
  sourceProjectsUseVersionPlaceholder,
  stamperSetsEveryProjectVersion,
  stamperStampsGeneratedVsixManifest,
  stamperStampsEveryWorkspaceCrateInLock,
  stamperLeavesDocumentedPinsUntouched,
  stamperRejectsInvalidVersion,
];

runContractSuite(tests, "release version stamping", "deslop-version-stamp-");

function sourceProjectsUseVersionPlaceholder() {
  const placeholder = "0.0.0-dev";
  assertCargoVersion(repoRoot, placeholder, ["deslop", "deslop-mcp"]);
  assertJsonVersion(repoRoot, "shipwright.json", placeholder);
  assertJsonVersion(repoRoot, "clients/vscode/package.json", placeholder);
  assertJsonVersion(repoRoot, "clients/vscode/package-lock.json", placeholder);
  assertJsonVersion(repoRoot, "clients/vscode/webview-ui/package.json", placeholder);
  assertJsonVersion(repoRoot, "clients/vscode/webview-ui/package-lock.json", placeholder);
  assertJsonVersion(repoRoot, "site/package.json", placeholder);
  assertJsonVersion(repoRoot, "site/package-lock.json", placeholder);
}

function stamperSetsEveryProjectVersion(work) {
  copyStampInputs(work);
  runStamper(work);

  assertCargoVersion(work, version, ["deslop", "deslop-lsp"]);
  assertJsonVersion(work, "shipwright.json", version);
  // The VSIX package version is the Marketplace-legal core MAJOR.MINOR.PATCH;
  // every other project keeps the full version including the prerelease suffix.
  const marketplace = version.split(/[-+]/, 1)[0];
  if (marketplace === version) {
    throw new Error("test version must carry a prerelease suffix to exercise marketplace stamping");
  }
  assertJsonVersion(work, "clients/vscode/package.json", marketplace);
  assertJsonVersion(work, "clients/vscode/package-lock.json", marketplace);
  assertJsonVersion(work, "clients/vscode/webview-ui/package.json", version);
  assertJsonVersion(work, "clients/vscode/webview-ui/package-lock.json", version);
  assertJsonVersion(work, "site/package.json", version);
  assertJsonVersion(work, "site/package-lock.json", version);
}

function stamperStampsGeneratedVsixManifest(work) {
  copyStampInputs(work);
  const stagedManifest = "clients/vscode/shipwright.json";
  const dest = copyFileAt(join(work, "shipwright.json"), join(work, stagedManifest));

  runStamper(work);

  assertJsonVersion(work, stagedManifest, version);
}

// Every workspace/path crate (a Cargo.lock `[[package]]` with no `source =`
// line, i.e. not from a registry/git) must be stamped. A hardcoded crate list
// silently skips any crate it omits, leaving Cargo.lock out of sync with the
// stamped Cargo.toml so the release's `cargo build --locked` fails — the
// regression a new workspace crate (deslop-test-support) introduced.
function stamperStampsEveryWorkspaceCrateInLock(work) {
  copyStampInputs(work);
  runStamper(work);

  const lock = read(work, "Cargo.lock");
  let workspaceCrates = 0;
  for (const block of lock.split("[[package]]").slice(1)) {
    const name = block.match(/\nname = "([^"]+)"/)?.[1];
    if (!name || /\nsource = /.test(block)) continue;
    workspaceCrates++;
    const lockVersion = block.match(/\nversion = "([^"]+)"/)?.[1];
    if (lockVersion !== version) {
      throw new Error(
        `workspace crate ${name} left at ${lockVersion} in Cargo.lock, expected ${version}`,
      );
    }
  }
  if (workspaceCrates === 0) throw new Error("Cargo.lock had no workspace crates to stamp");
}

// The inverse of what this once asserted, and the reason the reversal is safe.
//
// Stamping rewrites the runner's checkout, never the commit, so a stamped pin
// never reaches the tag a consumer resolves — and the tag's README is the body
// of the Marketplace listing. Stamping the pins therefore could not keep them
// true: whatever version was *committed* is what every visitor copied, which is
// how v0.30.0 shipped a listing advertising `@v0.27.0`. The docs now commit no
// version at all, so a stamper that touched them would reintroduce exactly one
// failure mode — writing a real version into a file that ships as-is, where it
// would go stale at the very next release with nothing left to catch it.
//
// Byte equality is deliberately stricter than the "only the version moved"
// check it replaces: that one had to reason about line counts, backtick
// balance and token substitution to bound the blast radius of a rewrite. With
// no rewrite at all, an untouched file proves every one of those properties at
// once, and proves them for the whole file rather than the pin lines.
// [ACTION-VERSION]
function stamperLeavesDocumentedPinsUntouched(work) {
  copyStampInputs(work);
  runStamper(work);

  for (const doc of actionPinDocs) assertPinSurvived(work, doc);

  // The SHA-pinned example documents the case where the ref carries no version,
  // so `version:` is required. Rewriting it to a tag would destroy the very
  // thing it illustrates — the stamper must leave a non-`@v` ref alone.
  for (const doc of actionPinDocs.slice(1)) {
    assertIncludes(read(work, doc), "uses: Nimblesite/Deslop@8f4c1e2a9b7d3f6a5c8e1b4d7a0f3c6e9b2d5a8f");
  }
}

/** Runs the stamper over the inputs already copied into `work`. */
function runStamper(work) {
  const result = spawnSync("node", [stamper, version, "--root", work], { encoding: "utf8" });
  if (result.status !== 0) throw new Error(`stamper failed: ${result.stderr}`);
}

/** Cargo.toml, and each named workspace crate in Cargo.lock, carry `expected`. */
function assertCargoVersion(root, expected, crates) {
  assertIncludes(read(root, "Cargo.toml"), `version = "${expected}"`);
  for (const crate of crates) {
    assertIncludes(read(root, "Cargo.lock"), `name = "${crate}"\nversion = "${expected}"`);
  }
}

/** A documented action pin must ship exactly as it was committed. */
function assertPinSurvived(work, doc) {
  const before = read(repoRoot, doc);
  const after = read(work, doc);
  if (after !== before) {
    throw new Error(`${doc}: stamping rewrote a published surface that must ship exactly as committed`);
  }
  if (!after.includes(actionPinPrefix)) throw new Error(`${doc} has no action pin left to protect`);
  if (after.includes(`${actionPinPrefix}${version}`)) {
    throw new Error(`${doc}: the stamped version reached a committed pin`);
  }
}

function stamperRejectsInvalidVersion(work) {
  copyStampInputs(work);
  const result = spawnSync("node", [stamper, "v9.8", "--root", work], { encoding: "utf8" });
  if (result.status === 0) throw new Error("stamper accepted an invalid version");
  assertIncludes(`${result.stdout}\n${result.stderr}`, "semantic version");
}

function copyStampInputs(work) {
  for (const file of [
    "Cargo.toml",
    "Cargo.lock",
    "shipwright.json",
    ...actionPinDocs,
    "clients/vscode/package.json",
    "clients/vscode/package-lock.json",
    "clients/vscode/webview-ui/package.json",
    "clients/vscode/webview-ui/package-lock.json",
    "site/package.json",
    "site/package-lock.json",
  ]) {
    copyFileAt(join(repoRoot, file), join(work, file));
  }
}

function assertJsonVersion(root, file, expected) {
  const value = JSON.parse(read(root, file));
  if (value.version !== expected && value.product?.version !== expected) {
    throw new Error(`${file} was not stamped to ${expected}`);
  }
  for (const component of value.components ?? []) {
    if (component.expectedVersion !== expected) {
      throw new Error(`${file} has stale ${component.id} expectedVersion`);
    }
  }
  if (value.packages?.[""]?.version && value.packages[""].version !== expected) {
    throw new Error(`${file} root package lock version was not stamped`);
  }
}

function read(root, file) {
  return readFileSync(join(root, file), "utf8");
}

function assertIncludes(value, expected) {
  if (!value.includes(expected)) throw new Error(`missing ${expected}`);
}
