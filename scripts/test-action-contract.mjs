// Proof suite for the GitHub Marketplace action. [ACTION-TESTS]
//
// Covers the pieces a hosted runner cannot cheaply prove on every PR: the
// runner -> release-asset mapping, version derivation from `github.action_ref`,
// checksum rejection, report-output extraction, and the static shape of
// action.yml. The runner-side behaviour (download, extract, gate) is proven
// end-to-end by .github/workflows/action-selftest.yml.

import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import assert from "node:assert/strict";

import { releaseArtifact, resolveRelease, resolveVersion } from "./action-resolve-artifact.mjs";
import { expectedDigest, verifyChecksum } from "./action-verify-checksum.mjs";
import { readOutputs } from "./action-read-outputs.mjs";

const scratch = mkdtempSync(join(tmpdir(), "deslop-action-"));
let checked = 0;

function check(label, body) {
  body();
  checked += 1;
  console.log(`  ok  ${label}`);
}

function expectThrows(label, body, needle) {
  check(label, () => {
    assert.throws(body, (error) => {
      assert.ok(
        error.message.includes(needle),
        `expected the error to mention "${needle}", got "${error.message}"`,
      );
      return true;
    });
  });
}

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
});

check("a breached run still publishes its measurements", () => {
  assert.equal(readOutputs(reportPrefix, 3)["duplication-percent"], "12.15");
});

check("a usage error reports only its exit code", () => {
  const outputs = readOutputs(join(scratch, "absent"), 2);
  assert.deepEqual(outputs, { "exit-code": "2" });
});

expectThrows(
  "a clean run that rendered no report is a hard error, not an empty output",
  () => readOutputs(join(scratch, "absent"), 0),
  "rendered no JSON report",
);

// --- action.yml static shape -------------------------------------------------

const action = readFileSync("action.yml", "utf8");

check("action.yml declares the Marketplace-required metadata", () => {
  for (const field of ["name: Deslop", "description:", "author: Nimblesite", "using: composite"]) {
    assert.ok(action.includes(field), `action.yml is missing ${field}`);
  }
  assert.ok(action.includes("icon: copy"), "branding icon must be a Feather icon name");
  assert.ok(action.includes("color: purple"), "branding colour must be one of the nine allowed values");
});

check("every documented input is declared", () => {
  const inputs = [
    "path", "version", "fail-over", "no-fail-over", "min-nodes", "config",
    "output", "nojson", "notext", "nohtml", "log-level", "upload-artifact", "artifact-name",
  ];
  for (const input of inputs) assert.ok(action.includes(`\n  ${input}:`), `action.yml lost input ${input}`);
});

check("every output is wired to the report step", () => {
  const outputs = [
    "duplication-percent", "cluster-count", "threshold-percent",
    "exit-code", "report-json", "report-text", "report-html",
  ];
  for (const output of outputs) {
    assert.ok(
      action.includes(`value: \${{ steps.report.outputs.${output} }}`),
      `action.yml output ${output} is not wired to the report step`,
    );
  }
});

check("the nested action is pinned to a full-length commit SHA", () => {
  const marker = "actions/upload-artifact@";
  const start = action.indexOf(marker) + marker.length;
  assert.ok(start > marker.length - 1, "action.yml no longer uploads the reports");
  const pin = action.slice(start).split(" ")[0].trim();
  assert.equal(pin.length, 40, `upload-artifact must be pinned to a 40-character SHA, found "${pin}"`);
  assert.ok(
    [...pin].every((character) => "0123456789abcdef".includes(character)),
    `upload-artifact pin "${pin}" is not hexadecimal`,
  );
});

check("the gate re-raises the CLI status rather than swallowing it", () => {
  assert.ok(action.includes('exit "${EXIT_CODE}"'), "the gate step must re-raise the CLI exit code");
  assert.ok(
    action.includes("if: steps.run.outputs.exit-code != '0'"),
    "the gate step must run for every non-zero status",
  );
});

// GitHub injects `-e` into composite `shell: bash` steps, so a bare `deslop`
// invocation would abort the run step on a breach before the status reaches
// GITHUB_OUTPUT — leaving every output empty and skipping the report, upload,
// and gate steps. [ACTION-GATE]
check("a breach cannot abort the run step before the status is captured", () => {
  assert.ok(
    action.includes('deslop "${args[@]}" || status=$?'),
    "the run step must capture the CLI status with an errexit-proof || guard",
  );
  assert.ok(
    action.includes('echo "exit-code=${status}" >> "${GITHUB_OUTPUT}"'),
    "the captured status must be written to GITHUB_OUTPUT",
  );
  assert.ok(
    !action.includes('echo "exit-code=$?"'),
    "the status must come from the guard variable, not $? after a guarded call",
  );
});

console.log(`\naction contract: ${checked} checks passed`);
