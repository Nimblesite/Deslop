// Artifacts the verifier proofs are pointed at.
// [DEPLOY-GATE-PORTABILITY] [DEPLOY-VSIX-PACKAGE] [DEPLOY-JETBRAINS-PACKAGE]
//
// Each builder stages a package that breaks exactly one rule, so that a
// verifier's rejection has something real to reject and the proof suite next
// door reads as a list of rules rather than a list of file-writing.
//
// Two things here are deliberate and were once wrong. The staged binaries are
// compiled programs, because every verifier ends by running the artifact and
// reading what it prints — a shell script is not runnable on Windows, and the
// proofs that staged one were reading the empty output of a process that never
// started. And the archives are written in-process, because `zip` is a program
// Windows does not ship, so the builders could not produce a fixture there at
// all.

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { writeFakeBinary } from "../lib/fake-binary.mjs";
import { copyFileAt, writeFileAt } from "../lib/write-file.mjs";
import { writeArchive } from "../lib/zip.mjs";
import { repoRoot } from "../lib/repo-root.mjs";
import { currentPlatform, executableName, VSIX_PLATFORMS } from "../release/vsix-platforms.mjs";

/** The version a valid fixture reports everywhere, matching a dev build. */
export const VALID_VERSION = "0.0.0-dev";

/** The product every fixture component belongs to, and the component names. */
const PRODUCT = "deslop";
const LSP = "deslop-lsp";
const MCP = "deslop-mcp";

/** The top-level entry each package carries everything under. */
const JETBRAINS_ROOT = "deslop-jetbrains";
const VSIX_ROOT = "extension";

/** The platform whose binaries this host can actually run. */
export const hostPlatform = currentPlatform();

/** A published platform that is not this one, for the foreign-bundle proof. */
export function foreignPlatform() {
  const other = VSIX_PLATFORMS.find((candidate) => candidate !== hostPlatform);
  if (!other) throw new Error(`${hostPlatform} is the only published platform`);
  return other;
}

/** How this host spells the executable called `name`. */
export function executableNamed(name) {
  return executableName(name, hostPlatform);
}

/** The two answers a real binary gives `--version` and `--version --json`. */
export function versionAnswers(name, version, kind, product = PRODUCT) {
  return {
    plain: `${name} ${version}`,
    json: { manifestVersion: 1, name, version, kind, language: "rust", product },
  };
}

/**
 * Writes a one-component manifest naming the CLI, so the binary verifier has
 * something to check a staged binary against.
 *
 * @param {string} work per-test temp directory
 * @param {{expectedVersion?: string}} overrides fields to break
 * @returns {string} path to the manifest
 */
export function writeManifestWithDeslopComponent(work, overrides) {
  const path = join(work, "manifest.json");
  const component = {
    id: PRODUCT,
    kind: "cli",
    language: "rust",
    binaryName: PRODUCT,
    expectedVersion: overrides.expectedVersion ?? VALID_VERSION,
    versionCheckStrategy: "version-flag",
    required: true,
    platforms: [hostPlatform],
  };
  const manifest = { manifestVersion: 1, product: { id: PRODUCT, version: VALID_VERSION }, components: [component] };
  writeFileSync(path, JSON.stringify(manifest));
  return path;
}

/**
 * Stages a JetBrains plugin zip, breaking whichever rule `options` names.
 *
 * @param {string} work per-test temp directory
 * @param {object} options one broken rule, or `{}` for a valid package
 * @returns {string} path to the zip
 */
export function buildJetBrainsZip(work, options) {
  const { stagingRoot, packageRoot } = stageRoot(work, JETBRAINS_ROOT, options);
  stageJetBrainsBinaries(packageRoot, options);
  stageSharedUiJar(packageRoot, options);
  return writeArchive(join(work, "plugin.zip"), stagingRoot, JETBRAINS_ROOT);
}

/**
 * Stages a VSIX, breaking whichever rule `options` names.
 *
 * @param {string} work per-test temp directory
 * @param {object} options one broken rule, or `{}` for a valid package
 * @returns {string} path to the VSIX
 */
export function buildVsixZip(work, options) {
  const { stagingRoot, packageRoot } = stageRoot(work, VSIX_ROOT, options);
  const identity = { publisher: "nimblesite", name: "deslop-live" };
  writeFileSync(join(packageRoot, "package.json"), JSON.stringify(identity));
  stageVsixBinaries(packageRoot, options);
  stageVsixStrays(packageRoot, options);
  return writeArchive(join(work, "package.vsix"), stagingRoot, VSIX_ROOT);
}

/** The staging tree both packages start from: one root, the manifest inside it. */
function stageRoot(work, rootName, options) {
  const stagingRoot = join(work, "stage");
  const packageRoot = join(stagingRoot, rootName);
  mkdirSync(packageRoot, { recursive: true });
  stageManifest(packageRoot, options);
  return { stagingRoot, packageRoot };
}

/** Copies the repository's real deployment manifest in, unless the proof omits it. */
function stageManifest(packageRoot, options) {
  if (options.manifest === null) return;
  copyFileAt(options.manifest ?? join(repoRoot, "shipwright.json"), join(packageRoot, "shipwright.json"));
}

/** The single LSP a JetBrains plugin bundles, plus any undeclared extra. */
function stageJetBrainsBinaries(pluginRoot, options) {
  const binDir = stageBinDir(pluginRoot);
  stageBundledLsp(binDir, options);
  stageUndeclaredBinary(binDir, options);
}

/** This platform's `bin/` directory inside a staged package. */
function stageBinDir(packageRoot) {
  const binDir = join(packageRoot, "bin", hostPlatform);
  mkdirSync(binDir, { recursive: true });
  return binDir;
}

/** The LSP both packages bundle, under whichever name and version is asked for. */
function stageBundledLsp(binDir, options) {
  if (options.skipBundledLsp) return;
  const name = options.lspName ?? LSP;
  stageBinary(binDir, LSP, versionAnswers(name, options.lspVersion ?? VALID_VERSION, "lsp"));
}

/** The three binaries a VSIX bundles, plus any undeclared extra. */
function stageVsixBinaries(extensionRoot, options) {
  const binDir = stageBinDir(extensionRoot);
  stageBinary(binDir, PRODUCT, versionAnswers(PRODUCT, VALID_VERSION, "cli"));
  stageBundledLsp(binDir, options);
  if (!options.skipBundledMcp) {
    stageBinary(binDir, MCP, versionAnswers(MCP, VALID_VERSION, "mcp"));
  }
  stageUndeclaredBinary(binDir, options);
}

/** A binary no manifest component claims, for the undeclared-bundle proofs. */
function stageUndeclaredBinary(binDir, options) {
  if (!options.extraBinName) return;
  const answers = versionAnswers(options.extraBinName, VALID_VERSION, "cli", options.extraBinName);
  stageBinary(binDir, options.extraBinName, answers);
}

/** Entries a VSIX must never carry: another platform's bin dir, compiled output. */
function stageVsixStrays(extensionRoot, options) {
  if (options.extraPlatform) {
    const extraDir = join(extensionRoot, "bin", options.extraPlatform);
    mkdirSync(extraDir, { recursive: true });
    // Named the way `options.extraPlatform` names an executable, not the way
    // this host does — it is that directory's presence the verifier rejects.
    writeFakeBinary(join(extraDir, LSP), versionAnswers(LSP, VALID_VERSION, "lsp"));
  }
  if (!options.extraOutFile) return;
  writeFileAt(join(extensionRoot, "out", "extension.js"), "compiled test artifact");
}

// [DEPLOY-JETBRAINS-PACKAGE] The real Gradle build stages the shared UI jar directly
// under lib/ (deslop-jetbrains-bundling.gradle.kts hoists it out of lib/modules/ so the
// tool window + Tools action load from the main plugin classloader). A valid fixture must
// mirror that or verifyFlatClasspath has no bite: omit it to prove the missing-shared-jar
// rejection, or leave it under lib/modules/ to prove the content-module rejection.
function stageSharedUiJar(pluginRoot, options) {
  if (options.skipSharedJar) return;
  const libDir = options.sharedJarUnderModules ? join(pluginRoot, "lib", "modules") : join(pluginRoot, "lib");
  mkdirSync(libDir, { recursive: true });
  writeFileSync(join(libDir, `deslop-shared-${VALID_VERSION}.jar`), "");
}

/** Writes a runnable fixture binary under this host's name for `name`. */
function stageBinary(binDir, name, answers) {
  writeFakeBinary(join(binDir, executableNamed(name)), answers);
}

/**
 * Writes a release workflow missing whichever gate `options` names.
 *
 * @param {string} work per-test temp directory
 * @param {object} options one omitted gate, or `{}` for the wired workflow
 * @returns {string} path to the workflow
 */
export function writeReleaseWorkflow(work, options) {
  const lines = [...releaseBuildJob(options), ...releasePackageJob(options)];
  const path = join(work, "release.yml");
  writeFileSync(path, `${lines.join("\n")}\n`);
  return path;
}

/** The tag-triggered build job, with its per-platform matrix and its gates. */
function releaseBuildJob(options) {
  const lines = [
    "name: Release", "on:", "  push:", "    tags: ['v*']", "jobs:", "  build:",
    "    runs-on: ubuntu-latest", "    strategy:", "      matrix:", "        include:",
    ...VSIX_PLATFORMS.map((target) => `          - vsix_target: ${target}`),
    "    steps:", "      - uses: actions/setup-node@v4", "        with:",
    '          node-version: "22.x"', "      - run: cargo build --release",
  ];
  if (!options.skipVersionStamper) lines.push("      - run: node scripts/release/stamp-release-version.mjs 0.1.0");
  if (!options.skipManifestGate) lines.push("      - run: node scripts/deployment/verify-deployment-manifest.mjs shipwright.json");
  if (!options.skipBinaryGate) {
    lines.push("      - run: node scripts/deployment/verify-deployment-binaries.mjs shipwright.json target/release linux-x64");
  }
  return lines;
}

/** The packaging job, whose one step is what the `--target` rule is about. */
function releasePackageJob(options) {
  const lines = ["  package-vsix:", "    runs-on: ubuntu-latest", "    steps:"];
  const verify = "      - run: node clients/vscode/scripts/verify-vsix-package.mjs";
  if (options.useBareVsce) {
    return [...lines, "      - run: npx vsce package --no-dependencies -o deslop.vsix", verify];
  }
  if (options.skipVsixGate) return [...lines, "      - run: echo skipped"];
  const packaged = "deslop-live-0.1.0-${{ matrix.vsix_target }}.vsix";
  const target = "${{ matrix.vsix_target }}";
  return [...lines, `      - run: cd clients/vscode && npm run package -- --target ${target} --out ${packaged}`, verify];
}
