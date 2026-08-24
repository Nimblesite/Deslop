// Contract checks for the action's helper scripts. [ACTION-TESTS]
//
// Drives the .mjs scripts the composite action runs: the runner ->
// release-asset mapping, version derivation from `github.action_ref`, checksum
// rejection, and report-output extraction. Imported for its side effects by
// test-action-contract.mjs, which prints the suite total.

import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import assert from "node:assert/strict";

import { check, expectThrows } from "../lib/contract-harness.mjs";
import { releaseArtifact, resolveRelease, resolveVersion } from "./action-resolve-artifact.mjs";
import { expectedDigest, verifyChecksum } from "./action-verify-checksum.mjs";
import { readOutputs } from "./action-read-outputs.mjs";

const scratch = mkdtempSync(join(tmpdir(), "deslop-action-"));

// --- runner -> release asset -------------------------------------------------
// The five names must stay identical to `matrix.artifact_name` in
// .github/workflows/release.yml, or the action downloads a 404.

check("every published runner maps to its release asset", () => {
  assert.equal(releaseArtifact("Linux", "X64"), "linux-x64");
  assert.equal(releaseArtifact("Linux", "ARM64"), "linux-arm64");
  assert.equal(releaseArtifact("macOS", "X64"), "macos-x64");
  assert.equal(releaseArtifact("macOS", "ARM64"), "macos-arm64");
  assert.equal(releaseArtifact("Windows", "X64"), "windows-x64");
});

expectThrows(
  "an unpublished runner names the offending pair",
  () => releaseArtifact("Windows", "ARM64"),
  "unsupported runner Windows/ARM64",
);

check("the release workflow still publishes every asset this action requests", () => {
  const workflow = readFileSync(".github/workflows/release.yml", "utf8");
  for (const artifact of ["linux-x64", "linux-arm64", "macos-x64", "macos-arm64", "windows-x64"]) {
    assert.ok(
      workflow.includes(`artifact_name: ${artifact}`),
      `release.yml no longer publishes ${artifact}; the action would download a 404`,
    );
  }
});

// --- version derivation ------------------------------------------------------

check("a tag-pinned action installs the matching CLI version", () => {
  assert.equal(resolveVersion("", "v0.1.0"), "0.1.0");
  assert.equal(resolveVersion("", "v1.2.3-rc.1"), "1.2.3-rc.1");
});

check("an explicit version input overrides the pinned ref", () => {
  assert.equal(resolveVersion("0.2.0", "v0.1.0"), "0.2.0");
  assert.equal(resolveVersion("v0.2.0", "v0.1.0"), "0.2.0");
});

expectThrows(
  "a SHA-pinned action demands an explicit version instead of guessing latest",
  () => resolveVersion("", "8f4b7c2e9a1d3f5b6c8e0a2d4f6b8c0e2a4d6f80"),
  "set the `version` input explicitly",
);

expectThrows(
  "a branch-pinned action demands an explicit version",
  () => resolveVersion("", "main"),
  "cannot derive the deslop version",
);

// --- download coordinates ----------------------------------------------------
// Archive and staging-directory names must match release.yml's packaging step
// exactly: `deslop-<version>-<artifact>` staged inside `deslop-...tar.gz`.

check("unix coordinates match the packaged tarball", () => {
  const release = resolveRelease("Linux", "X64", "", "v0.1.0");
  assert.equal(release.archive, "deslop-0.1.0-linux-x64.tar.gz");
  assert.equal(release.stage, "deslop-0.1.0-linux-x64");
  assert.equal(
    release.url,
    "https://github.com/Nimblesite/Deslop/releases/download/v0.1.0/deslop-0.1.0-linux-x64.tar.gz",
  );
  assert.equal(release.checksumUrl, `${release.url}.sha256`);
});

check("windows coordinates match the packaged zip", () => {
  const release = resolveRelease("Windows", "X64", "0.1.0", "");
  assert.equal(release.archive, "deslop-0.1.0-windows-x64.zip");
  assert.equal(release.stage, "deslop-0.1.0-windows-x64");
});

// --- checksum verification ---------------------------------------------------

const archivePath = join(scratch, "deslop-0.1.0-linux-x64.tar.gz");
const checksumPath = `${archivePath}.sha256`;
writeFileSync(archivePath, "pretend archive bytes");
// sha256 of the line above, in the `<hash>  <filename>` form release.yml emits.
const trueDigest = "6c1b3f0e4fd6c1c0a9d2f3ba1c8e6e6a4a1f4b3a3e0f2f0f6f2d6b0b6d9a1c62";

check("a matching checksum is accepted", () => {
  const actual = createHash("sha256").update(readFileSync(archivePath)).digest("hex");
  writeFileSync(checksumPath, `${actual}  deslop-0.1.0-linux-x64.tar.gz\n`);
  assert.equal(verifyChecksum(archivePath, checksumPath), actual);
});

expectThrows(
  "a tampered archive is rejected before extraction",
  () => {
    writeFileSync(checksumPath, `${trueDigest}  deslop-0.1.0-linux-x64.tar.gz\n`);
    verifyChecksum(archivePath, checksumPath);
  },
  "checksum mismatch",
);

check("the sidecar's `<hash>  <file>` form is parsed to the hash alone", () => {
  assert.equal(expectedDigest(`${trueDigest}  deslop-0.1.0-linux-x64.tar.gz\n`), trueDigest);
  assert.equal(expectedDigest(`${trueDigest.toUpperCase()}\t x.tar.gz`), trueDigest);
});

expectThrows("an empty sidecar is rejected", () => expectedDigest("   \n"), "checksum file is empty");

// --- report outputs ----------------------------------------------------------

const reportPrefix = join(scratch, "deslop-report");
writeFileSync(
  `${reportPrefix}.json`,
  JSON.stringify({
    metrics: {
      duplication_percent: 12.15,
      clusters_total: 47,
      threshold: { percent: 12.2, breached: false, source: "config" },
    },
  }),
);

check("outputs carry the measured percentage, cluster count and ceiling", () => {
  const outputs = readOutputs(reportPrefix, 0);
  assert.equal(outputs["duplication-percent"], "12.15");
  assert.equal(outputs["cluster-count"], "47");
  assert.equal(outputs["threshold-percent"], "12.2");
  assert.equal(outputs["exit-code"], "0");
  assert.equal(outputs["report-html"], `${reportPrefix}.html`);
  assert.equal(outputs["gate-scope"], "repository", "a diff-less run is gated repo-wide");
  assert.equal(outputs["gate-percent"], "12.15");
  assert.equal(outputs["gate-threshold-percent"], "12.2");
});

check("a breached run still publishes its measurements", () => {
  assert.equal(readOutputs(reportPrefix, 3)["duplication-percent"], "12.15");
});

check("a usage error reports only its exit code", () => {
  const outputs = readOutputs(join(scratch, "absent"), 2);
  assert.deepEqual(outputs, { "exit-code": "2" });
});

// --- diff-scoped gate outputs ------------------------------------------------
// Under `--only-changed` the mechanical gate reads `metrics.diff` — duplicated
// added lines over added lines ([METRICS-DIFF-SCOPE]) — and the gate step's
// breach message names that population through `gate-scope`. [ACTION-GATE]

const diffReportPrefix = join(scratch, "deslop-diff-report");
writeFileSync(
  `${diffReportPrefix}.json`,
  JSON.stringify({
    metrics: {
      duplication_percent: 41.3,
      clusters_total: 12,
      threshold: { percent: 0, breached: true, source: "cli" },
      diff: {
        added_loc: 80,
        duplicated_added_loc: 12,
        duplication_percent: 15,
        threshold: { percent: 10, breached: true, source: "cli" },
      },
    },
  }),
);

check("a diff-gated run names the added-lines scope with the diff figures", () => {
  const outputs = readOutputs(diffReportPrefix, 3, true);
  assert.equal(outputs["gate-scope"], "added-lines");
  assert.equal(outputs["gate-percent"], "15");
  assert.equal(outputs["gate-threshold-percent"], "10");
  assert.equal(outputs["duplication-percent"], "41.3", "the repo-wide outputs stay repo-wide");
  assert.equal(outputs["threshold-percent"], "0");
  assert.equal(outputs["exit-code"], "3");
});

check("the report alone reroutes the scope when its diff threshold is live", () => {
  // metrics.diff.threshold.source != "none" records that the diff gate
  // governed ([METRICS-DIFF-SCOPE]); the scope must follow the report even
  // when the only-changed input echo is lost.
  const outputs = readOutputs(diffReportPrefix, 3, false);
  assert.equal(outputs["gate-scope"], "added-lines");
  assert.equal(outputs["gate-percent"], "15");
  assert.equal(outputs["gate-threshold-percent"], "10");
});

const taggedReportPrefix = join(scratch, "deslop-tagged-report");
writeFileSync(
  `${taggedReportPrefix}.json`,
  JSON.stringify({
    metrics: {
      duplication_percent: 41.3,
      clusters_total: 12,
      threshold: { percent: 50, breached: false, source: "config" },
      diff: {
        added_loc: 80,
        duplicated_added_loc: 12,
        duplication_percent: 15,
        threshold: { percent: 0, breached: false, source: "none" },
      },
    },
  }),
);

check("a tagging-only --diff run keeps the repository gate scope", () => {
  const outputs = readOutputs(taggedReportPrefix, 0, false);
  assert.equal(outputs["gate-scope"], "repository", "--diff alone never moves the gate");
  assert.equal(outputs["gate-percent"], "41.3");
  assert.equal(outputs["gate-threshold-percent"], "50");
});

expectThrows(
  "an only-changed run whose report lacks diff metrics is a hard error",
  () => readOutputs(reportPrefix, 3, true),
  "carries no metrics.diff duplication_percent",
);

expectThrows(
  "a clean run that rendered no report is a hard error, not an empty output",
  () => readOutputs(join(scratch, "absent"), 0),
  "rendered no JSON report",
);
