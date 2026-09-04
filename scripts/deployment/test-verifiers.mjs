// Proof tests for Shipwright verifier scripts. Tests [DEPLOY-VSIX-PACKAGE].
// Each fake artifact
// violates one Shipwright contract rule so verifier failures have bite.
//
// The artifacts themselves are staged by verifier-fixtures.mjs; this file is
// the list of rules. Every proof runs against the platform this host can
// execute, so the `.exe` naming rule is under test on Windows and the Unix
// naming rule on Linux — the two are also asserted against each other below,
// where no binary has to run.

import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runContractSuite } from "../lib/contract-harness.mjs";
import { repoRoot } from "../lib/repo-root.mjs";
import { VSIX_PLATFORMS } from "../release/vsix-platforms.mjs";
import {
  buildJetBrainsZip,
  buildVsixZip,
  executableNamed,
  foreignPlatform,
  hostPlatform,
  VALID_VERSION,
  versionAnswers,
  writeManifestWithDeslopComponent,
  writeReleaseWorkflow,
} from "./verifier-fixtures.mjs";
import { writeFakeBinary } from "../lib/fake-binary.mjs";

const verifyManifest = join(repoRoot, "scripts/deployment/verify-deployment-manifest.mjs");
const verifyBinaries = join(repoRoot, "scripts/deployment/verify-deployment-binaries.mjs");
const verifyJetBrains = join(repoRoot, "scripts/deployment/verify-jetbrains-package.mjs");
const verifyVsix = join(repoRoot, "clients/vscode/scripts/verify-vsix-package.mjs");
const verifyReleaseWorkflow = join(repoRoot, "scripts/release/verify-release-workflow-gates.mjs");
const platform = hostPlatform;
const validVersion = VALID_VERSION;

const cases = [
  manifestRejectsMissingProductId, manifestRejectsLooseSemver,
  manifestRejectsHostVerifyingUnknownComponent, manifestRejectsExpectedVersionDrift,
  manifestAcceptsRepoManifest, binariesRejectWrongVersion,
  binariesRejectWrongComponentName, binariesRejectMissingBinary,
  binariesNameTheFileEachTargetPlatformSpells,
  binariesRejectStaleJsonManifestVersion, binariesAcceptValidContract,
  jetbrainsRejectsMissingManifest, jetbrainsRejectsMissingBundledBinary,
  jetbrainsRejectsWrongVersionBundle, jetbrainsRejectsWrongComponentNameBundle,
  jetbrainsRejectsUndeclaredBundle, jetbrainsRejectsContentModuleJar,
  jetbrainsRejectsMissingSharedJar, jetbrainsAcceptsValidPackage,
  vsixRejectsMissingManifest, vsixRejectsMissingBundledLsp,
  vsixRejectsMissingBundledMcp, vsixRejectsWrongVersionBundle,
  vsixRejectsWrongComponentNameBundle, vsixRejectsUndeclaredBundle,
  vsixRejectsForeignPlatformBundle, vsixRejectsCompiledOutDir,
  vsixAcceptsValidPackage, releaseWorkflowRejectsMissingManifestGate,
  releaseWorkflowRejectsMissingBinaryGate, releaseWorkflowRejectsMissingVsixGate,
  releaseWorkflowRejectsMissingVersionStamper, releaseWorkflowRejectsBareVsce,
  releaseWorkflowAcceptsRepoWorkflow,
];

runContractSuite(cases, "verifier proof", "deslop-verifier-");

function manifestRejectsMissingProductId(work) {
  const path = join(work, "missing-product.json");
  writeManifest(path, { manifestVersion: 1, components: [] });
  expectFail(verifyManifest, [path], /product\.id/);
}

function manifestRejectsLooseSemver(work) {
  const path = join(work, "loose-semver.json");
  writeManifest(path, { manifestVersion: 1, product: { id: "deslop", version: "v0.1" }, components: [] });
  expectFail(verifyManifest, [path], /semantic version/);
}

function manifestRejectsHostVerifyingUnknownComponent(work) {
  const path = join(work, "unknown-host-component.json");
  writeManifest(path, {
    manifestVersion: 1,
    product: { id: "deslop", version: validVersion },
    components: [cliComponent(validVersion)],
    hosts: { vscode: { activationVerifies: ["does-not-exist"] } },
  });
  expectFail(verifyManifest, [path], /unknown component does-not-exist/);
}

function manifestRejectsExpectedVersionDrift(work) {
  const path = join(work, "drifted.json");
  writeManifest(path, {
    manifestVersion: 1,
    product: { id: "deslop", version: "0.2.0" },
    components: [cliComponent(validVersion)],
  });
  expectFail(verifyManifest, [path], /expectedVersion 0\.0\.0-dev must match product\.version 0\.2\.0/);
}

function manifestAcceptsRepoManifest() {
  const path = join(repoRoot, "shipwright.json");
  expectSuccess(verifyManifest, [path], /valid deployment manifest/);
}

// ---------- binary verifier ----------

function binariesRejectWrongVersion(work) {
  binariesReject(work, versionAnswers("deslop", "0.0.9", "cli"), /reported deslop 0\.0\.9/, { expectedVersion: validVersion });
}

function binariesRejectWrongComponentName(work) {
  binariesReject(work, versionAnswers("wrong-name", validVersion, "cli"), /reported wrong-name 0\.0\.0-dev/);
}

function binariesRejectMissingBinary(work) {
  binariesReject(work, null, /Missing deslop/);
}

// [DEPLOY-BINARY-FILE-NAME] A win32 artifact is `deslop.exe` and every other
// platform's is `deslop`. Nothing here runs, so all five published platforms
// are checked from any host: the naming rule is a rule about the target, and
// pinning it only to whichever platform the runner happens to be leaves the
// other four spellings unasserted — which is how the `.exe` branch went
// unexercised on every machine that has ever run this suite.
function binariesNameTheFileEachTargetPlatformSpells(work) {
  const manifestPath = writeManifestWithDeslopComponent(work, {});
  const binDir = stageBinDir(work, null);
  for (const target of VSIX_PLATFORMS) {
    const expected = join(binDir, target.startsWith("win32") ? "deslop.exe" : "deslop");
    expectFail(verifyBinaries, [manifestPath, binDir, target], `Missing deslop at ${expected}`);
  }
}

function binariesRejectStaleJsonManifestVersion(work) {
  const manifestPath = writeManifestWithDeslopComponent(work, {});
  const answers = versionAnswers("deslop", validVersion, "cli");
  answers.json.manifestVersion = 999;
  const binDir = stageBinDir(work, answers);
  expectFail(verifyBinaries, [manifestPath, binDir, platform], /manifestVersion must be 1/);
}

function binariesAcceptValidContract(work) {
  binariesAccept(work, versionAnswers("deslop", validVersion, "cli"), /Verified deployment binaries/);
}

// ---------- jetbrains package verifier ----------

function jetbrainsRejectsMissingManifest(work) {
  jetbrainsRejects(work, { manifest: null }, /Missing .*shipwright\.json/);
}

function jetbrainsRejectsMissingBundledBinary(work) {
  jetbrainsRejects(work, { skipBundledLsp: true }, /Missing .*deslop-lsp/);
}

function jetbrainsRejectsWrongVersionBundle(work) {
  jetbrainsRejects(work, { lspVersion: "0.0.9" }, /reported deslop-lsp 0\.0\.9/);
}

function jetbrainsRejectsWrongComponentNameBundle(work) {
  jetbrainsRejects(work, { lspName: "deslop" }, /reported deslop 0\.0\.0-dev/);
}

function jetbrainsRejectsUndeclaredBundle(work) {
  jetbrainsRejects(work, { extraBinName: "rogue-helper" }, /Undeclared JetBrains binary/);
}

function jetbrainsRejectsContentModuleJar(work) {
  jetbrainsRejects(work, { sharedJarUnderModules: true }, /under lib\/modules\//);
}

function jetbrainsRejectsMissingSharedJar(work) {
  jetbrainsRejects(work, { skipSharedJar: true }, /missing the shared UI jar/);
}

function jetbrainsAcceptsValidPackage(work) {
  jetbrainsAccepts(work, {}, /Verified JetBrains package/);
}

// ---------- vsix package verifier ----------

function vsixRejectsMissingManifest(work) {
  vsixRejects(work, { manifest: null }, /Missing extension\/shipwright\.json/);
}

function vsixRejectsMissingBundledLsp(work) {
  vsixRejects(work, { skipBundledLsp: true }, /Missing extension\/bin\/.*\/deslop-lsp/);
}

function vsixRejectsMissingBundledMcp(work) {
  vsixRejects(work, { skipBundledMcp: true }, /Missing extension\/bin\/.*\/deslop-mcp/);
}

function vsixRejectsWrongVersionBundle(work) {
  vsixRejects(work, { lspVersion: "0.0.9" }, /reported deslop-lsp 0\.0\.9/);
}

function vsixRejectsWrongComponentNameBundle(work) {
  vsixRejects(work, { lspName: "wrong-name" }, /reported wrong-name 0\.0\.0-dev/);
}

function vsixRejectsUndeclaredBundle(work) {
  vsixRejects(work, { extraBinName: "rogue-helper" }, /Undeclared executable in VSIX/);
}

function vsixRejectsForeignPlatformBundle(work) {
  vsixRejects(work, { extraPlatform: foreignPlatform() }, /must contain only/);
}

function vsixRejectsCompiledOutDir(work) {
  vsixRejects(work, { extraOutFile: true }, /from outside the extension: [^\n]*extension\/out\//);
}

function vsixAcceptsValidPackage(work) {
  vsixAccepts(work, {}, /Verified deployment manifest/);
}

// ---------- release workflow gate verifier ----------

function releaseWorkflowRejectsMissingManifestGate(work) {
  workflowRejects(work, { skipManifestGate: true }, /missing the manifest validator/);
}

function releaseWorkflowRejectsMissingBinaryGate(work) {
  workflowRejects(work, { skipBinaryGate: true }, /missing the binary version contract verifier/);
}

function releaseWorkflowRejectsMissingVsixGate(work) {
  workflowRejects(work, { skipVsixGate: true }, /missing the VSIX package verifier/);
}

function releaseWorkflowRejectsMissingVersionStamper(work) {
  workflowRejects(work, { skipVersionStamper: true }, /missing the build-time release version stamper/);
}

function releaseWorkflowRejectsBareVsce(work) {
  workflowRejects(work, { useBareVsce: true }, /without --target|must package VSIX artifacts with --target/);
}

function releaseWorkflowAcceptsRepoWorkflow() {
  const path = join(repoRoot, ".github/workflows/release.yml");
  expectSuccess(verifyReleaseWorkflow, [path], /release workflow gates wired/);
}

// ---------- helpers ----------

/**
 * The one shape every proof below has: stage an artifact that breaks one rule,
 * point the verifier at it, and state what its refusal must say. Named once so
 * that each proof is the rule it pins and nothing else — twenty copies of the
 * scaffolding is twenty places for a proof to stop pointing at its verifier
 * without the suite noticing.
 */
function jetbrainsRejects(work, options, expected) {
  expectFail(verifyJetBrains, [buildJetBrainsZip(work, options), platform], expected);
}

function jetbrainsAccepts(work, options, expected) {
  expectSuccess(verifyJetBrains, [buildJetBrainsZip(work, options), platform], expected);
}

function vsixRejects(work, options, expected) {
  expectFail(verifyVsix, [buildVsixZip(work, options), platform], expected);
}

function vsixAccepts(work, options, expected) {
  expectSuccess(verifyVsix, [buildVsixZip(work, options), platform], expected);
}

function workflowRejects(work, options, expected) {
  expectFail(verifyReleaseWorkflow, [writeReleaseWorkflow(work, options)], expected);
}

function binariesReject(work, answers, expected, overrides = {}) {
  verifyStagedBinary(expectFail, work, answers, expected, overrides);
}

function binariesAccept(work, answers, expected, overrides = {}) {
  verifyStagedBinary(expectSuccess, work, answers, expected, overrides);
}

function verifyStagedBinary(expect, work, answers, expected, overrides) {
  const manifestPath = writeManifestWithDeslopComponent(work, overrides);
  expect(verifyBinaries, [manifestPath, stageBinDir(work, answers), platform], expected);
}

/** A `bin/` directory holding the CLI fixture, or nothing when `answers` is null. */
function stageBinDir(work, answers) {
  const binDir = join(work, "bin");
  mkdirSync(binDir, { recursive: true });
  if (answers) writeFakeBinary(join(binDir, executableNamed("deslop")), answers);
  return binDir;
}

/** The one valid CLI component the manifest proofs vary one field of. */
function cliComponent(expectedVersion) {
  return {
    id: "deslop",
    kind: "cli",
    language: "rust",
    binaryName: "deslop",
    expectedVersion,
    versionCheckStrategy: "version-flag",
  };
}

/** Writes a manifest fixture verbatim, with no defaults filled in. */
function writeManifest(path, manifest) {
  writeFileSync(path, JSON.stringify(manifest));
}

/** True when `expected` — a substring or a pattern — describes `text`. */
function describes(expected, text) {
  return typeof expected === "string" ? text.includes(expected) : expected.test(text);
}

/** The verifier must refuse, and say why in a way `expected` describes. */
function expectFail(script, args, expected) {
  runVerifier(script, args, expected, false);
}

/** The verifier must accept, and its report must be what `expected` describes. */
function expectSuccess(script, args, expected) {
  runVerifier(script, args, expected, true);
}

/**
 * Runs one verifier and holds it to a verdict and to what it said. A refusal
 * is read from both streams, because a verifier may name the artifact on
 * stdout before throwing; an acceptance is read from stdout alone, so a
 * warning on stderr can never stand in for the report that proves it ran.
 */
function runVerifier(script, args, expected, mustSucceed) {
  const result = spawnSync("node", [script, ...args], { encoding: "utf8" });
  const verdict = `${result.stdout}\n${result.stderr}`;
  if ((result.status === 0) !== mustSucceed) {
    const wanted = mustSucceed ? "success" : "failure";
    throw new Error(`expected ${wanted} but the verifier exited ${result.status}\n${verdict}`);
  }
  const said = mustSucceed ? result.stdout : verdict;
  if (!describes(expected, said)) {
    throw new Error(`expected output to match ${expected}\ngot=${said}`);
  }
}
