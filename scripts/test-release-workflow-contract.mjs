// Release workflow contract tests for the Shipwright deployment path.
//
// These are intentionally focused on the workflow, not the verifier script:
// the tests fail when the tagged source and published artifacts can drift, or
// when package-manager manifests declare binaries missing from the archives.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const repoRoot = resolve(new URL("..", import.meta.url).pathname);
const workflowPath = resolve(repoRoot, ".github/workflows/release.yml");
const workflow = readFileSync(workflowPath, "utf8");

const tests = [
  releaseBuildsTaggedSourceWithoutPostTagVersionCommit,
  releaseArchivesContainPackageManagerDeclaredBinaries,
  releaseBuildsPlatformSpecificVsixArtifacts,
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

function assertAbsent(pattern, message) {
  if (pattern.test(workflow)) throw new Error(message);
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

function sectionBetween(start, end) {
  const startIndex = workflow.indexOf(start);
  if (startIndex < 0) throw new Error(`missing workflow section ${start}`);
  const endIndex = workflow.indexOf(end, startIndex + start.length);
  if (endIndex < 0) throw new Error(`missing workflow section ${end}`);
  return workflow.slice(startIndex, endIndex);
}
