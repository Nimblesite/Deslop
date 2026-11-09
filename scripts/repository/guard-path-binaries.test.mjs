// PATH-guard contract. [DEPLOY-EXTENSION-BUNDLED-TESTS] [DEPLOY-EXTERNAL-MCP-CONSUMER]
//
// Tests must run the extension's bundled binaries, so a `deslop` on PATH must
// never answer for them. The scrub that used to enforce this uninstalled the
// Homebrew copy and deleted the curl installer's, which every agent on the
// machine was using (#611). The guard instead puts refusing stubs first on
// PATH and leaves every install alone. These tests drive the real script
// against a fixture PATH and HOME; nothing outside the fixture is touched.
// Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { chmodSync, existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, symlinkSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { repoRoot } from "../lib/repo-root.mjs";
import { makefileLines, recipeBlocks, variableValue } from "../lib/makefile.mjs";
import { posixShell, shellPath } from "../lib/posix-shell.mjs";

/** The guard under test, the target that runs it, and the prefix every test recipe carries. */
const SCRIPT = resolve(repoRoot, "scripts/repository/guard-path-binaries.sh");
const GUARD_TARGET = "_guard-path-binaries";
const SCRIPT_INVOCATION = "bash scripts/repository/guard-path-binaries.sh";
const GUARDED_PATH = "$(GUARDED_PATH)";
const GUARD_DIR_VARIABLE = "PATH_GUARD_DIR";
const GUARDED_PATH_VARIABLE = "GUARDED_PATH";

/** Every make target that runs a test suite, and so must run behind the guard. */
const TEST_TARGETS = ["test", "_test-shard", "_coverage-run", "_vsix-test", "_vsix-test-ollama", "_vsix-coverage"];

/** Commands that remove a user's install. The guard and the Makefile run none of them. */
const DESTRUCTIVE_COMMANDS = ["brew uninstall", "cargo uninstall", "scoop uninstall", "_delete-path-binaries"];

/** Absolute bash: the 3.2 build macOS ships, or Git Bash on Windows. */
const BASH = posixShell();
const SCRIPT_FOR_SHELL = shellPath(SCRIPT);

/** System directories supplying `mkdir`, `chmod` and `cat`; never a place Deslop is installed. */
const SYSTEM_PATH = "/usr/bin:/bin";

/** Names the extension bundles, which no test may resolve from PATH. */
const BINARY_NAMES = ["deslop", "deslop-lsp", "deslop-mcp"];

/** What a stub says, so a refused call in a test log explains itself. */
const REFUSAL_RULE = "[DEPLOY-EXTENSION-BUNDLED-TESTS]";

/** Exit statuses: success, a stub's refusal or a failed check, and a usage error. */
const OK = 0;
const REFUSED = 1;
const USAGE = 2;

const EXECUTABLE_MODE = 0o755;
const INSTALLED_BODY = "#!/bin/sh\necho installed-copy\n";

/** True when the name exists as a directory entry, dangling links included. */
function entryExists(path) {
  return lstatSync(path, { throwIfNoEntry: false }) !== undefined;
}

/** A throwaway root holding the guard directory, a HOME, and a brew call log. */
function fixture() {
  const root = mkdtempSync(join(tmpdir(), "deslop-guard-"));
  const homeDir = join(root, "home");
  mkdirSync(homeDir);
  const brewLog = join(root, "brew-calls.log");
  writeFileSync(brewLog, "");
  return { root, homeDir, brewLog, guardDir: join(root, "path-guard") };
}

/** Writes an executable `name` into `directory` with `body`. */
function stageExecutable(directory, name, body = INSTALLED_BODY) {
  mkdirSync(directory, { recursive: true });
  const path = join(directory, name);
  writeFileSync(path, body);
  chmodSync(path, EXECUTABLE_MODE);
  return path;
}

/** A Homebrew prefix whose `brew` logs every call, and the curl installer's `~/.local/bin`, each holding all three binaries. */
function stageReleaseLockedInstalls(dirs) {
  const brewPrefix = join(dirs.root, "homebrew", "bin");
  const curlBin = join(dirs.homeDir, ".local", "bin");
  stageExecutable(brewPrefix, "brew", `#!/bin/sh\necho "$*" >> "${shellPath(dirs.brewLog)}"\n`);
  const installed = [brewPrefix, curlBin].flatMap((directory) =>
    BINARY_NAMES.map((name) => stageExecutable(directory, name)));
  return { installed, brewPrefix, curlBin };
}

/** PATH for the shell: `entries` in order, then the system tools. */
function shellPathOf(entries) {
  return [...entries.map((entry) => shellPath(entry)), SYSTEM_PATH].join(":");
}

/** Runs the guard over `dirs.guardDir` with `pathEntries` on PATH. */
function runGuard(dirs, pathEntries, args = [dirs.guardDir]) {
  return spawnSync(BASH, [SCRIPT_FOR_SHELL, ...args.map((arg) => shellPath(arg))], {
    encoding: "utf8",
    env: { PATH: shellPathOf(pathEntries), HOME: shellPath(dirs.homeDir) },
  });
}

/** Runs `script` in bash with `pathEntries` on PATH. */
function inShell(dirs, pathEntries, script) {
  return spawnSync(BASH, ["-c", script], {
    encoding: "utf8",
    env: { PATH: shellPathOf(pathEntries), HOME: shellPath(dirs.homeDir) },
  });
}

/** Asserts every release-locked binary is still there, byte for byte. */
function assertInstallsSurvive(installed, when) {
  for (const binary of installed) {
    assert.equal(entryExists(binary), true, `${when}: ${binary} must survive the guard`);
    assert.equal(readFileSync(binary, "utf8"), INSTALLED_BODY, `${when}: ${binary} must be unchanged`);
  }
}

test("every Deslop name resolves to a guard stub that refuses to run and says why", () => {
  const dirs = fixture();
  const guarded = runGuard(dirs, [dirs.guardDir]);
  assert.equal(guarded.status, OK, `guard failed: ${guarded.stdout}${guarded.stderr}`);
  assert.ok(guarded.stdout.includes(`deslop deslop-lsp deslop-mcp resolve to ${shellPath(dirs.guardDir)}`), guarded.stdout);
  for (const name of BINARY_NAMES) {
    const resolved = inShell(dirs, [dirs.guardDir], `command -v ${name}`);
    assert.equal(resolved.stdout.trim(), shellPath(join(dirs.guardDir, name)), `${name} resolves to its stub`);
    const called = inShell(dirs, [dirs.guardDir], `${name} --version`);
    assert.equal(called.status, REFUSED, `${name} on PATH must refuse to run`);
    assert.equal(called.stdout, "", `${name} must not print a version as if it ran`);
    assert.ok(called.stderr.startsWith(`${name}: refused`), `${name} names itself: ${called.stderr}`);
    assert.ok(called.stderr.includes(REFUSAL_RULE), `${name} cites the rule: ${called.stderr}`);
    assert.ok(existsSync(join(dirs.guardDir, `${name}.cmd`)), `${name}.cmd guards cmd.exe and PowerShell callers`);
  }
  assert.deepEqual(readdirSync(dirs.guardDir).sort(), BINARY_NAMES.flatMap((name) => [name, `${name}.cmd`]).sort());
});

test("[#611] the guard never uninstalls or deletes a Homebrew or curl-installer deslop, and still shadows both", () => {
  const dirs = fixture();
  const { installed, brewPrefix, curlBin } = stageReleaseLockedInstalls(dirs);
  for (const run of ["first run", "second run"]) {
    const guarded = runGuard(dirs, [dirs.guardDir, brewPrefix, curlBin]);
    assert.equal(guarded.status, OK, `${run}: guard failed: ${guarded.stdout}${guarded.stderr}`);
    assert.equal(readFileSync(dirs.brewLog, "utf8"), "", `${run}: the guard must never call brew`);
    assert.ok(!guarded.stdout.includes("deleting"), `${run}: the guard deleted something: ${guarded.stdout}`);
    assertInstallsSurvive(installed, run);
    for (const name of BINARY_NAMES) {
      const resolved = inShell(dirs, [dirs.guardDir, brewPrefix, curlBin], `command -v ${name}`);
      assert.equal(resolved.stdout.trim(), shellPath(join(dirs.guardDir, name)), `${run}: ${name} hits the guard first`);
    }
  }
  const brewCopy = inShell(dirs, [brewPrefix], "deslop-mcp");
  assert.equal(brewCopy.stdout.trim(), "installed-copy", "the agent's Homebrew deslop-mcp still runs outside the guard");
});

test("the guard fails closed, naming each culprit, when a Deslop binary resolves ahead of it, and deletes nothing", () => {
  const dirs = fixture();
  const { installed, brewPrefix, curlBin } = stageReleaseLockedInstalls(dirs);
  for (const ahead of [brewPrefix, curlBin]) {
    const guarded = runGuard(dirs, [ahead, dirs.guardDir]);
    assert.equal(guarded.status, REFUSED, `a PATH copy ahead of the guard must fail the gate: ${guarded.stdout}`);
    for (const name of BINARY_NAMES) {
      assert.ok(guarded.stdout.includes(`FAIL: ${name} resolves to '${shellPath(join(ahead, name))}'`), guarded.stdout);
    }
    assert.ok(guarded.stdout.includes("GUARDED_PATH"), "the failure names the fix");
    assertInstallsSurvive(installed, `copy ahead in ${ahead}`);
  }
  assert.equal(readFileSync(dirs.brewLog, "utf8"), "", "a failing check must not call brew either");
});

test("the guard fails closed when it is not on PATH at all", () => {
  const dirs = fixture();
  const guarded = runGuard(dirs, []);
  assert.equal(guarded.status, REFUSED);
  for (const name of BINARY_NAMES) assert.ok(guarded.stdout.includes(`FAIL: ${name} resolves to 'nothing'`), guarded.stdout);
});

test("[#474] a dangling leftover ahead of the guard cannot run, so the guard holds and leaves it alone", () => {
  const dirs = fixture();
  const leftovers = join(dirs.root, "leftovers");
  mkdirSync(leftovers);
  const link = join(leftovers, "deslop-mcp");
  symlinkSync(join(dirs.root, "deslop-live-0.15.0-darwin-arm64", "bin", "deslop-mcp"), link);
  const guarded = runGuard(dirs, [leftovers, dirs.guardDir]);
  assert.equal(guarded.status, OK, `a dangling link executes nothing: ${guarded.stdout}`);
  assert.equal(entryExists(link), true, "the guard deletes nothing, not even a dangling link");
  const called = inShell(dirs, [leftovers, dirs.guardDir], "deslop-mcp");
  assert.equal(called.status, REFUSED, "the call falls through the dead link to the guard");
});

test("missing or extra arguments are rejected before anything is written", () => {
  const dirs = fixture();
  for (const args of [[], ["", ""], [dirs.guardDir, dirs.root], [""]]) {
    const rejected = runGuard(dirs, [dirs.guardDir], args);
    assert.equal(rejected.status, USAGE, `args ${JSON.stringify(args)} must be a usage error`);
    assert.ok(rejected.stderr.includes("<guard-dir>"), "usage names the one argument");
  }
  assert.equal(entryExists(dirs.guardDir), false, "a rejected call writes no guard");
});

test("the Makefile runs every test suite behind the guard", () => {
  const [guard, ...others] = recipeBlocks(GUARD_TARGET);
  assert.ok(guard, `the Makefile declares ${GUARD_TARGET}`);
  assert.equal(others.length, 0, `exactly one ${GUARD_TARGET} recipe`);
  assert.ok(guard.body.includes(`${GUARDED_PATH} ${SCRIPT_INVOCATION} "$(${GUARD_DIR_VARIABLE})"`), guard.body);
  assert.ok(variableValue(GUARD_DIR_VARIABLE).length > 0, `${GUARD_DIR_VARIABLE} is declared`);
  assert.ok(variableValue(GUARDED_PATH_VARIABLE).includes(`$(${GUARD_DIR_VARIABLE}):$$PATH`), "GUARDED_PATH puts the guard first");
  for (const target of TEST_TARGETS) {
    const blocks = recipeBlocks(target);
    assert.ok(blocks.length > 0, `${target} is declared`);
    assert.ok(blocks.some((block) => block.header.includes(GUARD_TARGET)), `${target} depends on ${GUARD_TARGET}`);
    assert.ok(blocks.some((block) => block.body.includes(GUARDED_PATH)), `${target} runs its suite under ${GUARDED_PATH}`);
  }
});

test("[#611] neither the guard nor any Makefile recipe removes a user's install", () => {
  const sources = [["Makefile", makefileLines().join("\n")], [SCRIPT, readFileSync(SCRIPT, "utf8")]];
  for (const [name, text] of sources) {
    for (const command of DESTRUCTIVE_COMMANDS) {
      assert.ok(!text.includes(command), `${name} must not run ${command}`);
    }
  }
});
