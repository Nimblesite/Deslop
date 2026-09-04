// Process-scrub contract, and the host-shell contract it depends on.
// [DEPLOY-EXTENSION-BUNDLED-TESTS]
//
// `_kill-deslop-processes` used to inline `pgrep`/`pkill`/`kill -0` directly in
// the Makefile. Git Bash ships none of them and Windows PIDs are not visible to
// `kill -0`, so on Windows the target could not name a single running process —
// and `vsix-rebuild`, which starts with it, could not clear `target/release`
// while a `deslop.exe` from an abandoned test still held its own image open.
// The same recipes were also handed to `powershell.exe`, which cannot parse the
// POSIX shell every recipe in the Makefile is written in.
//
// The kill itself is never run here: it would terminate the developer's real
// editor session. These tests drive `--list`, the detection half, against a
// fixture process the test owns and reaps.

import { test } from "node:test";
import assert from "node:assert/strict";
import { copyFileSync, mkdirSync, mkdtempSync } from "node:fs";
import { spawn, spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { repoRoot } from "../lib/repo-root.mjs";
import { makefileLines, recipeBlocks, variableWords } from "../lib/makefile.mjs";

/** The scrub under test, and the target that must delegate to it. */
const SCRIPT = resolve(repoRoot, "scripts/repository/kill-deslop-processes.sh");
const KILL_TARGET = "_kill-deslop-processes";
const SCRIPT_INVOCATION = "bash scripts/repository/kill-deslop-processes.sh";

/** Query mode: name the PIDs, terminate nothing. */
const LIST_FLAG = "--list";

/** Windows spells executables with an extension; nothing else does. */
const IS_WINDOWS = process.platform === "win32";
const EXECUTABLE_SUFFIX = IS_WINDOWS ? ".exe" : "";

/** A bundled name the scrub must find, and a longer name it must not. */
const BUNDLED_NAME = "deslop-lsp";
const NEIGHBOURING_NAME = "deslop-lsp-helper";

/** Every executable the VSIX bundles — all three block a Windows rebuild. */
const PROCESS_NAMES = ["deslop", "deslop-lsp", "deslop-mcp"];

/** What the fixture process prints once it is unmistakably running. */
const READY = "ready";
const FIXTURE_PROGRAM = `console.log("${READY}"); setInterval(() => {}, 1000);`;

/** Runs the scrub in query mode and returns the PIDs it named. */
function listedPids(args = [LIST_FLAG]) {
  const result = spawnSync("bash", [SCRIPT, ...args], { encoding: "utf8" });
  assert.equal(result.status, 0, `${LIST_FLAG} must never fail: ${result.stdout}${result.stderr}`);
  return result.stdout.split("\n").filter((line) => line.length > 0);
}

/**
 * Starts a long-lived process under `name`, resolving once it has said so on
 * stdout — a process that has written is a process the OS is running, which
 * makes the assertion below independent of any timer.
 */
function startNamed(name) {
  const binDir = mkdtempSync(join(tmpdir(), "deslop-kill-"));
  mkdirSync(binDir, { recursive: true });
  const executable = join(binDir, `${name}${EXECUTABLE_SUFFIX}`);
  copyFileSync(process.execPath, executable);
  const child = spawn(executable, ["-e", FIXTURE_PROGRAM], { stdio: ["ignore", "pipe", "ignore"] });
  return new Promise((resolvePromise, rejectPromise) => {
    child.on("error", rejectPromise);
    child.stdout.on("data", (chunk) => {
      if (String(chunk).includes(READY)) resolvePromise(child);
    });
  });
}

test(`${LIST_FLAG} names a running ${BUNDLED_NAME} by PID`, async (t) => {
  const child = await startNamed(BUNDLED_NAME);
  t.after(() => child.kill());
  assert.ok(
    listedPids().includes(String(child.pid)),
    `a running ${BUNDLED_NAME} (pid ${child.pid}) must be found; a rebuild cannot replace a binary it cannot see`,
  );
});

test("a process whose name merely starts with a bundled name is left alone", async (t) => {
  const child = await startNamed(NEIGHBOURING_NAME);
  t.after(() => child.kill());
  assert.equal(
    listedPids().includes(String(child.pid)),
    false,
    `${NEIGHBOURING_NAME} is not ${BUNDLED_NAME}; matching by prefix would kill a developer's own build`,
  );
});

test("an unknown argument is rejected rather than silently killing processes", () => {
  const rejected = spawnSync("bash", [SCRIPT, "--kill-everything"], { encoding: "utf8" });
  assert.equal(rejected.status, 2, "an unrecognised mode must not fall through to the destructive path");
  assert.ok(rejected.stderr.includes(LIST_FLAG), "usage must name the only supported flag");
});

test(`${KILL_TARGET} delegates to the script these tests drive`, () => {
  const blocks = recipeBlocks(KILL_TARGET);
  assert.equal(blocks.length, 1, `the Makefile must declare exactly one ${KILL_TARGET} recipe; found ${blocks.length}`);
  assert.ok(
    blocks[0].body.includes(SCRIPT_INVOCATION),
    `${KILL_TARGET} must run ${SCRIPT_INVOCATION}; an inline copy would be untested and unportable`,
  );
  for (const absent of ["pgrep", "pkill", "kill -0"]) {
    assert.ok(
      !blocks[0].body.includes(absent),
      `the target must not re-inline ${absent}: Git Bash on Windows has no such thing`,
    );
  }
});

test("every bundled executable is in the scrub's kill list", () => {
  const declaration = readScriptLine("PROCESS_NAMES=");
  for (const name of PROCESS_NAMES) {
    assert.ok(
      declaration.includes(name),
      `${name} must be scrubbed: on Windows a running image cannot be deleted, so cargo clean fails`,
    );
  }
});

/** The single line of the scrub declaring `prefix`. */
function readScriptLine(prefix) {
  const lines = spawnSync("bash", ["-c", `cat "${SCRIPT}"`], { encoding: "utf8" }).stdout.split("\n");
  const found = lines.filter((line) => line.startsWith(prefix));
  assert.equal(found.length, 1, `the scrub must declare ${prefix} exactly once; found ${found.length}`);
  return found[0];
}

// --- Host shell contract -------------------------------------------------
// Recipes here are POSIX shell. Windows must run them under Git Bash, found by
// absolute path: `bash.exe` resolved by name finds WSL's bash in System32,
// which mounts a different filesystem and cannot see this checkout at all.

/** The trimmed Makefile line declaring `name`, wherever it is indented. */
function declarationOf(name) {
  const found = makefileLines().map((line) => line.trim()).filter((line) => line.startsWith(`${name} `));
  assert.equal(found.length, 1, `the Makefile must declare ${name} exactly once; found ${found.length}`);
  return found[0];
}

test("Windows runs the POSIX recipes under Git Bash, by absolute path", () => {
  assert.ok(declarationOf("SHELL").endsWith("$(GIT_BASH)"), "the Windows shell must come from the overridable GIT_BASH");
  const gitBash = declarationOf("GIT_BASH");
  assert.ok(gitBash.includes(":/"), `GIT_BASH must be an absolute path, not a PATH lookup: ${gitBash}`);
  assert.ok(gitBash.endsWith("bash.exe"), `GIT_BASH must name bash.exe: ${gitBash}`);
  assert.ok(declarationOf(".SHELLFLAGS").endsWith("-c"), "a POSIX shell takes -c, not PowerShell's -Command");
});

test("no recipe is handed to PowerShell, which cannot parse a POSIX recipe", () => {
  const offending = makefileLines().filter((line) => line.includes("powershell"));
  assert.deepEqual(offending, [], "recipes use case/for/[ -f ]/|| — PowerShell rejects every one of them");
});

test("the file-removal helpers stay POSIX, because recipes interpolate them into POSIX shell", () => {
  assert.deepEqual(variableWords("RM"), ["rm", "-rf"]);
  assert.deepEqual(variableWords("MKDIR"), ["mkdir", "-p"]);
});
