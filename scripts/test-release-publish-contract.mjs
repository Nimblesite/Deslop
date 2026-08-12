// Release publish completeness contract ([DEPLOY-PUBLISH-COMPLETE], issue #348).
//
// v0.31.0 shipped from a run where one transient Marketplace timeout aborted
// the publish loop after 2 of 5 platform VSIXes were attempted: darwin-arm64
// got v0.31.0, darwin-x64 timed out, and linux-arm64/linux-x64/win32-x64 were
// never attempted at all — while Open VSX, whose loop happened to succeed,
// served all five. Until a manual re-run the two registries disagreed about
// what the current release was, and nothing in the run named the missing
// platforms.
//
// These tests execute the release workflow's own publish `run:` blocks in a
// sandbox: five fake platform VSIX artifacts, a stub `npx` scripted to time
// out the way the Marketplace did, and a stub `az` for the token mint. Both
// registries are held to the same contract:
//   - a persistently failing platform must not stop the other four from being
//     attempted, and the job must fail naming the platform that never
//     reached the registry;
//   - a transient timeout must be retried until the platform publishes, and
//     the job must then succeed reporting all five;
//   - every attempt stays idempotent via --skip-duplicate, so retries and
//     re-runs of a partially published tag are safe.

import { spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const workflow = readFileSync(resolve(repoRoot, ".github/workflows/release.yml"), "utf8");

const VSIX_TARGETS = ["darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64", "win32-x64"];
const TIMING_OUT_TARGET = "darwin-x64";
const TEST_VERSION = "9.9.9";
// A failure count no retry policy should ever exhaust: the platform never publishes.
const PERSISTENT = 1_000_000;

// Records every publish invocation, then fails the FAIL_TARGET package the
// first FAIL_TIMES times it is attempted — a scripted Marketplace timeout.
// Everything else publishes instantly.
const NPX_STUB = `#!/bin/sh
printf '%s\\n' "$*" >> "\${STUB_LOG}"
package=""
previous=""
for argument in "$@"; do
  if [ "\${previous}" = "--packagePath" ]; then package="\${argument}"; fi
  previous="\${argument}"
done
case "\${package}" in
  *"\${FAIL_TARGET}"*)
    attempts=$(grep -cF -- "\${package}" "\${STUB_LOG}")
    if [ "\${attempts}" -le "\${FAIL_TIMES}" ]; then
      echo "##[error]Request timeout: /_apis/gallery/publishers/nimblesite/extensions/deslop-live" >&2
      exit 1
    fi
    ;;
esac
printf 'stub published %s\\n' "\${package}"
`;

// Stands in for the workflow's \`az account get-access-token\` token mint; the
// publish step only needs a token-shaped string on stdout.
const AZ_STUB = `#!/bin/sh
echo "stub-access-token"
`;

const bash = resolveRunnerBash();
const marketplaceRunBlock = publishRunBlock("\n  publish-marketplace:", "\n  publish-openvsx:");
const openvsxRunBlock = publishRunBlock("\n  publish-openvsx:", null);

const tests = [
  marketplaceAttemptsEveryPlatformWhenOneKeepsTimingOut,
  marketplaceRetriesATransientTimeoutUntilThePlatformPublishes,
  openvsxAttemptsEveryPlatformWhenOneKeepsTimingOut,
  openvsxRetriesATransientTimeoutUntilThePlatformPublishes,
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
  console.error(`\n${failed} release publish contract test(s) failed`);
  process.exit(1);
}
console.log(`\n${tests.length} release publish contract tests passed`);

function marketplaceAttemptsEveryPlatformWhenOneKeepsTimingOut() {
  assertSurvivesPersistentTimeout(marketplaceScenario(PERSISTENT), "Marketplace");
}

function marketplaceRetriesATransientTimeoutUntilThePlatformPublishes() {
  assertRetriesTransientTimeout(marketplaceScenario(1), "Marketplace");
}

function openvsxAttemptsEveryPlatformWhenOneKeepsTimingOut() {
  assertSurvivesPersistentTimeout(openvsxScenario(PERSISTENT), "Open VSX");
}

function openvsxRetriesATransientTimeoutUntilThePlatformPublishes() {
  assertRetriesTransientTimeout(openvsxScenario(1), "Open VSX");
}

// One platform never reaches the registry no matter how often it is tried.
// The other four must still be attempted, the job must fail, and the failure
// must name the platform that is missing — "which OSes lost the release" was
// exactly the question the v0.31.0 run could not answer.
function assertSurvivesPersistentTimeout(scenario, registry) {
  for (const target of VSIX_TARGETS) {
    assert(
      (scenario.attempts[target] ?? 0) >= 1,
      `${registry}: ${target} was never attempted after ${TIMING_OUT_TARGET} kept timing out; ` +
        `one flaky platform must not suppress the others (attempts: ${JSON.stringify(scenario.attempts)})`,
    );
  }
  assert(
    scenario.status !== 0,
    `${registry}: a publish that leaves ${TIMING_OUT_TARGET} off the registry must fail the job`,
  );
  const namingLine = scenario.output
    .split("\n")
    .find((line) => /error/i.test(line) && line.includes(TIMING_OUT_TARGET));
  assert(
    namingLine !== undefined,
    `${registry}: the failure must name ${TIMING_OUT_TARGET} as the platform that never reached the registry`,
  );
  for (const invocation of scenario.invocations) {
    assert(
      invocation.includes("--skip-duplicate"),
      `${registry}: every publish attempt must stay idempotent via --skip-duplicate; got: ${invocation}`,
    );
  }
  assert(
    !scenario.output.includes(`Published ${VSIX_TARGETS.length} VSIX`),
    `${registry}: a partial publish must not claim all ${VSIX_TARGETS.length} platforms were published`,
  );
}

// The v0.31.0 timeout was transient — the manual re-run published the same
// VSIX without incident. One scripted failure followed by success must be
// absorbed by a retry: every platform publishes and the job succeeds.
function assertRetriesTransientTimeout(scenario, registry) {
  assert(
    (scenario.attempts[TIMING_OUT_TARGET] ?? 0) >= 2,
    `${registry}: one transient timeout on ${TIMING_OUT_TARGET} must be retried, not surrendered to ` +
      `(attempts: ${JSON.stringify(scenario.attempts)})`,
  );
  for (const target of VSIX_TARGETS) {
    assert(
      (scenario.attempts[target] ?? 0) >= 1,
      `${registry}: ${target} was never attempted (attempts: ${JSON.stringify(scenario.attempts)})`,
    );
  }
  assert(
    scenario.status === 0,
    `${registry}: after the retry publishes ${TIMING_OUT_TARGET} the job must succeed; ` +
      `exited ${scenario.status} with output:\n${scenario.output}`,
  );
  assert(
    scenario.output.includes(`Published ${VSIX_TARGETS.length} VSIX`),
    `${registry}: the job must report all ${VSIX_TARGETS.length} platform VSIXes published`,
  );
}

function marketplaceScenario(failTimes) {
  return runPublishStep({ runBlock: marketplaceRunBlock, failTimes, extraEnv: {} });
}

function openvsxScenario(failTimes) {
  return runPublishStep({
    runBlock: openvsxRunBlock,
    failTimes,
    extraEnv: { OVSX_PAT: "stub-openvsx-token" },
  });
}

// Executes a publish step's run block the way the runner does: cwd holding
// the downloaded artifacts/ tree, PATH resolving npx and az to the stubs, and
// the step's env populated. scripts/ is linked into the sandbox so the block
// may delegate to repository scripts. DESLOP_PUBLISH_BACKOFF_SECONDS=0 keeps
// any retry backoff out of the suite's runtime — tests must not sleep.
function runPublishStep({ runBlock, failTimes, extraEnv }) {
  const sandbox = mkdtempSync(join(tmpdir(), "deslop-publish-contract-"));
  try {
    for (const target of VSIX_TARGETS) {
      const artifactDir = join(sandbox, "artifacts", `vsix-${target}`);
      mkdirSync(artifactDir, { recursive: true });
      writeFileSync(join(artifactDir, `deslop-live-${TEST_VERSION}-${target}.vsix`), "stub vsix");
    }
    symlinkSync(resolve(repoRoot, "scripts"), join(sandbox, "scripts"));
    const stubBin = join(sandbox, "stub-bin");
    mkdirSync(stubBin);
    for (const [name, body] of [["npx", NPX_STUB], ["az", AZ_STUB]]) {
      writeFileSync(join(stubBin, name), body);
      chmodSync(join(stubBin, name), 0o755);
    }
    const log = join(sandbox, "publish-attempts.log");
    writeFileSync(log, "");

    const result = spawnSync(bash, ["-c", runBlock], {
      cwd: sandbox,
      encoding: "utf8",
      timeout: 120_000,
      env: {
        ...process.env,
        PATH: `${stubBin}:${process.env.PATH}`,
        STUB_LOG: log,
        FAIL_TARGET: TIMING_OUT_TARGET,
        FAIL_TIMES: String(failTimes),
        GITHUB_REF_NAME: `v${TEST_VERSION}`,
        DESLOP_PUBLISH_BACKOFF_SECONDS: "0",
        ...extraEnv,
      },
    });
    if (result.error) throw result.error;

    const invocations = readFileSync(log, "utf8")
      .split("\n")
      .filter((line) => line !== "");
    const attempts = Object.fromEntries(
      VSIX_TARGETS.map((target) => [
        target,
        invocations.filter((line) => line.includes(`-${target}.vsix`)).length,
      ]),
    );
    return {
      status: result.status,
      output: `${result.stdout ?? ""}${result.stderr ?? ""}`,
      attempts,
      invocations,
    };
  } finally {
    rmSync(sandbox, { recursive: true, force: true });
  }
}

// The dedented body of the publish step's `run: |` block inside the given job.
function publishRunBlock(jobHeader, nextJobHeader) {
  const job = sectionBetween(jobHeader, nextJobHeader);
  const stepIndex = job.indexOf("- name: Publish each platform VSIX");
  if (stepIndex < 0) throw new Error(`${jobHeader.trim()} has no "Publish each platform VSIX" step`);
  const marker = "run: |";
  const runIndex = job.indexOf(marker, stepIndex);
  if (runIndex < 0) throw new Error(`${jobHeader.trim()} publish step has no run block`);

  const lines = job.slice(runIndex + marker.length).split("\n").slice(1);
  const firstBodyLine = lines.find((line) => line.trim() !== "");
  if (firstBodyLine === undefined) throw new Error(`${jobHeader.trim()} publish run block is empty`);
  const indent = firstBodyLine.length - firstBodyLine.trimStart().length;

  const body = [];
  for (const line of lines) {
    if (line.trim() === "") {
      body.push("");
      continue;
    }
    if (line.length - line.trimStart().length < indent) break;
    body.push(line.slice(indent));
  }
  const script = body.join("\n");
  if (script.includes("${{")) {
    throw new Error(
      `${jobHeader.trim()} publish run block contains unexpanded \${{ }} expressions; the sandbox cannot execute it`,
    );
  }
  return script;
}

function sectionBetween(start, end) {
  const startIndex = workflow.indexOf(start);
  if (startIndex < 0) throw new Error(`missing workflow section ${start.trim()}`);
  if (end === null) return workflow.slice(startIndex);
  const endIndex = workflow.indexOf(end, startIndex + start.length);
  if (endIndex < 0) throw new Error(`missing workflow section ${end.trim()}`);
  return workflow.slice(startIndex, endIndex);
}

// The publish steps run on ubuntu-latest under GNU bash >= 5 and use
// `shopt -s globstar`, which macOS's bash 3.2 rejects — aborting the step for
// a reason the real runner never sees. Executing the steps faithfully
// therefore needs a bash that supports it.
function resolveRunnerBash() {
  const candidates = ["bash", "/opt/homebrew/bin/bash", "/usr/local/bin/bash", "/usr/bin/bash"];
  for (const candidate of candidates) {
    const probe = spawnSync(candidate, ["-c", "shopt -s globstar"], { encoding: "utf8" });
    if (probe.status === 0) return candidate;
  }
  throw new Error(
    "no bash with globstar support found; executing the release publish steps needs bash >= 4 (macOS: `brew install bash`)",
  );
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
