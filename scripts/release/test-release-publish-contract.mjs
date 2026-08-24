// Release publish completeness contract ([DEPLOY-PUBLISH-COMPLETE], issue #348).
//
// v0.31.0 shipped from a run where one Marketplace timeout aborted the publish
// loop after 2 of 5 platform VSIXes were attempted: darwin-arm64 got v0.31.0,
// darwin-x64 timed out, and linux-arm64/linux-x64/win32-x64 were never
// attempted at all — while Open VSX, whose loop happened to succeed, served
// all five. Until a manual re-run the two registries disagreed about what the
// current release was, and nothing in the run named the missing platforms.
//
// These tests execute the release workflow's own publish `run:` blocks in a
// sandbox: five fake platform VSIX artifacts, a stub `npx` scripted to fail
// the way the Marketplace did, and a stub `az` for the token mint. Both
// registries are held to the same contract:
//   - a failing platform must not stop the other four from being attempted,
//     and the job must fail naming the platform that never reached the
//     registry;
//   - a failing platform is attempted exactly once — publishing is idempotent
//     via --skip-duplicate, so re-running the job is the retry and the job
//     must not burn its timeout budget re-hanging on a sick registry;
//   - an artifact set that is not exactly the expected platforms publishes
//     nothing at all, rather than shipping the subset that did upload.

import { spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { mappingValues, stepBody } from "../actions/action-yaml.mjs";
import { runContractSuite } from "../lib/contract-harness.mjs";
import { VSIX_MATRIX_KEY, VSIX_PLATFORMS } from "./vsix-platforms.mjs";

const repoRoot = fileURLToPath(new URL("../..", import.meta.url));
const workflow = readFileSync(resolve(repoRoot, ".github/workflows/release.yml"), "utf8");

// The publish steps are `set -euo pipefail` plus a node call — no globstar,
// no bash-4 features — so the stock shell on any runner or dev box runs them.
const SHELL = "bash";
const MARKETPLACE_STEP = "Publish each platform VSIX (Entra OIDC, no PAT)";
const OPENVSX_STEP = "Publish each platform VSIX to Open VSX";
const FAILING_PLATFORM = "darwin-x64";
const TEST_VERSION = "9.9.9";
const RELEASE_TAG = `v${TEST_VERSION}`;
const PRERELEASE_TAG = `v${TEST_VERSION}-rc.1`;
// Count-preserving corruption: still five artifact directories, but win32-x64
// never uploaded and a stray one took its place.
const MISCOUNTED_PLATFORMS = [...VSIX_PLATFORMS.filter((p) => p !== "win32-x64"), "linux-x64-stray"];
const NO_FAILURE = "none-of-the-platforms";

// Records every publish invocation, then fails any package whose path carries
// FAIL_TARGET. Everything else publishes instantly.
const NPX_STUB = `#!/bin/sh
printf '%s\\n' "$*" >> "\${STUB_LOG}"
case "$*" in
  *"\${FAIL_TARGET}"*)
    echo "##[error]Request timeout: /_apis/gallery/publishers/nimblesite/extensions/deslop-live" >&2
    exit 1
    ;;
esac
printf 'stub published\\n'
`;

// Stands in for the workflow's `az account get-access-token` token mint; the
// publish step only needs a token-shaped string on stdout.
const AZ_STUB = `#!/bin/sh
echo "stub-access-token"
`;

const marketplaceRunBlock = publishRunBlock(MARKETPLACE_STEP);
const openvsxRunBlock = publishRunBlock(OPENVSX_STEP);

const tests = [
  marketplaceAttemptsEveryPlatformExactlyOnceWhenOneFails,
  openvsxAttemptsEveryPlatformExactlyOnceWhenOneFails,
  marketplacePublishesEveryPlatformAgainstAHealthyRegistry,
  openvsxPublishesEveryPlatformAgainstAHealthyRegistry,
  anIncompleteArtifactSetPublishesNothing,
  aHyphenatedTagPublishesAsAPrerelease,
  declaredPlatformsMatchTheBuildMatrix,
];

runContractSuite(tests, "release publish contract");

function marketplaceAttemptsEveryPlatformExactlyOnceWhenOneFails() {
  assertNamesTheMissingPlatform(marketplaceScenario(FAILING_PLATFORM), "Marketplace");
}

function openvsxAttemptsEveryPlatformExactlyOnceWhenOneFails() {
  assertNamesTheMissingPlatform(openvsxScenario(FAILING_PLATFORM), "Open VSX");
}

function marketplacePublishesEveryPlatformAgainstAHealthyRegistry() {
  assertPublishesEveryPlatform(marketplaceScenario(NO_FAILURE), "Marketplace");
}

function openvsxPublishesEveryPlatformAgainstAHealthyRegistry() {
  assertPublishesEveryPlatform(openvsxScenario(NO_FAILURE), "Open VSX");
}

// One platform never reaches the registry. The other four must still be
// attempted, each exactly once, and the failure must name the missing
// platform — "which OSes lost the release" was exactly the question the
// v0.31.0 run could not answer.
function assertNamesTheMissingPlatform(scenario, registry) {
  for (const platform of VSIX_PLATFORMS) {
    assert(
      scenario.attempts[platform] === 1,
      `${registry}: ${platform} was attempted ${scenario.attempts[platform]} times, expected exactly 1; ` +
        `one failing platform must neither suppress the others nor be retried ` +
        `(attempts: ${JSON.stringify(scenario.attempts)})`,
    );
  }
  assert(
    scenario.status !== 0,
    `${registry}: a publish that leaves ${FAILING_PLATFORM} off the registry must fail the job`,
  );
  assertErrorNames(scenario, FAILING_PLATFORM, registry);
  assertEveryInvocationSkipsDuplicates(scenario, registry);
}

// A healthy registry must produce a green job that names every platform it
// published — the success line is the record the next release is audited from.
function assertPublishesEveryPlatform(scenario, registry) {
  for (const platform of VSIX_PLATFORMS) {
    assert(
      scenario.attempts[platform] === 1,
      `${registry}: ${platform} was attempted ${scenario.attempts[platform]} times, expected exactly 1 ` +
        `(attempts: ${JSON.stringify(scenario.attempts)})`,
    );
  }
  assert(scenario.status === 0, `${registry}: a healthy publish must exit 0; output:\n${scenario.output}`);
  for (const platform of VSIX_PLATFORMS) {
    assert(
      scenario.output.includes(platform),
      `${registry}: the success report must name ${platform}; output:\n${scenario.output}`,
    );
  }
  assertEveryInvocationSkipsDuplicates(scenario, registry);
}

// The completeness check must key on platform identity, not artifact count:
// five VSIXes that are not the five expected platforms is the partial release
// this contract exists to prevent, so nothing may be published at all.
function anIncompleteArtifactSetPublishesNothing() {
  const scenario = runPublishStep({
    runBlock: marketplaceRunBlock,
    failTarget: NO_FAILURE,
    platforms: MISCOUNTED_PLATFORMS,
  });
  assert(
    scenario.invocations.length === 0,
    `an artifact set missing win32-x64 must publish nothing, but ${scenario.invocations.length} ` +
      `publish call(s) ran: ${scenario.invocations.join(" | ")}`,
  );
  assert(scenario.status !== 0, "an incomplete artifact set must fail the job");
  assertErrorNames(scenario, "win32-x64", "Marketplace");
}

// A hyphenated tag is a prerelease: the Marketplace rejects a SemVer suffix in
// the version field, so the flag is the only channel that carries it.
function aHyphenatedTagPublishesAsAPrerelease() {
  const scenario = marketplaceScenario(NO_FAILURE, PRERELEASE_TAG);
  assert(scenario.status === 0, `a prerelease tag must publish; output:\n${scenario.output}`);
  for (const invocation of scenario.invocations) {
    assert(
      invocation.includes("--pre-release"),
      `tag ${PRERELEASE_TAG} must publish with --pre-release; got: ${invocation}`,
    );
  }
}

// The publisher's platform list and the build matrix are two declarations of
// the same fact. A sixth matrix leg added without updating the list would
// otherwise surface at release time, as a five-of-six release.
function declaredPlatformsMatchTheBuildMatrix() {
  const matrix = [...new Set(mappingValues(workflow, VSIX_MATRIX_KEY))].sort();
  assert(matrix.length >= 1, "the build matrix declares no vsix_target entries");
  assert(
    JSON.stringify(matrix) === JSON.stringify([...VSIX_PLATFORMS].sort()),
    `scripts/release/vsix-platforms.mjs declares [${VSIX_PLATFORMS.join(", ")}] but the build ` +
      `matrix produces [${matrix.join(", ")}]`,
  );
}

function assertErrorNames(scenario, platform, registry) {
  const namingLine = scenario.output
    .split("\n")
    .find((line) => line.toLowerCase().includes("error") && line.includes(platform));
  assert(
    namingLine !== undefined,
    `${registry}: the failure must name ${platform} as missing from the registry; output:\n${scenario.output}`,
  );
}

function assertEveryInvocationSkipsDuplicates(scenario, registry) {
  for (const invocation of scenario.invocations) {
    assert(
      invocation.includes("--skip-duplicate"),
      `${registry}: every publish must stay idempotent via --skip-duplicate; got: ${invocation}`,
    );
  }
}

function marketplaceScenario(failTarget, tag = RELEASE_TAG) {
  return runPublishStep({ runBlock: marketplaceRunBlock, failTarget, tag });
}

function openvsxScenario(failTarget, tag = RELEASE_TAG) {
  return runPublishStep({
    runBlock: openvsxRunBlock,
    failTarget,
    tag,
    extraEnv: { OVSX_PAT: "stub-openvsx-token" },
  });
}

// Executes a publish step's run block the way the runner does: cwd holding the
// downloaded artifacts/ tree, PATH resolving npx and az to the stubs, and the
// step's env populated.
function runPublishStep({ runBlock, failTarget, tag = RELEASE_TAG, extraEnv = {}, platforms = VSIX_PLATFORMS }) {
  const sandbox = mkdtempSync(join(tmpdir(), "deslop-publish-contract-"));
  try {
    const { stubBin, log } = buildSandbox(sandbox, platforms);
    const env = { ...process.env, PATH: `${stubBin}${delimiter}${process.env.PATH}` };
    const result = spawnSync(SHELL, ["-c", runBlock], {
      cwd: sandbox,
      encoding: "utf8",
      timeout: 120_000,
      env: { ...env, STUB_LOG: log, FAIL_TARGET: failTarget, GITHUB_REF_NAME: tag, ...extraEnv },
    });
    if (result.error) throw result.error;
    return summarize(result, readFileSync(log, "utf8"));
  } finally {
    rmSync(sandbox, { recursive: true, force: true });
  }
}

// One artifact directory per platform, the npx/az stubs, and the attempt log.
// scripts/ is linked into the sandbox (a junction on Windows, where plain
// symlinks need privileges) so the block may delegate to repository scripts.
function buildSandbox(sandbox, platforms) {
  for (const platform of platforms) {
    const artifactDir = join(sandbox, "artifacts", `vsix-${platform}`);
    mkdirSync(artifactDir, { recursive: true });
    writeFileSync(join(artifactDir, `deslop-live-${TEST_VERSION}-${platform}.vsix`), "stub vsix");
  }
  symlinkSync(resolve(repoRoot, "scripts"), join(sandbox, "scripts"), "junction");
  const stubBin = join(sandbox, "stub-bin");
  mkdirSync(stubBin);
  for (const [name, body] of [["npx", NPX_STUB], ["az", AZ_STUB]]) {
    writeFileSync(join(stubBin, name), body);
    chmodSync(join(stubBin, name), 0o755);
  }
  const log = join(sandbox, "publish-attempts.log");
  writeFileSync(log, "");
  return { stubBin, log };
}

function summarize(result, attemptsLog) {
  const invocations = attemptsLog.split("\n").filter((line) => line !== "");
  const attempts = Object.fromEntries(
    VSIX_PLATFORMS.map((platform) => [
      platform,
      invocations.filter((line) => line.includes(`-${platform}.vsix`)).length,
    ]),
  );
  return {
    status: result.status,
    output: `${result.stdout ?? ""}${result.stderr ?? ""}`,
    attempts,
    invocations,
  };
}

// The dedented body of one publish step's `run:` block, read through the
// shared YAML step reader rather than pattern-matched out of the file.
function publishRunBlock(stepName) {
  const script = stepBody(workflow, stepName);
  if (script.includes("${{")) {
    throw new Error(`"${stepName}" contains unexpanded \${{ }} expressions; the sandbox cannot execute it`);
  }
  return script;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
