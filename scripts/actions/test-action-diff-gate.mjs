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
// Usage: node scripts/actions/test-action-diff-gate.mjs [path/to/deslop]

import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

import { check, total } from "../lib/contract-harness.mjs";
import { firstRustFile, writeCopyPatch } from "./action-copy-patch.mjs";
import { readOutputs } from "./action-read-outputs.mjs";
import { stepBody } from "./action-yaml.mjs";
import { posixShell, shellPath } from "../lib/posix-shell.mjs";
import { currentPlatform, executableName } from "../release/vsix-platforms.mjs";

/** The legacy-heavy fixture the self-test's gate matrix scans. */
const SCAN_PATH = "examples/rust";
/** Where the CLI lands under `make build`, spelled the way this host spells it. */
const DEFAULT_CLI = `target/release/${executableName("deslop", currentPlatform())}`;
/** The guard step whose emitted guidance is executed below. */
const STDIN_GUARD_STEP = "Reject the un-suppliable stdin diff";
/** The status the CLI exits on a usage error, and the guard must match. */
const USAGE_ERROR_STATUS = 2;
/** The base ref a `pull_request` event supplies. */
const PULL_REQUEST_BASE_REF = "main";

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
  // The step calls `deslop` by name, so the built binary has to be on the
  // PATH the step sees. Prepending it inside the shell — from a positional
  // argument, spelled the way that shell spells a path — is the only form
  // that works on both: a Windows directory handed in through `env` carries
  // the character PATH separates on, and the shell reads one entry as two.
  execFileSync(posixShell(), ["-eo", "pipefail", "-c", `PATH="$1:$PATH"
${body}`, "run-deslop", shellPath(dirname(cli))], {
    stdio: "pipe",
    env: {
      ...process.env,
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


/**
 * Runs a guard step's body verbatim under bash with `env`, the way the runner
 * does. Guards exit non-zero by design, so the status is returned rather than
 * thrown.
 *
 * @param {string} stepName the `- name:` value of the step to run
 * @param {Record<string, string>} env the env block the action composes
 * @returns {{status: number | null, output: string}}
 */
function runGuardStep(stepName, env) {
  const body = stepBody(readFileSync("action.yml", "utf8"), stepName);
  const result = spawnSync(posixShell(), ["-eo", "pipefail", "-c", body], {
    encoding: "utf8",
    env: { ...process.env, ...env },
  });
  return { status: result.status, output: `${result.stdout}${result.stderr}` };
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
  const source = firstRustFile(SCAN_PATH);
  const copied = join(dirname(source), "action_diff_gate_copy.rs");
  const patch = writeCopyPatch(
    source,
    copied,
    join(mkdtempSync(join(tmpdir(), "deslop-action-patch-")), "change.patch"),
  );
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
    rmSync(copied, { force: true });
  }
});

// [ACTION-GATE] The guard's whole job is to hand back a command that gets the
// caller unstuck. It used to build that command by interpolating
// `${{ github.base_ref }}` into the shell body, behind a backslash: GitHub
// substitutes the expression before bash parses the step, so an ordinary
// `main` base printed the unusable ref `origin/\main`, and a base carrying
// shell-significant text was only escaped up to its first character. The ref
// now arrives through `env` and is printed as a `printf` argument, so these
// run it and read what a caller would actually see.
check("the stdin-diff guard prints a runnable command for the event's base ref", () => {
  const { status, output } = runGuardStep(STDIN_GUARD_STEP, { BASE_REF: PULL_REQUEST_BASE_REF });
  assert.equal(status, USAGE_ERROR_STATUS, `the guard must exit ${USAGE_ERROR_STATUS}: ${output}`);
  assert.ok(
    output.includes(`git diff origin/${PULL_REQUEST_BASE_REF}...HEAD > /tmp/pr.diff`),
    `the guidance must be a command the caller can paste: ${output}`,
  );
  assert.ok(
    !output.includes("\\"),
    `a stray escape makes the suggested ref invalid — origin/\\main resolves to nothing: ${output}`,
  );
});

check("a push event with no base ref still gets a runnable command", () => {
  const { output } = runGuardStep(STDIN_GUARD_STEP, { BASE_REF: "" });
  assert.ok(
    output.includes(`git diff origin/${PULL_REQUEST_BASE_REF}...HEAD > /tmp/pr.diff`),
    `an absent base ref must fall back to a real branch, never print an empty ref: ${output}`,
  );
});

check("shell-significant text in the base ref is printed as data, never executed", () => {
  const marker = join(mkdtempSync(join(tmpdir(), "deslop-action-injection-")), "executed");
  const hostile = `main"; touch ${marker}; echo "`;
  const { output } = runGuardStep(STDIN_GUARD_STEP, { BASE_REF: hostile });
  assert.ok(!existsSync(marker), `the base ref reached the shell as code — it wrote ${marker}`);
  assert.ok(output.includes(hostile), `the ref must survive intact as data in the guidance: ${output}`);
});

console.log(`action diff-gate black-box: ${total()} checks passed`);
