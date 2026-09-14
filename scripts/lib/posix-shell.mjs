// The POSIX shell a Node contract test can spawn on this host, and the path
// spellings that shell understands.
// [DEPLOY-GATE-PORTABILITY] [DEPLOY-EXTENSION-BUNDLED-TESTS]
//
// Several gates here drive shell scripts Deslop publishes for macOS and Linux
// — the PATH scrub, the process scrub, the documented curl installer. The
// scripts are POSIX, so the tests must run them under a POSIX shell, and on
// Windows that shell is Git Bash. Two things then stop being interchangeable:
//
//   * `/bin/bash` is not a path Windows can spawn. It is an MSYS name that
//     exists only once bash is already running, so a test that hard-codes it
//     never starts a process at all and then reports on the empty output of a
//     program that never ran — a green gate over an unexercised script.
//   * A Windows path cannot go into a POSIX `PATH`. `C:\repo\bin` carries the
//     very character `PATH` separates on, so the shell reads one entry as two
//     and finds neither.
//
// The Windows shell location is read from the Makefile's own `GIT_BASH` rather
// than copied, so a developer whose Git for Windows lives elsewhere overrides
// one variable and both make and these tests follow it.

import { spawnSync } from "node:child_process";
import { platform } from "node:os";

import { variableValue } from "./makefile.mjs";

/** Make variable naming the Windows shell, and the environment override. */
export const GIT_BASH_VARIABLE = "GIT_BASH";

/** Where a POSIX host keeps the shell these scripts are written for. */
export const POSIX_SHELL = "/bin/bash";

/** Git Bash's own path translator, and its two directions. */
const TRANSLATOR = "cygpath";
const TO_SHELL = "-u";
const TO_HOST = "-w";

/**
 * Absolute path to a shell that can run this repository's POSIX scripts.
 *
 * @param {string} [host] `os.platform()` value; defaults to this host
 * @returns {string} a path `spawnSync` can execute on `host`
 */
export function posixShell(host = platform()) {
  if (host !== "win32") return POSIX_SHELL;
  return process.env[GIT_BASH_VARIABLE] ?? variableValue(GIT_BASH_VARIABLE);
}

/**
 * How [`posixShell`] spells `path` — `/c/repo/bin` for `C:\repo\bin`.
 *
 * Use it for anything handed to the shell as a `PATH` entry or compared
 * against a path the script printed. Off Windows it is the identity.
 *
 * @param {string} path a path spelled the way this host spells it
 * @param {string} [host] `os.platform()` value; defaults to this host
 * @returns {string}
 */
export function shellPath(path, host = platform()) {
  return translate(TO_SHELL, path, host);
}

/**
 * How this host spells a path the shell named — the inverse of
 * [`shellPath`]. Use it for anything a POSIX name must be opened by, such as
 * checking what actually sits in the shell's `/usr/bin`.
 *
 * @param {string} path a path spelled the way [`posixShell`] spells it
 * @param {string} [host] `os.platform()` value; defaults to this host
 * @returns {string}
 */
export function hostPath(path, host = platform()) {
  return translate(TO_HOST, path, host);
}

/**
 * Argv for running the installed command `name` with `args` on this host.
 *
 * Windows installs an npm command as a `.cmd` shim, and Node refuses to spawn
 * a batch file without a shell — so `spawnSync("npx", ...)` cannot start one
 * there at all, and a caller that tries reports the command as having failed
 * rather than as never having run. Routing through [`posixShell`], which this
 * repository already requires on Windows, starts it. Every argument stays
 * positional, so nothing is ever interpolated into a command line.
 *
 * @param {string} name the installed command to run
 * @param {string[]} args its arguments, passed through untouched
 * @param {string} [host] `os.platform()` value; defaults to this host
 * @returns {[string, string[]]} the file to spawn and the arguments for it
 */
export function spawnableCommand(name, args, host = platform()) {
  if (host !== "win32") return [name, args];
  return [posixShell(host), ["-c", 'exec "$@"', name, name, ...args]];
}

/**
 * Runs the shell's own translator. The path travels as a positional argument
 * rather than inside the command text, so a directory name is never read as
 * shell syntax.
 *
 * @param {string} direction
 * @param {string} path
 * @param {string} host
 * @returns {string}
 */
function translate(direction, path, host) {
  if (host !== "win32") return path;
  const result = spawnSync(
    posixShell(host),
    ["-c", `${TRANSLATOR} "$1" "$2"`, TRANSLATOR, direction, path],
    { encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(`${TRANSLATOR} ${direction} ${path} failed: ${result.stderr ?? result.error}`);
  }
  return result.stdout.trim();
}
