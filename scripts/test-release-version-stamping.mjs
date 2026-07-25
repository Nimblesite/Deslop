// Tests for first-class release/test version stamping.

import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const stamper = join(repoRoot, "scripts/stamp-release-version.mjs");
const version = "9.8.7-test.1";

const tests = [
  sourceProjectsUseVersionPlaceholder,
  stamperSetsEveryProjectVersion,
  stamperStampsGeneratedVsixManifest,
  stamperStampsEveryWorkspaceCrateInLock,
  stamperStampsEveryReadmeActionPin,
  stamperRejectsInvalidVersion,
];

let failed = 0;
for (const test of tests) {
  const work = mkdtempSync(join(tmpdir(), "deslop-version-stamp-"));
  try {
    test(work);
    console.log(`ok ${test.name}`);
  } catch (error) {
    failed++;
    console.error(`not ok ${test.name}`);
    console.error(`  ${error instanceof Error ? error.message : String(error)}`);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

if (failed > 0) {
  console.error(`\n${failed} release version stamping test(s) failed`);
  process.exit(1);
}
console.log(`\n${tests.length} release version stamping tests passed`);

function sourceProjectsUseVersionPlaceholder() {
  const placeholder = "0.0.0-dev";
  assertIncludes(read(repoRoot, "Cargo.toml"), `version = "${placeholder}"`);
  assertIncludes(read(repoRoot, "Cargo.lock"), `name = "deslop"\nversion = "${placeholder}"`);
  assertIncludes(read(repoRoot, "Cargo.lock"), `name = "deslop-mcp"\nversion = "${placeholder}"`);
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
  const result = spawnSync("node", [stamper, version, "--root", work], { encoding: "utf8" });
  if (result.status !== 0) throw new Error(`stamper failed: ${result.stderr}`);

  assertIncludes(read(work, "Cargo.toml"), `version = "${version}"`);
  assertIncludes(read(work, "Cargo.lock"), `name = "deslop"\nversion = "${version}"`);
  assertIncludes(read(work, "Cargo.lock"), `name = "deslop-lsp"\nversion = "${version}"`);
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
  const dest = join(work, stagedManifest);
  mkdirSync(dirname(dest), { recursive: true });
  copyFileSync(join(work, "shipwright.json"), dest);

  const result = spawnSync("node", [stamper, version, "--root", work], { encoding: "utf8" });
  if (result.status !== 0) throw new Error(`stamper failed: ${result.stderr}`);

  assertJsonVersion(work, stagedManifest, version);
}

// Every workspace/path crate (a Cargo.lock `[[package]]` with no `source =`
// line, i.e. not from a registry/git) must be stamped. A hardcoded crate list
// silently skips any crate it omits, leaving Cargo.lock out of sync with the
// stamped Cargo.toml so the release's `cargo build --locked` fails — the
// regression a new workspace crate (deslop-test-support) introduced.
function stamperStampsEveryWorkspaceCrateInLock(work) {
  copyStampInputs(work);
  const result = spawnSync("node", [stamper, version, "--root", work], { encoding: "utf8" });
  if (result.status !== 0) throw new Error(`stamper failed: ${result.stderr}`);

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

// The README is the body of the GitHub Marketplace listing, and the pinned ref
// IS the CLI version the action installs. An unstamped pin shipped v0.26.0's
// listing telling every visitor to use v0.25.0 — a tag that predates action.yml,
// so the advertised snippet failed outright. Every pin must move with the tag.
function stamperStampsEveryReadmeActionPin(work) {
  copyStampInputs(work);
  const result = spawnSync("node", [stamper, version, "--root", work], { encoding: "utf8" });
  if (result.status !== 0) throw new Error(`stamper failed: ${result.stderr}`);

  const readme = read(work, "README.md");
  const pins = readme.split("\n").filter((line) => line.includes("uses: Nimblesite/Deslop@"));
  if (pins.length < 2) {
    throw new Error(`README.md has ${pins.length} action pins, expected the 2 workflow examples`);
  }
  for (const pin of pins) {
    assertIncludes(pin, `uses: Nimblesite/Deslop@v${version}`);
  }
  // The quickstart pin sits inside a `steps:` list and must keep its indentation
  // and its `- ` marker, or the rendered snippet is invalid YAML.
  assertIncludes(readme, `      - uses: Nimblesite/Deslop@v${version}\n        with:`);
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
    "README.md",
    "clients/vscode/package.json",
    "clients/vscode/package-lock.json",
    "clients/vscode/webview-ui/package.json",
    "clients/vscode/webview-ui/package-lock.json",
    "site/package.json",
    "site/package-lock.json",
  ]) {
    const dest = join(work, file);
    mkdirSync(dirname(dest), { recursive: true });
    copyFileSync(join(repoRoot, file), dest);
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
