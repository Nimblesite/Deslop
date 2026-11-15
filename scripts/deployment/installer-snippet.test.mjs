// Installer fail-closed contract. [DEPLOY-DOCS-INSTALLER-FAILCLOSED]
//
// `install.sh`, which the docs pages tell readers to pipe into bash, must never
// extract or install an archive whose SHA-256 verification failed. Each test
// runs the script exactly as the published command does (`bash` reading it on
// stdin) against a local fixture release (no network: `DESLOP_RELEASE_BASE`
// points at a file:// mirror and `DESLOP_TAG` pins the version), and asserts on
// the recorded `tar`/`sudo` invocations. Run with `node --test`.
//
// The snippet chooses its archive from `uname`, so `uname` is stubbed too and
// the platform under test is chosen by the test rather than by whichever
// machine happens to be running it. That makes every assertion here identical
// on every host, puts all four published platforms under test on all of them,
// and lets the snippet's own unsupported-platform refusal be asserted instead
// of only being met by the host the suite could not run on.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { repoRoot } from "../lib/repo-root.mjs";
import { posixShell, shellPath } from "../lib/posix-shell.mjs";
import { writeFileAt } from "../lib/write-file.mjs";

const TAG = "v9.9.9";
const VERSION = "9.9.9";
const BINARIES = ["deslop", "deslop-lsp", "deslop-mcp"];
const SCRIPT = { name: "install.sh", path: "install.sh" };
const PUBLISHED_COMMAND =
  "curl -fsSL https://raw.githubusercontent.com/Nimblesite/Deslop/main/install.sh | bash";
const PAGES = ["site/src/docs/getting-started.md", "site/src/zh/docs/getting-started.md"];

/** The shell that runs the published snippet, whatever this host is. */
const BASH = posixShell();

/** Every `uname` answer the published snippet claims to support, and the
 * release archive each one must select. The snippet is the only source of
 * this mapping; the table restates it so a silent edit fails here. */
const PLATFORMS = [
  { system: "Linux", machine: "x86_64", platform: "linux-x64" },
  { system: "Linux", machine: "aarch64", platform: "linux-arm64" },
  { system: "Darwin", machine: "arm64", platform: "macos-arm64" },
  { system: "Darwin", machine: "x86_64", platform: "macos-x64" },
];

/** The platform the checksum scenarios run on. Any of the four would do —
 * the mapping itself is asserted over all of them separately. */
const DEFAULT_TARGET = PLATFORMS[0];

/** A `uname` answer no release is published for. */
const UNSUPPORTED_TARGET = { system: "Plan9", machine: "sparc", platform: undefined };

function sha256Of(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function writeFixtureRelease(fixtures, platform, goodChecksum) {
  const releaseDir = join(fixtures, "download", TAG);
  const payload = `deslop-${VERSION}-${platform}`;
  const stage = join(fixtures, "stage");
  mkdirSync(releaseDir, { recursive: true });
  for (const binary of BINARIES) {
    writeFileAt(join(stage, payload, binary), `#!/bin/sh\necho ${binary} ${VERSION}\n`, { mode: 0o755 });
  }
  const archive = join(releaseDir, `${payload}.tar.gz`);
  // Built through the same shell that will extract it, with the paths spelled
  // the way that shell spells them: GNU tar reads a leading `C:` as a remote
  // host and refuses the archive outright.
  const tar = spawnSync(
    BASH,
    ["-c", 'tar -czf "$1" -C "$2" "$3"', "tar", shellPath(archive), shellPath(stage), payload],
    { encoding: "utf8" },
  );
  assert.equal(tar.status, 0, `fixture tar failed: ${tar.stderr}`);
  const digest = sha256Of(archive);
  const published = goodChecksum ? digest : `${digest[0] === "0" ? "1" : "0"}${digest.slice(1)}`;
  writeFileSync(`${archive}.sha256`, `${published}  ${payload}.tar.gz\n`);
}

function writeStub(stubBin, name, body) {
  writeFileSync(join(stubBin, name), `#!/bin/sh\n${body}\n`, { mode: 0o755 });
}

// tar delegates to the real binary so extraction genuinely happens on the
// good-checksum path; sudo and deslop only record, so nothing touches
// /usr/local/bin and no real binary is needed.
function writeStubs(stubBin, log, target) {
  mkdirSync(stubBin, { recursive: true });
  const realTar = spawnSync(BASH, ["-c", "command -v tar"], { encoding: "utf8" }).stdout.trim();
  assert.ok(realTar, "real tar not found on PATH");
  const recorded = shellPath(log);
  writeStub(stubBin, "tar", `echo "tar $*" >> "${recorded}"\nexec "${realTar}" "$@"`);
  writeStub(stubBin, "sudo", `echo "sudo $*" >> "${recorded}"`);
  writeStub(stubBin, "deslop", `echo "deslop $*" >> "${recorded}"\necho "deslop ${VERSION}"`);
  writeUnameStub(stubBin, target);
}

/** Names the platform under test, and refuses every flag it was not given. */
function writeUnameStub(stubBin, target) {
  // The snippet reads the platform from `uname`, so the test names it. Every
  // other flag is refused rather than answered: a snippet that asked
  // something this stub silently made up would be tested against a fiction.
  writeStub(
    stubBin,
    "uname",
    [
      'case "$1" in',
      `  -s) echo ${target.system} ;;`,
      `  -m) echo ${target.machine} ;;`,
      '  *) echo "stub uname: unexpected flag $1" >&2; exit 1 ;;',
      "esac",
    ].join("\n"),
  );
}

function runSnippet(sandbox, extraEnv) {
  return spawnSync(BASH, [], {
    input: readFileSync(resolve(repoRoot, SCRIPT.path), "utf8"),
    cwd: join(sandbox, "cwd"),
    encoding: "utf8",
    env: {
      ...process.env,
      // A PATH entry is spelled the way the shell spells it: a host path can
      // carry the very character PATH separates on, and the shell would then
      // read one directory as two and find neither.
      PATH: `${shellPath(join(sandbox, "stub-bin"))}:${process.env.PATH}`,
      DESLOP_TAG: TAG,
      DESLOP_RELEASE_BASE: pathToFileURL(join(sandbox, "fixtures")).href,
      TMPDIR: shellPath(join(sandbox, "tmp")),
      ...extraEnv,
    },
  });
}

function setupAndRun(page, goodChecksum, target = DEFAULT_TARGET, envFor = () => ({})) {
  const sandbox = mkdtempSync(join(tmpdir(), "deslop-installer-"));
  const platform = target.platform;
  const log = join(sandbox, "recorder.log");
  mkdirSync(join(sandbox, "tmp"));
  mkdirSync(join(sandbox, "cwd"));
  if (platform) writeFixtureRelease(join(sandbox, "fixtures"), platform, goodChecksum);
  else mkdirSync(join(sandbox, "fixtures"), { recursive: true });
  writeStubs(join(sandbox, "stub-bin"), log, target);
  const result = runSnippet(sandbox, envFor(sandbox));
  return { result, sandbox, platform, log };
}

function recordedLines(log) {
  return existsSync(log) ? readFileSync(log, "utf8").split("\n").filter(Boolean) : [];
}

function assertFailsClosed(page, target = DEFAULT_TARGET) {
  const { result, sandbox, platform, log } = setupAndRun(page, false, target);
  const archive = `deslop-${VERSION}-${platform}.tar.gz`;
  const output = `${result.stdout}\n${result.stderr}`;
  assert.notEqual(result.status, 0, `${page.name}: snippet exited 0 despite a bad checksum`);
  assert.ok(output.includes(`${archive}: FAILED`), `${page.name}: checksum verification never reported the mismatch:\n${output}`);
  assert.deepEqual(recordedLines(log), [], `${page.name}: tar/sudo/deslop ran after a failed checksum`);
  assert.deepEqual(readdirSync(join(sandbox, "tmp")), [], `${page.name}: work directory leaked after failure`);
  assert.deepEqual(readdirSync(join(sandbox, "cwd")), [], `${page.name}: files were written to the caller's directory`);
  assert.ok(!output.includes(`deslop ${VERSION}`), `${page.name}: deslop --version ran despite a failed checksum`);
  rmSync(sandbox, { recursive: true, force: true });
}

function assertInstalls(page, target = DEFAULT_TARGET) {
  const { result, sandbox, platform, log } = setupAndRun(page, true, target);
  const archive = `deslop-${VERSION}-${platform}.tar.gz`;
  const lines = recordedLines(log);
  const tarLine = lines.find((line) => line.startsWith("tar "));
  const sudoLine = lines.find((line) => line.startsWith("sudo "));
  assert.equal(result.status, 0, `${page.name}: verified install failed:\n${result.stderr}`);
  assert.ok(result.stdout.includes(`${archive}: OK`), `${page.name}: checksum verification did not pass`);
  assert.ok(tarLine?.includes("-xzf") && tarLine.includes(archive), `${page.name}: archive was not extracted: ${tarLine}`);
  assertSudoInstallsAllBinaries(page, sudoLine, platform);
  assert.ok(lines.includes("deslop --version"), `${page.name}: deslop --version did not run after install`);
  assert.ok(result.stdout.includes(`deslop ${VERSION}`), `${page.name}: installed version was not printed`);
  assert.deepEqual(readdirSync(join(sandbox, "tmp")), [], `${page.name}: work directory leaked after success`);
  rmSync(sandbox, { recursive: true, force: true });
}

function assertSudoInstallsAllBinaries(page, sudoLine, platform) {
  assert.ok(sudoLine?.startsWith("sudo install -m 755 "), `${page.name}: install was not invoked via sudo: ${sudoLine}`);
  assert.ok(sudoLine.includes(" /usr/local/bin/"), `${page.name}: install target is not /usr/local/bin: ${sudoLine}`);
  for (const binary of BINARIES) {
    assert.ok(sudoLine.includes(` deslop-${VERSION}-${platform}/${binary} `), `${page.name}: ${binary} was not installed: ${sudoLine}`);
  }
}

test("both docs pages publish the same one-line install command for install.sh", () => {
  for (const page of PAGES) {
    const lines = readFileSync(resolve(repoRoot, page), "utf8").split("\n");
    assert.ok(lines.includes(PUBLISHED_COMMAND), `${page}: must publish exactly: ${PUBLISHED_COMMAND}`);
  }
});

test("DESLOP_INSTALL_DIR installs all three binaries there without sudo", () => {
  const installDirIn = (sandbox) => join(sandbox, "home", ".local", "bin");
  const { result, sandbox, log } = setupAndRun(SCRIPT, true, DEFAULT_TARGET, (root) => ({
    DESLOP_INSTALL_DIR: shellPath(installDirIn(root)),
  }));
  const installDir = installDirIn(sandbox);
  assert.equal(result.status, 0, `user-local install failed:\n${result.stderr}`);
  assert.deepEqual(readdirSync(installDir).sort(), [...BINARIES].sort(), "every binary lands in DESLOP_INSTALL_DIR");
  assert.ok(!recordedLines(log).some((line) => line.startsWith("sudo ")), "a user-local install never calls sudo");
  rmSync(sandbox, { recursive: true, force: true });
});

test("a bad checksum aborts before extraction and installation", () => assertFailsClosed(SCRIPT));
test("a good checksum extracts, installs all three binaries, and cleans up", () => assertInstalls(SCRIPT));

// [DEPLOY-DOCS-INSTALLER-FAILCLOSED] The snippet picks its archive from
// `uname`, and picking the wrong one is a 404 for the user rather than an
// install. Every published platform is exercised, not just this host's.
for (const target of PLATFORMS) {
  test(`${target.system}-${target.machine} installs the ${target.platform} archive`, () => {
    assertInstalls(SCRIPT, target);
  });
}

// [DEPLOY-DOCS-INSTALLER-FAILCLOSED] The refusal arm of that same `case`. A
// platform with no published release must stop before the first download —
// not fetch a URL that does not exist and report whatever curl said.
test("an unsupported platform aborts before anything is downloaded or installed", () => {
  const named = `${UNSUPPORTED_TARGET.system}-${UNSUPPORTED_TARGET.machine}`;
  for (const page of [SCRIPT]) {
    const { result, sandbox, log } = setupAndRun(page, true, UNSUPPORTED_TARGET);
    const output = `${result.stdout}\n${result.stderr}`;
    assert.notEqual(result.status, 0, `${page.name}: snippet exited 0 on ${named}`);
    assert.ok(
      output.includes(`unsupported platform: ${named}`),
      `${page.name}: the refusal must name the platform it could not serve:\n${output}`,
    );
    assert.deepEqual(recordedLines(log), [], `${page.name}: tar/sudo/deslop ran on ${named}`);
    assert.deepEqual(readdirSync(join(sandbox, "tmp")), [], `${page.name}: work directory leaked on ${named}`);
    assert.deepEqual(readdirSync(join(sandbox, "cwd")), [], `${page.name}: files were written to the caller's directory`);
    rmSync(sandbox, { recursive: true, force: true });
  }
});
