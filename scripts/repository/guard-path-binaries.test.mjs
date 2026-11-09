// PATH-scrub contract. [DEPLOY-EXTERNAL-MCP-CONSUMER]
//
// Issue #474: `_delete-path-binaries` detected leaked binaries with
// `command -v`, which reports only a resolvable executable. A
// `~/.local/bin/deslop-mcp` symlink pointing at a deleted VSIX bundle is not
// resolvable, so the scrub saw nothing, removed nothing, and exited 0 — the
// gate reported success while the shadowing name it exists to remove was
// still on disk. These tests drive the real script against a fixture PATH.
//
// The make target is never run here: it uninstalls the developer's real
// binaries. Every spawn below replaces the environment wholesale, so the
// script can only ever see the fixture directory plus the system tools it
// needs, and `assertSystemPathIsClean` fails the test rather than letting a
// real install fall inside the blast radius. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { chmodSync, lstatSync, mkdirSync, mkdtempSync, symlinkSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { repoRoot } from "../lib/repo-root.mjs";
import { recipeBlocks } from "../lib/makefile.mjs";
import { hostPath, posixShell, shellPath } from "../lib/posix-shell.mjs";

/** The scrub under test, and the target that must delegate to it. */
const SCRIPT = resolve(repoRoot, "scripts/repository/scrub-path-binaries.sh");
const SCRUB_TARGET = "_delete-path-binaries";
const SCRIPT_INVOCATION = "bash scripts/repository/scrub-path-binaries.sh";

/** Absolute bash: the 3.2 build macOS ships, or Git Bash on Windows. A bare
 * name would find WSL's bash on Windows, which sees a different filesystem. */
const BASH = posixShell();

/** The scrub as the shell must name it — bash opens the script by this path. */
const SCRIPT_FOR_SHELL = shellPath(SCRIPT);

/** Query mode: print what shadows, delete nothing, fail if anything does. */
const LIST_FLAG = "--list";

/** System directories supplying `rm`; never a place Deslop is installed. */
const SYSTEM_PATH = "/usr/bin:/bin";

/** Names that must never resolve anywhere but the unpacked VSIX. */
const BINARY_NAMES = ["deslop", "deslop-lsp", "deslop-mcp"];

/** Executable mode for a staged fixture binary, and a non-executable one. */
const EXECUTABLE_MODE = 0o755;
const READ_ONLY_MODE = 0o644;

/** True when the name exists as a directory entry, dangling links included. */
function entryExists(path) {
  return lstatSync(path, { throwIfNoEntry: false }) !== undefined;
}

/** True when the name resolves to something real (a dangling link does not). */
function targetExists(path) {
  const stats = lstatSync(path, { throwIfNoEntry: false });
  if (stats === undefined) return false;
  return !stats.isSymbolicLink() || entryExistsThroughLink(path);
}

function entryExistsThroughLink(path) {
  return spawnSync(BASH, ["-c", `[ -e "${path}" ]`], { env: { PATH: SYSTEM_PATH } }).status === 0;
}

/** Refuses to run the scrub if a real Deslop install sits in the system path. */
function assertSystemPathIsClean() {
  for (const directory of SYSTEM_PATH.split(":")) {
    for (const name of BINARY_NAMES) {
      // The system directories are named the way the *shell* names them, so
      // this must open them the way the host does. Left untranslated on
      // Windows it looks under the drive root, finds nothing, and passes
      // without having examined the shell's real /usr/bin at all.
      const path = join(hostPath(directory), name);
      assert.equal(entryExists(path), false, `${path} exists — refusing to run the scrub over a real install`);
    }
  }
}

/** A throwaway PATH directory and HOME, isolated from the developer's own. */
function fixture() {
  const root = mkdtempSync(join(tmpdir(), "deslop-scrub-"));
  const binDir = join(root, "bin");
  const homeDir = join(root, "home");
  mkdirSync(binDir);
  mkdirSync(homeDir);
  return { root, binDir, homeDir };
}

/** Runs the scrub with nothing but the fixture directory and the system tools. */
function runScrub({ binDir, homeDir }, args = [], { withSystemTools = true } = {}) {
  assertSystemPathIsClean();
  const fixtureEntry = shellPath(binDir);
  const path = withSystemTools ? `${fixtureEntry}:${SYSTEM_PATH}` : fixtureEntry;
  return spawnSync(BASH, [SCRIPT_FOR_SHELL, ...args], {
    encoding: "utf8",
    env: { PATH: path, HOME: shellPath(homeDir) },
  });
}

/** What `command -v` — the detection #474 relied on — reports for a name. */
function commandVee({ binDir, homeDir }, name) {
  const result = spawnSync(BASH, ["-c", `command -v ${name} || true`], {
    encoding: "utf8",
    env: { PATH: `${shellPath(binDir)}:${SYSTEM_PATH}`, HOME: shellPath(homeDir) },
  });
  return result.stdout.trim();
}

/** How the scrub prints `path` — it names what it found in the shell's own
 * spelling, which is the only spelling the operator can paste back. */
function asPrinted(path) {
  return shellPath(path);
}

/** Stages a symlink to a bundle path that was deleted — the #474 leftover. */
function stageDanglingLink(fixtureDirs, name) {
  const link = join(fixtureDirs.binDir, name);
  symlinkSync(join(fixtureDirs.root, "deslop-live-0.15.0-darwin-arm64", "bin", name), link);
  return link;
}

/** Stages a real file on the fixture PATH. */
function stageFile(fixtureDirs, name, mode) {
  const path = join(fixtureDirs.binDir, name);
  writeFileSync(path, "#!/bin/sh\nexit 0\n");
  chmodSync(path, mode);
  return path;
}

test("[#474] a dangling symlink is invisible to command -v — the detection the scrub relied on", () => {
  const dirs = fixture();
  const link = stageDanglingLink(dirs, "deslop-mcp");
  assert.equal(entryExists(link), true, "the leftover name is on PATH");
  assert.equal(targetExists(link), false, "and it points at a bundle that no longer exists");
  assert.equal(commandVee(dirs, "deslop-mcp"), "", "command -v reports nothing — this is why the scrub used to pass");
});

test("[#474] --list reports the dangling symlink and exits non-zero", () => {
  const dirs = fixture();
  const link = stageDanglingLink(dirs, "deslop-mcp");
  const listed = runScrub(dirs, [LIST_FLAG]);
  assert.equal(listed.stdout.trim(), asPrinted(link), "the shadowing path must be named exactly");
  assert.equal(listed.status, 1, "a shadowed PATH must fail closed, not report clean");
});

test("[#474] the scrub deletes the dangling symlink and only then reports success", () => {
  const dirs = fixture();
  const link = stageDanglingLink(dirs, "deslop-mcp");
  const scrubbed = runScrub(dirs);
  assert.equal(scrubbed.status, 0, `scrub failed: ${scrubbed.stdout}${scrubbed.stderr}`);
  assert.ok(scrubbed.stdout.includes(`deleting ${asPrinted(link)}`), "the scrub must say what it removed");
  assert.ok(scrubbed.stdout.includes("PATH is clear of deslop deslop-lsp deslop-mcp"), "and confirm the result");
  assert.equal(entryExists(link), false, "the leftover must be gone, not merely unreported");
  assert.equal(runScrub(dirs, [LIST_FLAG]).status, 0, "a re-check must now find nothing");
});

test("[#474] the scrub fails closed when a shadowing name survives deletion", () => {
  const dirs = fixture();
  const link = stageDanglingLink(dirs, "deslop-mcp");
  // Without the system directories the script cannot reach `rm`, so deletion
  // cannot succeed. The contract is that this exits non-zero and names the
  // survivor — never that it shrugs and reports a clean PATH.
  const scrubbed = runScrub(dirs, [], { withSystemTools: false });
  assert.equal(scrubbed.status, 1, "a scrub that cannot remove a shadowing name must fail");
  assert.ok(scrubbed.stdout.includes("FAIL: these PATH entries still shadow"), "and explain what is wrong");
  assert.ok(scrubbed.stdout.includes(asPrinted(link)), "naming every survivor");
  assert.equal(entryExists(link), true, "the survivor is genuinely still on PATH");
});

test("the scrub removes an executable and a non-executable leftover alike", () => {
  const dirs = fixture();
  const executable = stageFile(dirs, "deslop", EXECUTABLE_MODE);
  const inert = stageFile(dirs, "deslop-lsp", READ_ONLY_MODE);
  const listed = runScrub(dirs, [LIST_FLAG]);
  assert.deepEqual(listed.stdout.trim().split("\n").sort(), [executable, inert].map(asPrinted).sort());
  assert.equal(listed.status, 1);
  assert.equal(commandVee(dirs, "deslop"), asPrinted(executable), "an executable copy is exactly what command -v did catch");
  const scrubbed = runScrub(dirs);
  assert.equal(scrubbed.status, 0, `scrub failed: ${scrubbed.stdout}${scrubbed.stderr}`);
  assert.equal(entryExists(executable), false);
  assert.equal(entryExists(inert), false);
});

test("a directory named after a binary is left alone — it can never shadow an executable", () => {
  const dirs = fixture();
  const directory = join(dirs.binDir, "deslop");
  mkdirSync(directory);
  const linkToDirectory = join(dirs.binDir, "deslop-lsp");
  symlinkSync(directory, linkToDirectory);
  assert.equal(runScrub(dirs, [LIST_FLAG]).stdout, "", "neither entry can be executed, so neither is a leak");
  assert.equal(runScrub(dirs, [LIST_FLAG]).status, 0);
  const scrubbed = runScrub(dirs);
  assert.equal(scrubbed.status, 0, `scrub failed: ${scrubbed.stdout}${scrubbed.stderr}`);
  assert.equal(entryExists(directory), true, "the scrub must not delete a directory it cannot be shadowed by");
  assert.equal(entryExists(linkToDirectory), true);
});

test("an already-clean PATH scrubs to success and lists nothing", () => {
  const dirs = fixture();
  assert.equal(runScrub(dirs, [LIST_FLAG]).stdout, "");
  assert.equal(runScrub(dirs, [LIST_FLAG]).status, 0);
  const scrubbed = runScrub(dirs);
  assert.equal(scrubbed.status, 0, `scrub failed: ${scrubbed.stdout}${scrubbed.stderr}`);
  assert.ok(scrubbed.stdout.includes("PATH is clear of deslop deslop-lsp deslop-mcp"));
});

test("an unknown argument is rejected rather than silently scrubbing", () => {
  const rejected = runScrub(fixture(), ["--delete-everything"]);
  assert.equal(rejected.status, 2, "an unrecognised mode must not fall through to the destructive path");
  assert.ok(rejected.stderr.includes(LIST_FLAG), "usage must name the only supported flag");
});

test(`[#474] ${SCRUB_TARGET} delegates to the script these tests drive`, () => {
  const blocks = recipeBlocks(SCRUB_TARGET);
  assert.equal(blocks.length, 1, `the Makefile must declare exactly one ${SCRUB_TARGET} recipe; found ${blocks.length}`);
  assert.ok(
    blocks[0].body.includes(SCRIPT_INVOCATION),
    `${SCRUB_TARGET} must run ${SCRIPT_INVOCATION}; an inline copy of the scrub would be untested`,
  );
  assert.ok(
    !blocks[0].body.includes("command -v"),
    "the target must not re-introduce command -v detection: it cannot see a dangling symlink (#474)",
  );
});
