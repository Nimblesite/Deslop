// [VSIX-TESTING-COVERAGE-RESTORE] Exercise the real runner with tool-boundary probes.
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { vsixRoot } from "./coverage-paths.mjs";

const TIMEOUT_MS = 30_000;
const TOOL_FAILURE = 7;
const SUCCESS = 0;
const FAILURE_PHASES = ["build", "suite", "mapping"];
const SCRIPTS = ["webview-coverage.mjs", "coverage-paths.mjs"];
const PROBE = `
import childProcess from 'node:child_process';
import { syncBuiltinESMExports } from 'node:module';
import { appendFileSync } from 'node:fs';
childProcess.spawnSync = (tool, args) => {
  if (!['npm', 'npx'].includes(tool)) throw new Error('Unexpected tool: ' + tool);
  const phase = tool === 'npx' ? 'suite' : args.includes('--coverage') ? 'build' : 'restore';
  appendFileSync(process.env.CALL_LOG, phase + '\\n');
  const failed = phase === process.env.FAILURE_PHASE ||
    (phase === 'restore' && process.env.RESTORE_FAIL === 'yes');
  return { status: failed ? ${TOOL_FAILURE} : ${SUCCESS} };
};
syncBuiltinESMExports();
`;

function runFailure(phase, restoreFails) {
  const root = mkdtempSync(join(vsixRoot, ".tmp-webview-coverage-"));
  try {
    const scripts = join(root, "scripts");
    mkdirSync(scripts);
    for (const name of SCRIPTS) cpSync(join(vsixRoot, "scripts", name), join(scripts, name));
    const probe = join(root, "probe.mjs");
    writeFileSync(probe, PROBE);
    const log = join(root, "calls.log");
    const run = spawnSync(process.execPath, ["--import", probe, join(scripts, SCRIPTS[0])], {
      encoding: "utf8", timeout: TIMEOUT_MS,
      env: { ...process.env, CALL_LOG: log, FAILURE_PHASE: phase, RESTORE_FAIL: restoreFails ? "yes" : "no" },
    });
    assert.ifError(run.error);
    return { ...run, calls: readFileSync(log, "utf8").trim().split("\n") };
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

test("webview collection failures always restore the production bundle and retain restore failures", () => {
  for (const phase of FAILURE_PHASES) {
    for (const restoreFails of [false, true]) {
      const run = runFailure(phase, restoreFails);
      const expected = phase === "build" ? ["build", "restore"] : ["build", "suite", "restore"];
      assert.deepEqual(run.calls, expected, `${phase}: production restore must execute after failure`);
      assert.notEqual(run.status, SUCCESS, `${phase}: failed collection must fail the command`);
      assert.ok(run.stderr.length > 0, `${phase}: explain the failure`);
      if (restoreFails) {
        assert.match(run.stderr, /still staged/, "retain the additional restore failure");
        assert.ok(run.stderr.includes("media/webview"), "name the bundle left instrumented");
      }
    }
  }
});
