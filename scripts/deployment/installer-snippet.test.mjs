// Installer-snippet fail-closed contract. [DEPLOY-DOCS-INSTALLER-FAILCLOSED]
//
// The curl installer published on the docs pages must never extract or install
// an archive whose SHA-256 verification failed. Each test extracts the exact
// snippet from the published page, runs it against a local fixture release
// (no network: `DESLOP_RELEASE_BASE` points at a file:// mirror and
// `DESLOP_TAG` pins the version — both honored by the published snippet), and
// asserts on the recorded `tar`/`sudo` invocations. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { arch, tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { repoRoot } from "../lib/repo-root.mjs";

const TAG = "v9.9.9";
const VERSION = "9.9.9";
const BINARIES = ["deslop", "deslop-lsp", "deslop-mcp"];
const PAGES = [
  { name: "en", path: "site/src/docs/index.md", heading: "### macOS / Linux (curl)" },
  { name: "zh", path: "site/src/zh/docs/index.md", heading: "### macOS / Linux（curl）" },
];

function hostPlatform() {
  const mapping = {
    "linux-x64": "linux-x64",
    "linux-arm64": "linux-arm64",
    "darwin-arm64": "macos-arm64",
    "darwin-x64": "macos-x64",
  };
  const platform = mapping[`${process.platform}-${arch()}`];
  assert.ok(platform, `unsupported test host: ${process.platform}-${arch()}`);
  return platform;
}

// Fence extraction by exact line matching — no pattern matching on the code.
function extractSnippet(page) {
  const lines = readFileSync(resolve(repoRoot, page.path), "utf8").split("\n");
  const headingAt = lines.indexOf(page.heading);
  assert.ok(headingAt >= 0, `${page.path}: heading not found: ${page.heading}`);
  const open = lines.indexOf("```bash", headingAt);
  assert.ok(open >= 0, `${page.path}: no bash fence after the curl heading`);
  const close = lines.indexOf("```", open + 1);
  assert.ok(close > open, `${page.path}: unterminated bash fence`);
  return lines.slice(open + 1, close).join("\n");
}

function sha256Of(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function writeFixtureRelease(fixtures, platform, goodChecksum) {
  const releaseDir = join(fixtures, "download", TAG);
  const payload = `deslop-${VERSION}-${platform}`;
  const stage = join(fixtures, "stage");
  mkdirSync(releaseDir, { recursive: true });
  mkdirSync(join(stage, payload), { recursive: true });
  for (const binary of BINARIES) {
    writeFileSync(join(stage, payload, binary), `#!/bin/sh\necho ${binary} ${VERSION}\n`, { mode: 0o755 });
  }
  const archive = join(releaseDir, `${payload}.tar.gz`);
  const tar = spawnSync("tar", ["-czf", archive, "-C", stage, payload]);
  assert.equal(tar.status, 0, "fixture tar failed");
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
function writeStubs(stubBin, log) {
  mkdirSync(stubBin, { recursive: true });
  const realTar = spawnSync("bash", ["-c", "command -v tar"], { encoding: "utf8" }).stdout.trim();
  assert.ok(realTar, "real tar not found on PATH");
  writeStub(stubBin, "tar", `echo "tar $*" >> "${log}"\nexec "${realTar}" "$@"`);
  writeStub(stubBin, "sudo", `echo "sudo $*" >> "${log}"`);
  writeStub(stubBin, "deslop", `echo "deslop $*" >> "${log}"\necho "deslop ${VERSION}"`);
}

function runSnippet(snippet, sandbox) {
  return spawnSync("bash", ["-c", snippet], {
    cwd: join(sandbox, "cwd"),
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${join(sandbox, "stub-bin")}:${process.env.PATH}`,
      DESLOP_TAG: TAG,
      DESLOP_RELEASE_BASE: pathToFileURL(join(sandbox, "fixtures")).href,
      TMPDIR: join(sandbox, "tmp"),
    },
  });
}

function setupAndRun(page, goodChecksum) {
  const sandbox = mkdtempSync(join(tmpdir(), "deslop-installer-"));
  const platform = hostPlatform();
  const log = join(sandbox, "recorder.log");
  mkdirSync(join(sandbox, "tmp"));
  mkdirSync(join(sandbox, "cwd"));
  writeFixtureRelease(join(sandbox, "fixtures"), platform, goodChecksum);
  writeStubs(join(sandbox, "stub-bin"), log);
  const result = runSnippet(extractSnippet(page), sandbox);
  return { result, sandbox, platform, log };
}

function recordedLines(log) {
  return existsSync(log) ? readFileSync(log, "utf8").split("\n").filter(Boolean) : [];
}

function assertFailsClosed(page) {
  const { result, sandbox, platform, log } = setupAndRun(page, false);
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

function assertInstalls(page) {
  const { result, sandbox, platform, log } = setupAndRun(page, true);
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

function withoutTrailingComment(line) {
  const at = line.indexOf(" #");
  return at < 0 ? line : line.slice(0, at).trimEnd();
}

test("EN and ZH installer snippets are functionally identical (comments are the only translated lines)", () => {
  const [en, zh] = PAGES.map((page) => extractSnippet(page).split("\n").map(withoutTrailingComment));
  assert.deepEqual(en, zh, "EN and ZH installer snippets diverge outside comments");
});

test("both pages' no-sudo alternative creates ~/.local/bin before installing", () => {
  for (const page of PAGES) {
    const text = readFileSync(resolve(repoRoot, page.path), "utf8");
    assert.ok(
      text.includes('mkdir -p ~/.local/bin && install -m 755 "deslop-${version}-${platform}"/deslop{,-lsp,-mcp} ~/.local/bin/'),
      `${page.path}: the no-sudo alternative must mkdir -p ~/.local/bin first`,
    );
  }
});

for (const page of PAGES) {
  test(`${page.name}: a bad checksum aborts before extraction and installation`, () => assertFailsClosed(page));
  test(`${page.name}: a good checksum extracts, installs all three binaries, and cleans up`, () => assertInstalls(page));
}
