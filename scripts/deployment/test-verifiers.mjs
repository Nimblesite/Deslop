// Proof tests for Shipwright verifier scripts. Tests [DEPLOY-VSIX-PACKAGE].
// Each fake artifact
// violates one Shipwright contract rule so verifier failures have bite.

import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync, copyFileSync, chmodSync } from "node:fs";
import { join } from "node:path";

import { runContractSuite } from "../lib/contract-harness.mjs";
import { repoRoot } from "../lib/repo-root.mjs";

const verifyManifest = join(repoRoot, "scripts/deployment/verify-deployment-manifest.mjs");
const verifyBinaries = join(repoRoot, "scripts/deployment/verify-deployment-binaries.mjs");
const verifyJetBrains = join(repoRoot, "scripts/deployment/verify-jetbrains-package.mjs");
const verifyVsix = join(repoRoot, "clients/vscode/scripts/verify-vsix-package.mjs");
const verifyReleaseWorkflow = join(repoRoot, "scripts/release/verify-release-workflow-gates.mjs");
const platform = "darwin-arm64";
const validVersion = "0.0.0-dev";
const hostPlatform = detectHostPlatform();

const cases = [
  manifestRejectsMissingProductId, manifestRejectsLooseSemver,
  manifestRejectsHostVerifyingUnknownComponent, manifestRejectsExpectedVersionDrift,
  manifestAcceptsRepoManifest, binariesRejectWrongVersion,
  binariesRejectWrongComponentName, binariesRejectMissingBinary,
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
  writeFileSync(path, JSON.stringify({ manifestVersion: 1, components: [] }));
  expectFail(verifyManifest, [path], /product\.id/);
}

function manifestRejectsLooseSemver(work) {
  const path = join(work, "loose-semver.json");
  writeFileSync(
    path,
    JSON.stringify({
      manifestVersion: 1,
      product: { id: "deslop", version: "v0.1" },
      components: [],
    }),
  );
  expectFail(verifyManifest, [path], /semantic version/);
}

function manifestRejectsHostVerifyingUnknownComponent(work) {
  const path = join(work, "unknown-host-component.json");
  writeFileSync(
    path,
    JSON.stringify({
      manifestVersion: 1,
      product: { id: "deslop", version: validVersion },
      components: [
        {
          id: "deslop",
          kind: "cli",
          language: "rust",
          binaryName: "deslop",
          expectedVersion: validVersion,
          versionCheckStrategy: "version-flag",
        },
      ],
      hosts: { vscode: { activationVerifies: ["does-not-exist"] } },
    }),
  );
  expectFail(verifyManifest, [path], /unknown component does-not-exist/);
}

function manifestRejectsExpectedVersionDrift(work) {
  const path = join(work, "drifted.json");
  writeFileSync(
    path,
    JSON.stringify({
      manifestVersion: 1,
      product: { id: "deslop", version: "0.2.0" },
      components: [
        {
          id: "deslop",
          kind: "cli",
          language: "rust",
          binaryName: "deslop",
          expectedVersion: validVersion,
          versionCheckStrategy: "version-flag",
        },
      ],
    }),
  );
  expectFail(verifyManifest, [path], /expectedVersion 0\.0\.0-dev must match product\.version 0\.2\.0/);
}

function manifestAcceptsRepoManifest() {
  const path = join(repoRoot, "shipwright.json");
  expectSuccess(verifyManifest, [path], /valid deployment manifest/);
}

// ---------- binary verifier ----------

function binariesRejectWrongVersion(work) {
  const manifestPath = writeManifestWithDeslopComponent(work, { expectedVersion: validVersion });
  const binDir = join(work, "bin");
  mkdirSync(binDir);
  writeFakeBinary(join(binDir, "deslop"), {
    plain: "deslop 0.0.9",
    json: { manifestVersion: 1, name: "deslop", version: "0.0.9", kind: "cli", language: "rust", product: "deslop" },
  });
  expectFail(verifyBinaries, [manifestPath, binDir, platform], /reported deslop 0\.0\.9/);
}

function binariesRejectWrongComponentName(work) {
  const manifestPath = writeManifestWithDeslopComponent(work, {});
  const binDir = join(work, "bin");
  mkdirSync(binDir);
  writeFakeBinary(join(binDir, "deslop"), {
    plain: `wrong-name ${validVersion}`,
    json: { manifestVersion: 1, name: "wrong-name", version: validVersion, kind: "cli", language: "rust", product: "deslop" },
  });
  expectFail(verifyBinaries, [manifestPath, binDir, platform], /reported wrong-name 0\.0\.0-dev/);
}

function binariesRejectMissingBinary(work) {
  const manifestPath = writeManifestWithDeslopComponent(work, {});
  const binDir = join(work, "bin");
  mkdirSync(binDir);
  expectFail(verifyBinaries, [manifestPath, binDir, platform], /Missing deslop/);
}

function binariesRejectStaleJsonManifestVersion(work) {
  const manifestPath = writeManifestWithDeslopComponent(work, {});
  const binDir = join(work, "bin");
  mkdirSync(binDir);
  writeFakeBinary(join(binDir, "deslop"), {
    plain: `deslop ${validVersion}`,
    json: { manifestVersion: 999, name: "deslop", version: validVersion, kind: "cli", language: "rust", product: "deslop" },
  });
  expectFail(verifyBinaries, [manifestPath, binDir, platform], /manifestVersion must be 1/);
}

function binariesAcceptValidContract(work) {
  const manifestPath = writeManifestWithDeslopComponent(work, {});
  const binDir = join(work, "bin");
  mkdirSync(binDir);
  writeFakeBinary(join(binDir, "deslop"), {
    plain: `deslop ${validVersion}`,
    json: { manifestVersion: 1, name: "deslop", version: validVersion, kind: "cli", language: "rust", product: "deslop" },
  });
  expectSuccess(verifyBinaries, [manifestPath, binDir, platform], /Verified deployment binaries/);
}

// ---------- jetbrains package verifier ----------

function jetbrainsRejectsMissingManifest(work) {
  const zipPath = buildJetBrainsZip(work, { manifest: null });
  expectFail(verifyJetBrains, [zipPath, platform], /Missing .*shipwright\.json/);
}

function jetbrainsRejectsMissingBundledBinary(work) {
  const zipPath = buildJetBrainsZip(work, { skipBundledLsp: true });
  expectFail(verifyJetBrains, [zipPath, platform], /Missing .*deslop-lsp/);
}

function jetbrainsRejectsWrongVersionBundle(work) {
  const zipPath = buildJetBrainsZip(work, { lspVersion: "0.0.9" });
  expectFail(verifyJetBrains, [zipPath, platform], /reported deslop-lsp 0\.0\.9/);
}

function jetbrainsRejectsWrongComponentNameBundle(work) {
  const zipPath = buildJetBrainsZip(work, { lspName: "deslop" });
  expectFail(verifyJetBrains, [zipPath, platform], /reported deslop 0\.0\.0-dev/);
}

function jetbrainsRejectsUndeclaredBundle(work) {
  const zipPath = buildJetBrainsZip(work, { extraBinName: "rogue-helper" });
  expectFail(verifyJetBrains, [zipPath, platform], /Undeclared JetBrains binary/);
}

function jetbrainsRejectsContentModuleJar(work) {
  const zipPath = buildJetBrainsZip(work, { sharedJarUnderModules: true });
  expectFail(verifyJetBrains, [zipPath, platform], /under lib\/modules\//);
}

function jetbrainsRejectsMissingSharedJar(work) {
  const zipPath = buildJetBrainsZip(work, { skipSharedJar: true });
  expectFail(verifyJetBrains, [zipPath, platform], /missing the shared UI jar/);
}

function jetbrainsAcceptsValidPackage(work) {
  const zipPath = buildJetBrainsZip(work, {});
  expectSuccess(verifyJetBrains, [zipPath, platform], /Verified JetBrains package/);
}

// ---------- vsix package verifier ----------

function vsixRejectsMissingManifest(work) {
  const vsixPath = buildVsixZip(work, { manifest: null });
  expectFail(verifyVsix, [vsixPath, hostPlatform], /Missing extension\/shipwright\.json/);
}

function vsixRejectsMissingBundledLsp(work) {
  const vsixPath = buildVsixZip(work, { skipBundledLsp: true });
  expectFail(verifyVsix, [vsixPath, hostPlatform], /Missing extension\/bin\/.*\/deslop-lsp/);
}

function vsixRejectsMissingBundledMcp(work) {
  const vsixPath = buildVsixZip(work, { skipBundledMcp: true });
  expectFail(verifyVsix, [vsixPath, hostPlatform], /Missing extension\/bin\/.*\/deslop-mcp/);
}

function vsixRejectsWrongVersionBundle(work) {
  const vsixPath = buildVsixZip(work, { lspVersion: "0.0.9" });
  expectFail(verifyVsix, [vsixPath, hostPlatform], /reported deslop-lsp 0\.0\.9/);
}

function vsixRejectsWrongComponentNameBundle(work) {
  const vsixPath = buildVsixZip(work, { lspName: "wrong-name" });
  expectFail(verifyVsix, [vsixPath, hostPlatform], /reported wrong-name 0\.0\.0-dev/);
}

function vsixRejectsUndeclaredBundle(work) {
  const vsixPath = buildVsixZip(work, { extraBinName: "rogue-helper" });
  expectFail(verifyVsix, [vsixPath, hostPlatform], /Undeclared executable in VSIX/);
}

function vsixRejectsForeignPlatformBundle(work) {
  const vsixPath = buildVsixZip(work, { extraPlatform: foreignPlatform() });
  expectFail(verifyVsix, [vsixPath, hostPlatform], /must contain only/);
}

function vsixRejectsCompiledOutDir(work) {
  const vsixPath = buildVsixZip(work, { extraOutFile: true });
  expectFail(verifyVsix, [vsixPath, hostPlatform], /must not include extension\/out\//);
}

function vsixAcceptsValidPackage(work) {
  const vsixPath = buildVsixZip(work, {});
  expectSuccess(verifyVsix, [vsixPath, hostPlatform], /Verified deployment manifest/);
}

// ---------- release workflow gate verifier ----------

function releaseWorkflowRejectsMissingManifestGate(work) {
  const path = writeReleaseWorkflow(work, { skipManifestGate: true });
  expectFail(verifyReleaseWorkflow, [path], /missing the manifest validator/);
}

function releaseWorkflowRejectsMissingBinaryGate(work) {
  const path = writeReleaseWorkflow(work, { skipBinaryGate: true });
  expectFail(verifyReleaseWorkflow, [path], /missing the binary version contract verifier/);
}

function releaseWorkflowRejectsMissingVsixGate(work) {
  const path = writeReleaseWorkflow(work, { skipVsixGate: true });
  expectFail(verifyReleaseWorkflow, [path], /missing the VSIX package verifier/);
}

function releaseWorkflowRejectsMissingVersionStamper(work) {
  const path = writeReleaseWorkflow(work, { skipVersionStamper: true });
  expectFail(verifyReleaseWorkflow, [path], /missing the build-time release version stamper/);
}

function releaseWorkflowRejectsBareVsce(work) {
  const path = writeReleaseWorkflow(work, { useBareVsce: true });
  expectFail(verifyReleaseWorkflow, [path], /without --target|must package VSIX artifacts with --target/);
}

function releaseWorkflowAcceptsRepoWorkflow() {
  const path = join(repoRoot, ".github/workflows/release.yml");
  expectSuccess(verifyReleaseWorkflow, [path], /release workflow gates wired/);
}

// ---------- helpers ----------

function writeManifestWithDeslopComponent(work, overrides) {
  const path = join(work, "manifest.json");
  writeFileSync(
    path,
    JSON.stringify({
      manifestVersion: 1,
      product: { id: "deslop", version: validVersion },
      components: [
        {
          id: "deslop",
          kind: "cli",
          language: "rust",
          binaryName: "deslop",
          expectedVersion: overrides.expectedVersion ?? validVersion,
          versionCheckStrategy: "version-flag",
          required: true,
          platforms: [platform],
        },
      ],
    }),
  );
  return path;
}

function writeFakeBinary(path, payload) {
  const json = JSON.stringify(payload.json).replace(/'/g, "'\\''");
  const script = `#!/bin/sh\nif [ "$1" = "--version" ] && [ "$2" = "--json" ]; then\n  printf '%s\\n' '${json}'\nelse\n  printf '%s\\n' '${payload.plain}'\nfi\n`;
  writeFileSync(path, script);
  chmodSync(path, 0o755);
}

function buildJetBrainsZip(work, options) {
  const stagingRoot = join(work, "stage");
  const pluginRoot = join(stagingRoot, "deslop-jetbrains");
  const binDir = join(pluginRoot, "bin", platform);
  mkdirSync(binDir, { recursive: true });

  if (options.manifest !== null) {
    const manifestSource = options.manifest ?? join(repoRoot, "shipwright.json");
    copyFileSync(manifestSource, join(pluginRoot, "shipwright.json"));
  }

  if (!options.skipBundledLsp) {
    const name = options.lspName ?? "deslop-lsp";
    const version = options.lspVersion ?? validVersion;
    writeFakeBinary(join(binDir, "deslop-lsp"), {
      plain: `${name} ${version}`,
      json: { manifestVersion: 1, name, version, kind: "lsp", language: "rust", product: "deslop" },
    });
  }

  if (options.extraBinName) {
    writeFakeBinary(join(binDir, options.extraBinName), {
      plain: `${options.extraBinName} ${validVersion}`,
      json: { manifestVersion: 1, name: options.extraBinName, version: validVersion, kind: "cli", language: "rust", product: options.extraBinName },
    });
  }

  stageSharedUiJar(pluginRoot, options);

  const zipPath = join(work, "plugin.zip");
  const result = spawnSync("zip", ["-rq", zipPath, "deslop-jetbrains"], {
    cwd: stagingRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) throw new Error(`zip failed: ${result.stderr}`);
  return zipPath;
}

// [DEPLOY-JETBRAINS-PACKAGE] The real Gradle build stages the shared UI jar directly
// under lib/ (deslop-jetbrains-bundling.gradle.kts hoists it out of lib/modules/ so the
// tool window + Tools action load from the main plugin classloader). A valid fixture must
// mirror that or verifyFlatClasspath has no bite: omit it to prove the missing-shared-jar
// rejection, or leave it under lib/modules/ to prove the content-module rejection.
function stageSharedUiJar(pluginRoot, options) {
  if (options.skipSharedJar) return;
  const libDir = options.sharedJarUnderModules
    ? join(pluginRoot, "lib", "modules")
    : join(pluginRoot, "lib");
  mkdirSync(libDir, { recursive: true });
  writeFileSync(join(libDir, `deslop-shared-${validVersion}.jar`), "");
}

function buildVsixZip(work, options) {
  const stagingRoot = join(work, "stage");
  const extensionRoot = join(stagingRoot, "extension");
  const binDir = join(extensionRoot, "bin", hostPlatform);
  mkdirSync(binDir, { recursive: true });
  if (options.extraPlatform) {
    const extraDir = join(extensionRoot, "bin", options.extraPlatform);
    mkdirSync(extraDir, { recursive: true });
    writeFakeBinary(join(extraDir, "deslop-lsp"), {
      plain: `deslop-lsp ${validVersion}`,
      json: { manifestVersion: 1, name: "deslop-lsp", version: validVersion, kind: "lsp", language: "rust", product: "deslop" },
    });
  }

  if (options.manifest !== null) {
    const manifestSource = options.manifest ?? join(repoRoot, "shipwright.json");
    copyFileSync(manifestSource, join(extensionRoot, "shipwright.json"));
  }
  writeFileSync(
    join(extensionRoot, "package.json"),
    JSON.stringify({ publisher: "nimblesite", name: "deslop-live" }),
  );

  writeFakeBinary(join(binDir, "deslop"), {
    plain: `deslop ${validVersion}`,
    json: { manifestVersion: 1, name: "deslop", version: validVersion, kind: "cli", language: "rust", product: "deslop" },
  });
  if (!options.skipBundledLsp) {
    const name = options.lspName ?? "deslop-lsp";
    const version = options.lspVersion ?? validVersion;
    writeFakeBinary(join(binDir, "deslop-lsp"), {
      plain: `${name} ${version}`,
      json: { manifestVersion: 1, name, version, kind: "lsp", language: "rust", product: "deslop" },
    });
  }

  if (!options.skipBundledMcp) {
    writeFakeBinary(join(binDir, "deslop-mcp"), {
      plain: `deslop-mcp ${validVersion}`,
      json: { manifestVersion: 1, name: "deslop-mcp", version: validVersion, kind: "mcp", language: "rust", product: "deslop" },
    });
  }

  if (options.extraBinName) {
    writeFakeBinary(join(binDir, options.extraBinName), {
      plain: `${options.extraBinName} ${validVersion}`,
      json: { manifestVersion: 1, name: options.extraBinName, version: validVersion, kind: "cli", language: "rust", product: options.extraBinName },
    });
  }
  if (options.extraOutFile) {
    const outDir = join(extensionRoot, "out");
    mkdirSync(outDir, { recursive: true });
    writeFileSync(join(outDir, "extension.js"), "compiled test artifact");
  }

  const vsixPath = join(work, "package.vsix");
  const result = spawnSync("zip", ["-rq", vsixPath, "extension"], {
    cwd: stagingRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) throw new Error(`zip failed: ${result.stderr}`);
  return vsixPath;
}

function writeReleaseWorkflow(work, options) {
  const lines = [
    "name: Release",
    "on:",
    "  push:",
    "    tags: ['v*']",
    "jobs:",
    "  build:",
    "    runs-on: ubuntu-latest",
    "    strategy:",
    "      matrix:",
    "        include:",
    "          - vsix_target: linux-x64",
    "          - vsix_target: linux-arm64",
    "          - vsix_target: darwin-x64",
    "          - vsix_target: darwin-arm64",
    "          - vsix_target: win32-x64",
    "    steps:",
    "      - uses: actions/setup-node@v4",
    "        with:",
    "          node-version: \"22.x\"",
    "      - run: cargo build --release",
  ];
  if (!options.skipVersionStamper) {
    lines.push("      - run: node scripts/release/stamp-release-version.mjs 0.1.0");
  }
  if (!options.skipManifestGate) {
    lines.push("      - run: node scripts/deployment/verify-deployment-manifest.mjs shipwright.json");
  }
  if (!options.skipBinaryGate) {
    lines.push("      - run: node scripts/deployment/verify-deployment-binaries.mjs shipwright.json target/release linux-x64");
  }
  lines.push("  package-vsix:");
  lines.push("    runs-on: ubuntu-latest");
  lines.push("    steps:");
  if (options.useBareVsce) {
    lines.push("      - run: npx vsce package --no-dependencies -o deslop.vsix");
    lines.push("      - run: node clients/vscode/scripts/verify-vsix-package.mjs");
  } else if (!options.skipVsixGate) {
    lines.push("      - run: cd clients/vscode && npm run package -- --target ${{ matrix.vsix_target }} --out deslop-live-0.1.0-${{ matrix.vsix_target }}.vsix");
    lines.push("      - run: node clients/vscode/scripts/verify-vsix-package.mjs");
  } else {
    lines.push("      - run: echo skipped");
  }
  const path = join(work, "release.yml");
  writeFileSync(path, `${lines.join("\n")}\n`);
  return path;
}

function detectHostPlatform() {
  if (process.platform === "darwin" && process.arch === "arm64") return "darwin-arm64";
  if (process.platform === "darwin" && process.arch === "x64") return "darwin-x64";
  if (process.platform === "linux" && process.arch === "x64") return "linux-x64";
  if (process.platform === "linux" && process.arch === "arm64") return "linux-arm64";
  if (process.platform === "win32" && process.arch === "x64") return "win32-x64";
  throw new Error(`unsupported host platform ${process.platform}-${process.arch}`);
}

function foreignPlatform() {
  return hostPlatform === "linux-x64" ? "darwin-arm64" : "linux-x64";
}

function expectFail(script, args, expected) {
  const result = spawnSync("node", [script, ...args], { encoding: "utf8" });
  if (result.status === 0) {
    throw new Error(`expected failure but verifier exited 0\nstdout=${result.stdout}\nstderr=${result.stderr}`);
  }
  const combined = `${result.stdout}\n${result.stderr}`;
  if (!expected.test(combined)) {
    throw new Error(`expected stderr to match ${expected}\ngot=${combined}`);
  }
}

function expectSuccess(script, args, expected) {
  const result = spawnSync("node", [script, ...args], { encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(`expected success but verifier exited ${result.status}\nstdout=${result.stdout}\nstderr=${result.stderr}`);
  }
  if (!expected.test(result.stdout)) {
    throw new Error(`expected stdout to match ${expected}\ngot=${result.stdout}`);
  }
}
