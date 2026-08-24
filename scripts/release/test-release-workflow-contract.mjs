// Release workflow contract tests for the Shipwright deployment path.
//
// These are intentionally focused on the workflow, not the verifier script:
// the tests fail when the tagged source and published artifacts can drift, or
// when package-manager manifests declare binaries missing from the archives.

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("../..", import.meta.url));
const workflowPath = resolve(repoRoot, ".github/workflows/release.yml");
const deployWorkflowPath = resolve(repoRoot, ".github/workflows/deploy-pages.yml");
const dependabotWorkflowPath = resolve(repoRoot, ".github/workflows/dependabot-automerge.yml");
const workflow = readFileSync(workflowPath, "utf8");
const deployWorkflow = readFileSync(deployWorkflowPath, "utf8");
const dependabotWorkflow = readFileSync(dependabotWorkflowPath, "utf8");

const tests = [
  releaseBuildsTaggedSourceWithoutPostTagVersionCommit,
  releaseArchivesContainPackageManagerDeclaredBinaries,
  scoopManifestExtractDirMatchesWindowsArchiveRoot,
  scoopAutoupdateTemplatesEveryVersionedSegment,
  releaseBuildsPlatformSpecificVsixArtifacts,
  pagesDeployCleansRerunArtifactsAndRetries,
  dependabotSweepLeavesNoDeadCheckOnHumanPullRequests,
  contractFilesCheckOutLfOnEveryPlatform,
];

// Every file this suite and test-action-contract.mjs match line-exactly. Git for
// Windows ships core.autocrlf=true in its system config, so without an explicit
// attribute these check out CRLF and every `\n`-anchored match silently misses.
const LINE_MATCHED_FILES = [
  ".github/workflows/release.yml",
  ".github/workflows/deploy-pages.yml",
  ".github/workflows/dependabot-automerge.yml",
  "action.yml",
];

// Values bound while evaluating the Scoop manifest generator. The version is
// deliberately not the current one: a manifest that hardcodes a real version
// would still look right against it.
const SCOOP_TEST_VERSION = "9.8.7";
const SCOOP_TEST_SHA256 = "d713ca72419bc535e6c64605381255e544553356290b900b6c3f1eed21bee735";
const SCOOP_TEST_REPOSITORY = "Nimblesite/Deslop";

runContractSuite(tests, "release workflow contract");

function releaseBuildsTaggedSourceWithoutPostTagVersionCommit() {
  const versionJob = sectionBetween("  version:", "  build:");
  assertAbsent(
    /ref:\s*main\b/,
    "release jobs must not checkout main; tag releases must build the tagged commit",
  );
  assertSectionAbsent(
    versionJob,
    /\bgit\s+commit\b/,
    "release workflow must not commit a version bump after the tag already triggered the release",
  );
  assertSectionAbsent(
    versionJob,
    /\bgit\s+push\b/,
    "release workflow must not push mutable source changes during a tag release",
  );
  assertPresent(
    /node scripts\/release\/stamp-release-version\.mjs "\$\{\{ steps\.extract\.outputs\.version \}\}"/,
    "release workflow must stamp the tag version as a build-time input",
  );
}

function releaseArchivesContainPackageManagerDeclaredBinaries() {
  const unixArchiveStep = sectionBetween(
    "- name: Package archive (unix)",
    "- name: Package archive (windows)",
  );
  const windowsArchiveStep = sectionBetween(
    "- name: Package archive (windows)",
    "- name: Stage VSIX binaries (unix)",
  );

  for (const binary of ["deslop-lsp", "deslop-mcp"]) {
    assertIncludes(unixArchiveStep, binary, `Unix release archives must include ${binary}`);
  }
  for (const binary of ["deslop-lsp.exe", "deslop-mcp.exe"]) {
    assertIncludes(
      windowsArchiveStep,
      binary,
      `Scoop declares ${binary}, so the Windows release archive must contain it`,
    );
  }
}

// The Windows zip nests every binary under a top-level staging directory, and
// Scoop resolves `bin` from the app root — so the manifest has to name that
// directory in `extract_dir` or shim creation fails after the download and hash
// check have already passed. `autoupdate` needs it too: a manifest that carries
// extract_dir only in the architecture block regresses on the next auto-update.
// The expected value is derived from the packaging step, not hardcoded, so
// flattening the archive and dropping extract_dir is equally accepted.
function scoopManifestExtractDirMatchesWindowsArchiveRoot() {
  const archiveRoot = windowsArchiveRoot(SCOOP_TEST_VERSION);
  const manifest = generatedScoopManifest(SCOOP_TEST_VERSION, SCOOP_TEST_SHA256);

  assertEqual(
    manifest.architecture["64bit"].extract_dir ?? null,
    archiveRoot,
    "the Scoop manifest must declare extract_dir matching the Windows archive's top-level directory",
  );
  assertEqual(
    manifest.autoupdate.architecture["64bit"].extract_dir ?? null,
    archiveRoot === null ? null : archiveRoot.split(SCOOP_TEST_VERSION).join("$version"),
    "autoupdate must carry the same extract_dir, templated on $version, or the next auto-updated manifest reverts to a broken shim",
  );
}

// Every versioned segment of the autoupdate URL has to stay a Scoop `$version`
// placeholder. Interpolating one at generation time freezes it: the tag segment
// keeps pointing at the release that produced the manifest, so every future
// version resolves back to that release's assets.
function scoopAutoupdateTemplatesEveryVersionedSegment() {
  const autoupdate = generatedScoopManifest(SCOOP_TEST_VERSION, SCOOP_TEST_SHA256).autoupdate;
  const url = autoupdate.architecture["64bit"].url;

  assertIncludes(
    url,
    "/download/v$version/",
    `the autoupdate url must template the release tag segment; got ${url}`,
  );
  assertExcludes(
    url,
    SCOOP_TEST_VERSION,
    `the autoupdate url must not embed the generating version (${SCOOP_TEST_VERSION}); a frozen segment pins every future update to this release`,
  );
}

// The archive root is whatever `Compress-Archive -Path` is handed: a staging
// directory becomes the archive's top-level entry, while `dist/$stage/*` or a
// bare file produces a flat archive. Returns null when the archive is flat, so
// the caller can require extract_dir to be absent instead.
function windowsArchiveRoot(version) {
  const step = sectionBetween(
    "- name: Package archive (windows)",
    "- name: Stage VSIX binaries (unix)",
  );
  const stage = expandGitHub(quotedValueAfter(step, '$stage = "'), version);
  const packed = quotedValueAfter(step, 'Compress-Archive -Path "');
  return packed === "dist/$stage" ? stage : null;
}

// Evaluates the workflow's manifest generator the way the runner does — GitHub
// expands `${{ … }}`, then bash expands `${…}` in the heredoc — and parses what
// it emits. Asserting on the generated manifest rather than on workflow text is
// what makes a half-templated value visible at all: `${base}/deslop-\$version…`
// reads as templated in the source and resolves to a frozen tag in the output.
function generatedScoopManifest(version, sha256) {
  const step = sectionBetween("- name: Build Scoop manifest", "- name: Checkout scoop-bucket");
  const opener = step.indexOf("<<EOF");
  if (opener < 0) throw new Error("missing Scoop manifest heredoc");
  const body = step.slice(opener + "<<EOF".length, step.indexOf("EOF", opener + "<<EOF".length));
  return JSON.parse(expandShell(expandGitHub(body, version), shellBindings(step, version, sha256)));
}

// Replays the step's own assignments in order, so the test never restates how
// the asset name or download URL is composed — it reuses the workflow's.
function shellBindings(step, version, sha256) {
  const bindings = new Map([["win_sha", sha256]]);
  for (const name of ["version", "base", "win_zip"]) {
    const assigned = expandGitHub(quotedValueAfter(step, `${name}="`), version);
    bindings.set(name, expandShell(assigned, bindings));
  }
  return bindings;
}

function expandGitHub(text, version) {
  return substitute(text, "${{", "}}", (expression) => {
    if (expression === "github.repository") return SCOOP_TEST_REPOSITORY;
    if (expression === "matrix.artifact_name") return "windows-x64";
    if (expression === "needs.version.outputs.version") return version;
    throw new Error(`unbound GitHub expression: ${expression}`);
  });
}

// `\$` reaches the manifest as a literal `$`, which is how Scoop's `$version`
// placeholder survives the heredoc.
function expandShell(text, bindings) {
  const expanded = substitute(text, "${", "}", (name) => {
    if (!bindings.has(name)) throw new Error(`unbound shell variable: ${name}`);
    return bindings.get(name);
  });
  return expanded.split("\\$").join("$");
}

// Scanner rather than a pattern match: `${{ … }}` and `${…}` share a prefix, so
// the passes have to run in order and must not overlap.
function substitute(text, open, close, resolve) {
  let result = "";
  let cursor = 0;
  for (;;) {
    const start = text.indexOf(open, cursor);
    if (start < 0) return result + text.slice(cursor);
    const end = text.indexOf(close, start + open.length);
    if (end < 0) return result + text.slice(cursor);
    result += text.slice(cursor, start) + resolve(text.slice(start + open.length, end).trim());
    cursor = end + close.length;
  }
}

// Value of the first double-quoted argument on the first line starting with
// `prefix`, e.g. `dist/$stage` from `Compress-Archive -Path "dist/$stage" …`.
function quotedValueAfter(step, prefix) {
  const line = step
    .split("\n")
    .map((candidate) => candidate.trim())
    .find((candidate) => candidate.startsWith(prefix));
  if (line === undefined) throw new Error(`missing workflow assignment: ${prefix}`);
  const value = line.slice(prefix.length);
  return value.slice(0, value.indexOf('"'));
}

function releaseBuildsPlatformSpecificVsixArtifacts() {
  for (const target of ["linux-x64", "linux-arm64", "darwin-x64", "darwin-arm64", "win32-x64"]) {
    assertIncludes(workflow, `vsix_target: ${target}`, `release workflow must include VSIX target ${target}`);
  }
  assertIncludes(
    workflow,
    "--target ${{ matrix.vsix_target }}",
    "release workflow must call the VS Code platform-specific package target",
  );
  assertIncludes(
    workflow,
    "deslop-live-${{ needs.version.outputs.version }}-${{ matrix.vsix_target }}.vsix",
    "release VSIX filenames must include the release version and platform target",
  );
  assertAbsent(
    /Flatten vsix-bin-/,
    "release workflow must not build one combined VSIX from every platform's binaries",
  );
}

function pagesDeployCleansRerunArtifactsAndRetries() {
  const deployPagesJob = sectionBetween("  deploy-pages:", "  publish-homebrew:");
  assertIncludes(
    deployPagesJob,
    "actions: write",
    "release workflow must let the called Pages workflow delete stale github-pages artifacts on rerun",
  );
  assertIncludes(
    deployWorkflow,
    "actions: write",
    "Pages deploy workflow must grant actions:write so stale github-pages artifacts can be deleted",
  );
  assertIncludes(
    deployWorkflow,
    "Remove stale Pages artifact from rerun",
    "Pages deploy workflow must remove stale github-pages artifacts before uploading a new one",
  );
  assertIncludes(
    deployWorkflow,
    'select(.name == "github-pages")',
    "Pages deploy workflow must delete only the Pages artifact from the current run",
  );
  // Matched on the action, not a pinned major: a Dependabot bump to the deploy
  // action is routine and must not read as the retry step going missing. Both
  // uses still have to agree, so a bump can never land on one step and leave the
  // retry a major behind — which would silently retry against different
  // behaviour than the attempt that failed.
  assertUniformRef(
    deployWorkflow,
    "- uses: actions/deploy-pages@",
    2,
    "Pages deploy workflow must retry a transient deploy-pages failure in the same job, on the same action version",
  );
  assertIncludes(
    deployWorkflow,
    "steps.deploy.outcome == 'failure'",
    "Pages deploy retry must be gated by the first deploy-pages outcome",
  );
  assertIncludes(
    deployWorkflow,
    "steps.deploy_retry.outcome == 'failure'",
    "Pages deploy workflow must fail only when the retry also fails",
  );
}

// The sweep is event-driven, and its base filter is what keeps it invisible to
// humans. A job-level `if:` does not make a job disappear — GitHub still
// materialises it as a check run with conclusion `skipped` — so subscribing to
// pull requests against `main` hangs a dead check on every human PR, one that
// by construction can never run. Filtering the base to the staging branch means
// the workflow is never instantiated for a human PR at all. ([GITHUB-DEPENDABOT])
function dependabotSweepLeavesNoDeadCheckOnHumanPullRequests() {
  const triggers = sectionBetween("\non:\n", "\npermissions:", dependabotWorkflow);
  assertExcludes(
    triggers,
    "main",
    "the sweep must not subscribe to pull requests against main: its job is actor-gated, and an if:-skipped job still reports a skipped check on every human PR",
  );
  assertIncludes(
    triggers,
    "- dependabot-upgrades",
    "the sweep must still fire on bumps opened against the staging branch, which is where every ecosystem in .github/dependabot.yml targets them",
  );
  assertExcludes(
    triggers,
    "pull_request_target",
    "pull_request_target would hand the write token and secrets to PR-controlled content, turning the merge bot into an exfiltration sink",
  );
  assertIncludes(
    dependabotWorkflow,
    "github.actor == 'dependabot[bot]'",
    "the sweep must still refuse to act for any actor but Dependabot",
  );
  assertIncludes(
    dependabotWorkflow,
    "startsWith(github.head_ref, 'dependabot/')",
    "the sweep must still require a dependabot/* source branch — the second half of the actor-AND-source gate",
  );
}

// The contract suites compare against `\n`-anchored literals, so a CRLF checkout
// turns every one of those assertions into a silent miss — the suite fails with
// "missing workflow section" on a file that is perfectly intact. Asserted through
// `git check-attr` rather than by reading .gitattributes, so what is verified is
// the attribute git actually resolves for the path, not the text of a rule that
// might not match it. ([DEPLOY-CI-GATES])
function contractFilesCheckOutLfOnEveryPlatform() {
  const result = spawnSync("git", ["check-attr", "eol", "--", ...LINE_MATCHED_FILES], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) throw new Error(`git check-attr failed: ${result.stderr ?? ""}`);

  const undeclared = result.stdout
    .trim()
    .split("\n")
    .filter((line) => line.slice(line.lastIndexOf(":") + 1).trim() !== "lf");
  if (undeclared.length > 0) {
    throw new Error(
      `these files must be declared eol=lf or exact line matching breaks on a Windows checkout:\n  ${undeclared.join("\n  ")}`,
    );
  }
}

function assertAbsent(pattern, message) {
  if (pattern.test(workflow)) throw new Error(message);
}

function assertExcludes(value, unexpected, message) {
  if (value.includes(unexpected)) throw new Error(message);
}

function assertSectionAbsent(section, pattern, message) {
  if (pattern.test(section)) throw new Error(message);
}

function assertPresent(pattern, message) {
  if (!pattern.test(workflow)) throw new Error(message);
}

function assertIncludes(value, expected, message) {
  if (!value.includes(expected)) throw new Error(message);
}

function assertEqual(actual, expected, message) {
  if (actual !== expected) throw new Error(`${message}; expected ${expected}, got ${actual}`);
}

// Asserts `prefix` introduces exactly `count` steps AND that every one pins the
// same ref. Counting alone would pass a workflow whose retry step drifted a
// major behind the attempt it retries.
function assertUniformRef(value, prefix, count, message) {
  const refs = value
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.startsWith(prefix))
    .map((line) => line.slice(prefix.length).split(/\s/u, 1)[0]);
  if (refs.length !== count) throw new Error(`${message}; found ${refs.length}`);
  const distinct = [...new Set(refs)];
  if (distinct.length !== 1) {
    throw new Error(`${message}; refs diverge: ${distinct.join(", ")}`);
  }
}

function sectionBetween(start, end, source = workflow) {
  const startIndex = source.indexOf(start);
  if (startIndex < 0) throw new Error(`missing workflow section ${start}`);
  const endIndex = source.indexOf(end, startIndex + start.length);
  if (endIndex < 0) throw new Error(`missing workflow section ${end}`);
  return source.slice(startIndex, endIndex);
}
