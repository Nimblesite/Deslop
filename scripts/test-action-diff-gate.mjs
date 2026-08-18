// Branch-executed black-box proof of the action's diff-gate path
// ([CLI-ARG-DIFF], [METRICS-DIFF-SCOPE], [ACTION-GATE]).
//
// The self-test workflow's `diff-gate` job installs the newest *published*
// release, so it stays skipped until one carries `--diff`/`--only-changed` —
// which leaves the pre-release action path with static contract checks and no
// execution behind it. This runs the same path against the branch-built CLI:
// the action's own `Run deslop` step body is extracted from action.yml and
// executed verbatim under the env the action sets, then the real
// `action-read-outputs.mjs` publishes the outputs and the real breach message
// is composed from them. Nothing here re-implements the action — a drift in
// argument forwarding or gate rerouting fails here the same way it would on a
// runner.
//
// Usage: node scripts/test-action-diff-gate.mjs [path/to/deslop]

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

import { check, total } from "./action-contract-harness.mjs";
import { readOutputs } from "./action-read-outputs.mjs";

/** The legacy-heavy fixture the self-test's gate matrix scans. */
const SCAN_PATH = "examples/rust";
/** Where the CLI lands under `make build`. */
const DEFAULT_CLI = "target/release/deslop";

/**
 * Extracts a composite step's `run:` body from action.yml, dedented to
 * column zero so bash reads it as a script.
 *
 * @param {string} action the action.yml source
 * @param {string} stepName the `- name:` value of the step to extract
 * @returns {string} the step's shell body
 */
function stepBody(action, stepName) {
  const lines = action.split("\n");
  const start = lines.findIndex((line) => line.trim() === `- name: ${stepName}`);
  assert.ok(start >= 0, `action.yml lost its "${stepName}" step`);
  const runAt = lines.findIndex((line, index) => index > start && line.trim() === "run: |");
  assert.ok(runAt > start, `action.yml step "${stepName}" no longer carries a run block`);
  const indent = (lines[runAt].match(/^ */) ?? [""])[0].length + 2;
  const body = [];
  for (const line of lines.slice(runAt + 1)) {
    if (line.trim() !== "" && (line.match(/^ */) ?? [""])[0].length < indent) break;
    body.push(line.slice(indent));
  }
  return body.join("\n");
}

/**
 * Runs the action's `Run deslop` step body verbatim under the env the
 * action composes from its inputs, with the branch-built CLI on PATH.
 *
 * @param {string} cli absolute path to the branch-built deslop binary
 * @param {Record<string, string>} inputs the action inputs under test
 * @returns {{exitCode: number, outputPrefix: string}}
 */
function runActionStep(cli, inputs) {
  const workdir = mkdtempSync(join(tmpdir(), "deslop-action-diff-"));
  const outputPrefix = join(workdir, "report");
  const githubOutput = join(workdir, "github-output");
  writeFileSync(githubOutput, "");
  const body = stepBody(readFileSync("action.yml", "utf8"), "Run deslop");
  execFileSync("bash", ["-eo", "pipefail", "-c", body], {
    stdio: "pipe",
    env: {
      ...process.env,
      PATH: `${resolve(cli, "..")}:${process.env.PATH ?? ""}`,
      GITHUB_OUTPUT: githubOutput,
      SCAN_PATH: SCAN_PATH,
      MIN_NODES: "30",
      OUTPUT: outputPrefix,
      LOG_LEVEL: "warn",
      FAIL_OVER: inputs.failOver,
      NO_FAIL_OVER: "false",
      CONFIG: "",
      DIFF: inputs.diff,
      ONLY_CHANGED: inputs.onlyChanged,
      NOJSON: "false",
      NOTEXT: "false",
      NOHTML: "false",
    },
  });
  const published = readFileSync(githubOutput, "utf8");
  const match = published.match(/exit-code=(\d+)/);
  assert.ok(match, `the step published no exit-code: ${published}`);
  return { exitCode: Number.parseInt(match[1], 10), outputPrefix };
}

/**
 * Writes a patch that adds a verbatim copy of an existing fixture file,
 * so the diff introduces duplication the scan is guaranteed to find.
 *
 * @param {string} sourceRelative fixture file to copy, relative to the repo root
 * @param {string} targetRelative the new file the patch adds
 * @returns {string} path to the written patch
 */
function writeCopyPatch(sourceRelative, targetRelative) {
  const lines = readFileSync(sourceRelative, "utf8").split("\n");
  const trailingNewline = lines.at(-1) === "";
  const body = trailingNewline ? lines.slice(0, -1) : lines;
  const hunk = [
    `diff --git a/${targetRelative} b/${targetRelative}`,
    "new file mode 100644",
    "--- /dev/null",
    `+++ b/${targetRelative}`,
    `@@ -0,0 +1,${body.length} @@`,
    ...body.map((line) => `+${line}`),
    ...(trailingNewline ? [] : ["\\ No newline at end of file"]),
    "",
  ].join("\n");
  const patch = join(mkdtempSync(join(tmpdir(), "deslop-action-patch-")), "change.patch");
  writeFileSync(patch, hunk);
  writeFileSync(targetRelative, readFileSync(sourceRelative));
  return patch;
}

/**
 * Composes the action's breach message for a gated run, from the same
 * outputs the `Gate on the result` step reads ([ACTION-GATE]).
 *
 * @param {Record<string, string>} outputs the published action outputs
 * @returns {string}
 */
function breachMessage(outputs) {
  const population = outputs["gate-scope"] === "added-lines" ? "added lines" : "analyzed lines";
  return `Deslop gate failed — ${outputs["gate-percent"]}% of ${population} are duplicated, ceiling is ${outputs["gate-threshold-percent"]}%.`;
}

const cli = resolve(process.argv[2] ?? DEFAULT_CLI);
assert.ok(
  existsSync(cli),
  `no deslop binary at ${cli} — run \`make build\` first, or pass the path as an argument`,
);
const emptyPatch = join(mkdtempSync(join(tmpdir(), "deslop-action-empty-")), "empty.patch");
writeFileSync(emptyPatch, "");

check("legacy debt passes a zero ceiling when the diff adds nothing", () => {
  const { exitCode, outputPrefix } = runActionStep(cli, {
    failOver: "0",
    diff: emptyPatch,
    onlyChanged: "true",
  });
  assert.equal(exitCode, 0, "an empty diff introduces no duplication, so the gate passes");
  const outputs = readOutputs(outputPrefix, exitCode, true);
  assert.equal(outputs["gate-scope"], "added-lines", "the diff gate governed, so the scope is the added lines");
  assert.equal(outputs["gate-percent"], "0", "no added lines means a 0% diff-scoped percentage");
  assert.equal(outputs["gate-threshold-percent"], "0", "the ceiling came from the fail-over input");
  const report = JSON.parse(readFileSync(`${outputPrefix}.json`, "utf8"));
  assert.equal(
    report.metrics.threshold.breached,
    true,
    "the fixture must stay legacy-heavy — the repo-wide verdict is breached even though the gate passed",
  );
  assert.equal(report.clusters.length, 0, "an empty diff touches no cluster");
  assert.ok(report.clusters_outside_diff >= 1, "the omitted legacy clusters are counted, never hidden");
  assert.equal(
    report.metrics.clusters_total,
    report.clusters.length,
    "the banner counts the body it sits above ([METRICS-REPO])",
  );
});

check("a changed-code diff that adds duplication breaches the same ceiling", () => {
  const source = firstRustFile();
  const copied = join(dirname(source), "action_diff_gate_copy.rs");
  const patch = writeCopyPatch(source, copied);
  try {
    const { exitCode, outputPrefix } = runActionStep(cli, {
      failOver: "0",
      diff: patch,
      onlyChanged: "true",
    });
    assert.equal(exitCode, 3, "the copied file duplicates its source, so the diff gate breaches");
    const outputs = readOutputs(outputPrefix, exitCode, true);
    assert.equal(outputs["exit-code"], "3");
    assert.equal(outputs["gate-scope"], "added-lines");
    assert.ok(
      Number.parseFloat(outputs["gate-percent"]) > 0,
      `the added lines must measure as duplicated: ${outputs["gate-percent"]}%`,
    );
    assert.match(
      breachMessage(outputs),
      /% of added lines are duplicated/,
      "the breach message must name the added-lines population, never the repo-wide one",
    );
    const report = JSON.parse(readFileSync(`${outputPrefix}.json`, "utf8"));
    assert.ok(report.clusters.length >= 1, "the newly copied clone survives --only-changed");
    assert.equal(
      report.metrics.clusters_total,
      report.clusters.length,
      "the banner counts the body it sits above ([METRICS-REPO])",
    );
    assert.equal(
      report.metrics.diff.threshold.source,
      "cli",
      "the report records that the diff gate governed",
    );
  } finally {
    execFileSync("rm", ["-f", copied]);
  }
});

/**
 * First Rust source under the fixture tree, for the copy patch — keeps
 * the proof working if the fixture layout is reorganised.
 *
 * @returns {string}
 */
function firstRustFile() {
  const found = execFileSync("find", [SCAN_PATH, "-name", "*.rs", "-type", "f"], { encoding: "utf8" })
    .split("\n")
    .filter(Boolean)
    .sort();
  assert.ok(found[0], `no Rust source under ${SCAN_PATH} to build a copy patch from`);
  return found[0];
}

console.log(`action diff-gate black-box: ${total()} checks passed`);
