// Release workflow contract tests for the Shipwright deployment path.
//
// These are intentionally focused on the workflow, not the verifier script:
// the tests fail when the tagged source and published artifacts can drift, or
// when package-manager manifests declare binaries missing from the archives.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const workflowPath = resolve(repoRoot, ".github/workflows/release.yml");
const deployWorkflowPath = resolve(repoRoot, ".github/workflows/deploy-pages.yml");
const dependabotWorkflowPath = resolve(repoRoot, ".github/workflows/dependabot-automerge.yml");
const workflow = readFileSync(workflowPath, "utf8");
const deployWorkflow = readFileSync(deployWorkflowPath, "utf8");
const dependabotWorkflow = readFileSync(dependabotWorkflowPath, "utf8");

const tests = [
  releaseBuildsTaggedSourceWithoutPostTagVersionCommit,
  releaseArchivesContainPackageManagerDeclaredBinaries,
  releaseBuildsPlatformSpecificVsixArtifacts,
  pagesDeployCleansRerunArtifactsAndRetries,
  dependabotSweepLeavesNoDeadCheckOnHumanPullRequests,
];

let failed = 0;
for (const test of tests) {
  try {
    test();
    console.log(`ok ${test.name}`);
  } catch (error) {
    failed++;
    console.error(`not ok ${test.name}`);
    console.error(`  ${error instanceof Error ? error.message : String(error)}`);
  }
}

if (failed > 0) {
  console.error(`\n${failed} release workflow contract test(s) failed`);
  process.exit(1);
}
console.log(`\n${tests.length} release workflow contract tests passed`);

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
    /node scripts\/stamp-release-version\.mjs "\$\{\{ steps\.extract\.outputs\.version \}\}"/,
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
  assertOccurrenceCount(
    deployWorkflow,
    "actions/deploy-pages@v4",
    2,
    "Pages deploy workflow must retry a transient deploy-pages failure in the same job",
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

// A job-level `if:` does not make a job disappear — GitHub still materialises
// it as a check run with conclusion `skipped`. So a workflow that subscribes to
// every pull request and then filters for the Dependabot actor hangs a dead
// `skipped` check on every human PR to `main`, forever. The sweep must select
// its own work by polling instead of being handed a PR it is not allowed to
// touch. ([GITHUB-DEPENDABOT])
function dependabotSweepLeavesNoDeadCheckOnHumanPullRequests() {
  const triggers = sectionBetween("\non:\n", "\npermissions:", dependabotWorkflow);
  assertExcludes(
    triggers,
    "pull_request",
    "the sweep must not subscribe to pull requests at all: an actor-gated job still reports a skipped check on every human PR to main, and never naming the trigger also makes the pull_request_target foot-gun unreachable by construction",
  );
  assertIncludes(
    triggers,
    "schedule:",
    "dropping the pull-request trigger strands security bumps, which GitHub always opens against main, unless the sweep polls for them",
  );
  assertIncludes(
    triggers,
    "workflow_dispatch:",
    "a maintainer must be able to sweep immediately instead of waiting for the next cron tick",
  );
  assertIncludes(
    dependabotWorkflow,
    "--author 'app/dependabot'",
    "polling must still restrict the sweep to Dependabot-authored pull requests",
  );
  assertIncludes(
    dependabotWorkflow,
    'startswith("dependabot/")',
    "polling must still require a dependabot/* source branch — the second half of the actor-AND-source gate",
  );
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

function assertOccurrenceCount(value, expected, count, message) {
  const actual = value.split(expected).length - 1;
  if (actual !== count) throw new Error(`${message}; found ${actual}`);
}

function sectionBetween(start, end, source = workflow) {
  const startIndex = source.indexOf(start);
  if (startIndex < 0) throw new Error(`missing workflow section ${start}`);
  const endIndex = source.indexOf(end, startIndex + start.length);
  if (endIndex < 0) throw new Error(`missing workflow section ${end}`);
  return source.slice(startIndex, endIndex);
}
