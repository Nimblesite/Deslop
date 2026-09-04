// Fixture binaries that really are binaries.
// [DEPLOY-GATE-PORTABILITY] [DEPLOY-VSIX-PACKAGE]
//
// Every deployment verifier ends by running the artifact it is checking and
// reading what it prints, so a proof that a verifier rejects a drifted binary
// has to stage something a verifier can actually run. A shell script is that
// on Unix and nothing at all on Windows, where `CreateProcess` starts an image
// and not a text file — so on Windows each proof spawned nothing, read empty
// output, and passed anyway.
//
// The fixture is therefore compiled, once, from `fake-binary.rs`. `rustc` is
// the one compiler this repository is guaranteed to have: the deployment gate
// runs after the workspace build, and the CI job that runs it installs the
// Rust toolchain for exactly that reason. Each fixture is a copy of that one
// build with its own two answers appended, so staging thirty of them costs one
// compile.

import { spawnSync } from "node:child_process";
import { appendFileSync, chmodSync, copyFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { currentPlatform, executableName } from "../release/vsix-platforms.mjs";

/** Must equal `MARKER` in fake-binary.rs, byte for byte. */
const PAYLOAD_MARKER = "\n@@DESLOP-FAKE-BINARY-PAYLOAD@@\n";

/** Mode a staged fixture carries, matching what a release build produces. */
const EXECUTABLE_MODE = 0o755;

/** Optimised and stripped: every fixture is copied into a zip several times. */
const RUSTC_ARGUMENTS = ["--edition", "2021", "-O", "-C", "strip=symbols"];

const SOURCE = fileURLToPath(new URL("fake-binary.rs", import.meta.url));

let compiled;

/**
 * Path to the compiled fixture program, building it on first use.
 *
 * @returns {string} absolute path to an executable for this host
 */
export function fakeBinaryTemplate() {
  if (compiled !== undefined) return compiled;
  const directory = mkdtempSync(join(tmpdir(), "deslop-fake-binary-"));
  const output = join(directory, executableName("fake", currentPlatform()));
  const result = spawnSync("rustc", [...RUSTC_ARGUMENTS, "-o", output, SOURCE], {
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(`rustc could not build ${SOURCE}: ${result.stderr ?? result.error}`);
  }
  compiled = output;
  return compiled;
}

/**
 * Stages a runnable fixture at `path` that answers `--version` with
 * `answers.plain` and `--version --json` with `answers.json`.
 *
 * @param {string} path where to write the fixture, suffix included
 * @param {{plain: string, json: object}} answers what the fixture reports
 * @returns {string} `path`
 */
export function writeFakeBinary(path, answers) {
  copyFileSync(fakeBinaryTemplate(), path);
  const lines = [answers.plain, JSON.stringify(answers.json)];
  appendFileSync(path, `${PAYLOAD_MARKER}${lines.join("\n")}\n`);
  chmodSync(path, EXECUTABLE_MODE);
  return path;
}
