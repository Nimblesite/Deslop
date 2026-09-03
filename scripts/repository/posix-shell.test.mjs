// Host-shell resolution contract. [DEPLOY-EXTENSION-BUNDLED-TESTS]
//
// The gates that drive Deslop's published shell scripts need three things
// from this host: a shell that can actually be spawned, the spelling that
// shell uses for a path, and the spelling this host uses for a path the shell
// named. Get any of them wrong on Windows and the failure is silent in the
// worst way — `spawnSync` returns a null status and empty output, and an
// assertion written against "what the script printed" is then comparing two
// empty strings and passing. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { platform } from "node:os";
import { isAbsolute, resolve } from "node:path";

import { repoRoot } from "../lib/repo-root.mjs";
import { variableValue } from "../lib/makefile.mjs";
import {
  GIT_BASH_VARIABLE,
  POSIX_SHELL,
  hostPath,
  posixShell,
  shellPath,
} from "../lib/posix-shell.mjs";

/** The two hosts the helper distinguishes. */
const WINDOWS = "win32";
const LINUX = "linux";

/** A directory every checkout has, used as the round-trip subject. */
const SUBJECT = resolve(repoRoot, "scripts", "repository");

test("the shell make runs POSIX recipes under is the shell these tests spawn", () => {
  const declared = variableValue(GIT_BASH_VARIABLE);
  assert.notEqual(declared, "", `the Makefile must declare ${GIT_BASH_VARIABLE}`);
  assert.ok(isAbsolute(declared), `${GIT_BASH_VARIABLE} must be absolute: ${declared}`);
  assert.equal(
    posixShell(WINDOWS),
    process.env[GIT_BASH_VARIABLE] ?? declared,
    "a Windows host must resolve the shell the Makefile declares, not a second copy of the path",
  );
  assert.equal(posixShell(LINUX), POSIX_SHELL, "a POSIX host keeps its own shell");
});

test("the resolved shell exists and runs a POSIX script", () => {
  const shell = posixShell();
  assert.ok(isAbsolute(shell), `the shell must be named absolutely: ${shell}`);
  assert.equal(existsSync(shell), true, `${shell} does not exist — set ${GIT_BASH_VARIABLE}`);
  // `case` is the construct that made these recipes POSIX-only in the first
  // place, so it is the one worth proving the resolved shell can parse.
  const result = spawnSync(shell, ["-c", 'case "$1" in ok) echo parsed ;; *) exit 3 ;; esac', "sh", "ok"], {
    encoding: "utf8",
  });
  assert.equal(result.status, 0, `the resolved shell could not run a POSIX script: ${result.stderr}`);
  assert.equal(result.stdout.trim(), "parsed");
});

test("a path survives the round trip between host and shell spelling", () => {
  const asShell = shellPath(SUBJECT);
  assert.equal(hostPath(asShell), SUBJECT, "translating a path and back must return the original");
  assert.equal(
    spawnSync(posixShell(), ["-c", '[ -d "$1" ]', "sh", asShell], { encoding: "utf8" }).status,
    0,
    `the shell cannot open ${asShell} — the spelling handed to it is not one it understands`,
  );
});

test("a shell PATH entry never carries the character PATH separates on", () => {
  const entry = shellPath(SUBJECT);
  assert.ok(entry.length > 0, "a translated PATH entry must not be empty");
  assert.equal(
    entry.includes(":"),
    false,
    `${entry} would be read as two PATH entries, and the shell would find neither`,
  );
  // The same subject, untranslated, is exactly what the mistake looks like on
  // Windows — proving the translation is doing work rather than passing a
  // string through that happened to be fine already.
  if (platform() === WINDOWS) {
    assert.equal(SUBJECT.includes(":"), true, "a Windows path carries the separator, which is why translation exists");
  }
});
