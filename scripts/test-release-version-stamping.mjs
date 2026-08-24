// Tests for first-class release/test version stamping.

import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { runContractSuite } from "./contract-suite.mjs";
import { readActionPins } from "./stamp-release-version.mjs";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const stamper = join(repoRoot, "scripts/stamp-release-version.mjs");
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
  stamperStampsEveryReadmeActionPin,
  stamperStampsPinsQuotedInProse,
  stamperRejectsInvalidVersion,
];

runContractSuite(tests, "release version stamping", "deslop-version-stamp-");

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
// so the advertised snippet failed outright. Every published surface that shows
// a copy-pasteable pin — the README and both locales of the Action doc page —
// must move with the tag, or a reader copies a stale version.
function stamperStampsEveryReadmeActionPin(work) {
  copyStampInputs(work);
  const result = spawnSync("node", [stamper, version, "--root", work], { encoding: "utf8" });
  if (result.status !== 0) throw new Error(`stamper failed: ${result.stderr}`);

  const total = actionPinDocs.reduce(
    (count, doc) => count + assertDocPinsStamped(doc, read(repoRoot, doc), read(work, doc)),
    0,
  );
  if (total < 7) throw new Error(`only ${total} action pins stamped across the docs, expected 7`);

  // The SHA-pinned example documents the case where the ref carries no version,
  // so `version:` is required. Rewriting it to a tag would destroy the very
  // thing it illustrates — the stamper must leave a non-`@v` ref alone.
  for (const doc of actionPinDocs.slice(1)) {
    assertIncludes(read(work, doc), "uses: Nimblesite/Deslop@8f4c1e2a9b7d3f6a5c8e1b4d7a0f3c6e9b2d5a8f");
  }
}

// Stamping must move the version and nothing else, anywhere in the file. The
// indentation is deliberately NOT asserted to one fixed depth: the README shows
// a bare two-step fragment while the doc pages show a whole workflow, so the
// pins legitimately sit at different depths. What has to hold is that whatever
// depth a pin had, it still has — these snippets are pasted straight into
// workflow YAML, where a shifted prefix is a syntax error.
function assertDocPinsStamped(doc, before, after) {
  const beforeLines = before.split("\n");
  const afterLines = after.split("\n");
  if (afterLines.length !== beforeLines.length) {
    throw new Error(`${doc}: stamping changed the line count`);
  }
  const pins = beforeLines.filter((line, index) =>
    assertLineStamped(doc, index + 1, line, afterLines[index]),
  );
  if (pins.length === 0) throw new Error(`${doc} has no action pin to stamp`);
  return pins.length;
}

// Returns true when the line carried a pin, so the caller can count them.
//
// The version comparison reuses the stamper's own parser rather than a second
// copy of its rule, which keeps the two from drifting — but it means a
// mis-parsed pin cannot be caught by parsing, because the check would repeat the
// mistake and agree with it. Substituting the old token back in has the same
// blind spot: whatever the parser wrongly ate is restored along with it. So the
// corruption is caught structurally instead. A version token holds only
// characters SemVer permits, so stamping one can never change how many backticks
// a line has — and swallowing the backtick that closes a prose-quoted pin is
// exactly what dropped it, unterminating the code span. [ACTION-VERSION]
function assertLineStamped(doc, lineNumber, before, after) {
  if (!before.includes(actionPinPrefix)) {
    if (after !== before) {
      throw new Error(`${doc}:${lineNumber} changed but carries no action pin: ${after}`);
    }
    return false;
  }
  const [wanted] = readActionPins(before);
  const [got] = readActionPins(after);
  if (got !== version) {
    throw new Error(`${doc}:${lineNumber} pins ${got}, expected ${version}`);
  }
  if (after.split("`").length !== before.split("`").length) {
    throw new Error(`${doc}:${lineNumber} stamping changed the inline code spans: ${after}`);
  }
  const reversed = after.split(actionPinPrefix + got).join(actionPinPrefix + wanted);
  if (reversed !== before) {
    throw new Error(`${doc}:${lineNumber} stamping moved more than the version: ${after}`);
  }
  return true;
}

// A pin does not only appear as a bare YAML line. The Action doc page states the
// derivation rule in prose — "so `uses: Nimblesite/Deslop@v0.27.0` installs
// `deslop` 0.27.0" — where the pin is closed by a backtick, not a space. Reading
// the version as everything up to the first space swallows that backtick into
// the version token and drops it on stamping, which unterminates the inline code
// span and leaves the rest of the sentence rendering as code, still quoting the
// old version beside the new pin. Assert the reader-visible outcome: the stale
// version is gone from the line and every backtick survives. [ACTION-VERSION]
function stamperStampsPinsQuotedInProse(work) {
  copyStampInputs(work);
  const before = read(repoRoot, "site/src/docs/github-action.md").split("\n");
  const result = spawnSync("node", [stamper, version, "--root", work], { encoding: "utf8" });
  if (result.status !== 0) throw new Error(`stamper failed: ${result.stderr}`);
  const after = read(work, "site/src/docs/github-action.md").split("\n");

  const prose = before.flatMap((line, index) =>
    line.includes(actionPinPrefix) && line.includes("`") ? [index] : [],
  );
  if (prose.length === 0) throw new Error("no prose-quoted pin in the Action doc page to exercise");

  for (const index of prose) {
    const stamped = after[index];
    const backticks = (text) => [...text].filter((character) => character === "`").length;
    if (backticks(stamped) !== backticks(before[index])) {
      throw new Error(
        `site/src/docs/github-action.md:${index + 1} lost a backtick to stamping, breaking the code span: ${stamped}`,
      );
    }
    if (!stamped.includes(`${actionPinPrefix}${version}\``)) {
      throw new Error(
        `site/src/docs/github-action.md:${index + 1} did not stamp the prose-quoted pin cleanly: ${stamped}`,
      );
    }
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
