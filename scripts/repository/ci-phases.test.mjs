// [CI-COVERAGE-ONCE] Drive the real Makefile orchestration with bounded stage probes.
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { repoRoot } from "../lib/repo-root.mjs";

const STAGES = ["_ci-build", "_ci-gate", "_ci-test", "_coverage-report", "_vsix-coverage-check"];
const SUCCESS = 0;
const TIMEOUT_MS = 30_000;
const MAKEFILE = readFileSync(join(repoRoot, "Makefile"), "utf8");

function runStages(failedStage = "") {
  const directory = mkdtempSync(join(tmpdir(), "deslop-ci-phases-"));
  try {
    const probes = STAGES.map(stage => `${stage}:\n\t@echo ${stage} >> stages\n\t@test '${stage}' != '${failedStage}'`).join("\n");
    writeFileSync(join(directory, "Makefile"), `${MAKEFILE}\n${probes}\n`);
    const result = spawnSync("make", ["--no-print-directory", "ci"], {
      cwd: directory, encoding: "utf8", timeout: TIMEOUT_MS,
    });
    assert.ifError(result.error);
    return { status: result.status, stages: readFileSync(join(directory, "stages"), "utf8").trim().split("\n") };
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

test("[CI-COVERAGE-ONCE] local CI enforces coverage after collection and propagates every failure", () => {
  const successful = runStages();
  assert.equal(successful.status, SUCCESS);
  assert.deepEqual(successful.stages, STAGES, "CI must enforce both coverage thresholds after all collectors finish");
  for (const [index, stage] of STAGES.entries()) {
    const failed = runStages(stage);
    assert.notEqual(failed.status, SUCCESS, `${stage} failure must fail CI`);
    assert.deepEqual(failed.stages, STAGES.slice(0, index + 1), `${stage} failure must stop later phases`);
  }
});
