// Proof tests for deployment-toolkit verifier scripts.
//
// These cover DTK-MIG-DESLOP-CI-GATES (#41), DTK-MIG-DESLOP-JETBRAINS-PACKAGE-VERIFIER
// (#55), and the version-check acceptance criterion shared with
// DTK-MIG-DESLOP-JETBRAINS-RESOLVER (#40). Each verifier script is fed a fake
// binary or fake plugin zip that violates exactly one Shipwright contract
// rule (wrong version, wrong component id, missing binary, missing manifest,
// undeclared bundled binary) and the test asserts the verifier exits non-zero
// with a message that names the violation. A passing run proves the verifier
// is not silently green on broken inputs — the version gate has bite.

import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync, copyFileSync, chmodSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const repoRoot = resolve(new URL("..", import.meta.url).pathname);
const verifyManifest = join(repoRoot, "scripts/verify-deployment-manifest.mjs");
const verifyBinaries = join(repoRoot, "scripts/verify-deployment-binaries.mjs");
const verifyJetBrains = join(repoRoot, "scripts/verify-jetbrains-package.mjs");
const verifyVsix = join(repoRoot, "clients/vscode/scripts/verify-vsix-package.mjs");
const platform = "darwin-arm64";
// verify-vsix-package.mjs hard-codes currentPlatform() and has no override
// arg, so the fake-VSIX cases must stage binaries under the host platform.
const hostPlatform = detectHostPlatform();

const cases = [
  manifestRejectsMissingProductId,
  manifestRejectsLooseSemver,
  manifestRejectsHostVerifyingUnknownComponent,
  manifestAcceptsRepoManifest,
  binariesRejectWrongVersion,
  binariesRejectWrongComponentName,
  binariesRejectMissingBinary,
  binariesRejectStaleJsonManifestVersion,
  binariesAcceptValidContract,
  jetbrainsRejectsMissingManifest,
  jetbrainsRejectsMissingBundledBinary,
  jetbrainsRejectsWrongVersionBundle,
  jetbrainsRejectsWrongComponentNameBundle,
  jetbrainsRejectsUndeclaredBundle,
  jetbrainsAcceptsValidPackage,
  vsixRejectsMissingManifest,
  vsixRejectsMissingBundledLsp,
  vsixRejectsMissingBundledMcp,
  vsixRejectsWrongVersionBundle,
  vsixRejectsWrongComponentNameBundle,
  vsixRejectsUndeclaredBundle,
  vsixAcceptsValidPackage,
];

let failed = 0;
for (const test of cases) {
  const work = mkdtempSync(join(tmpdir(), "deslop-verifier-"));
  try {
    test(work);
    console.log(`ok ${test.name}`);
  } catch (error) {
    failed++;
    console.error(`not ok ${test.name}`);
    console.error(`  ${error instanceof Error ? error.message : String(error)}`);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

if (failed > 0) {
  console.error(`\n${failed} verifier proof test(s) failed`);
  process.exit(1);
}
console.log(`\n${cases.length} verifier proof tests passed`);

// ---------- manifest verifier ----------

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
      product: { id: "deslop", version: "0.1.0" },
      components: [
        {
          id: "deslop",
          kind: "cli",
          language: "rust",
          binaryName: "deslop",
          expectedVersion: "0.1.0",
          versionCheckStrategy: "version-flag",
        },
      ],
      hosts: { vscode: { activationVerifies: ["does-not-exist"] } },
    }),
  );
  expectFail(verifyManifest, [path], /unknown component does-not-exist/);
}

function manifestAcceptsRepoManifest() {
  const path = join(repoRoot, "deployment-toolkit.json");
  expectSuccess(verifyManifest, [path], /valid deployment manifest/);
}

// ---------- binary verifier ----------

function binariesRejectWrongVersion(work) {
  const manifestPath = writeManifestWithDeslopComponent(work, { expectedVersion: "0.1.0" });
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
    plain: "wrong-name 0.1.0",
    json: { manifestVersion: 1, name: "wrong-name", version: "0.1.0", kind: "cli", language: "rust", product: "deslop" },
  });
  expectFail(verifyBinaries, [manifestPath, binDir, platform], /reported wrong-name 0\.1\.0/);
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
    plain: "deslop 0.1.0",
    json: { manifestVersion: 999, name: "deslop", version: "0.1.0", kind: "cli", language: "rust", product: "deslop" },
  });
  expectFail(verifyBinaries, [manifestPath, binDir, platform], /manifestVersion must be 1/);
}

function binariesAcceptValidContract(work) {
  const manifestPath = writeManifestWithDeslopComponent(work, {});
  const binDir = join(work, "bin");
  mkdirSync(binDir);
  writeFakeBinary(join(binDir, "deslop"), {
    plain: "deslop 0.1.0",
    json: { manifestVersion: 1, name: "deslop", version: "0.1.0", kind: "cli", language: "rust", product: "deslop" },
  });
  expectSuccess(verifyBinaries, [manifestPath, binDir, platform], /Verified deployment binaries/);
}

// ---------- jetbrains package verifier ----------

function jetbrainsRejectsMissingManifest(work) {
  const zipPath = buildJetBrainsZip(work, { manifest: null });
  expectFail(verifyJetBrains, [zipPath, platform], /Missing .*deployment-toolkit\.json/);
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
  expectFail(verifyJetBrains, [zipPath, platform], /reported deslop 0\.1\.0/);
}

function jetbrainsRejectsUndeclaredBundle(work) {
  const zipPath = buildJetBrainsZip(work, { extraBinName: "rogue-helper" });
  expectFail(verifyJetBrains, [zipPath, platform], /Undeclared JetBrains binary/);
}

function jetbrainsAcceptsValidPackage(work) {
  const zipPath = buildJetBrainsZip(work, {});
  expectSuccess(verifyJetBrains, [zipPath, platform], /Verified JetBrains package/);
}

// ---------- vsix package verifier ----------

function vsixRejectsMissingManifest(work) {
  const vsixPath = buildVsixZip(work, { manifest: null });
  expectFail(verifyVsix, [vsixPath], /Missing extension\/deployment-toolkit\.json/);
}

function vsixRejectsMissingBundledLsp(work) {
  const vsixPath = buildVsixZip(work, { skipBundledLsp: true });
  expectFail(verifyVsix, [vsixPath], /Missing extension\/bin\/.*\/deslop-lsp/);
}

function vsixRejectsMissingBundledMcp(work) {
  const vsixPath = buildVsixZip(work, { skipBundledMcp: true });
  expectFail(verifyVsix, [vsixPath], /Missing extension\/bin\/.*\/deslop-mcp/);
}

function vsixRejectsWrongVersionBundle(work) {
  const vsixPath = buildVsixZip(work, { lspVersion: "0.0.9" });
  expectFail(verifyVsix, [vsixPath], /reported deslop-lsp 0\.0\.9/);
}

function vsixRejectsWrongComponentNameBundle(work) {
  const vsixPath = buildVsixZip(work, { lspName: "wrong-name" });
  expectFail(verifyVsix, [vsixPath], /reported wrong-name 0\.1\.0/);
}

function vsixRejectsUndeclaredBundle(work) {
  const vsixPath = buildVsixZip(work, { extraBinName: "rogue-helper" });
  expectFail(verifyVsix, [vsixPath], /Undeclared executable in VSIX/);
}

function vsixAcceptsValidPackage(work) {
  const vsixPath = buildVsixZip(work, {});
  expectSuccess(verifyVsix, [vsixPath], /Verified deployment manifest/);
}

// ---------- helpers ----------

function writeManifestWithDeslopComponent(work, overrides) {
  const path = join(work, "manifest.json");
  writeFileSync(
    path,
    JSON.stringify({
      manifestVersion: 1,
      product: { id: "deslop", version: "0.1.0" },
      components: [
        {
          id: "deslop",
          kind: "cli",
          language: "rust",
          binaryName: "deslop",
          expectedVersion: overrides.expectedVersion ?? "0.1.0",
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
    const manifestSource = options.manifest ?? join(repoRoot, "deployment-toolkit.json");
    copyFileSync(manifestSource, join(pluginRoot, "deployment-toolkit.json"));
  }

  if (!options.skipBundledLsp) {
    const name = options.lspName ?? "deslop-lsp";
    const version = options.lspVersion ?? "0.1.0";
    writeFakeBinary(join(binDir, "deslop-lsp"), {
      plain: `${name} ${version}`,
      json: { manifestVersion: 1, name, version, kind: "lsp", language: "rust", product: "deslop" },
    });
  }

  if (options.extraBinName) {
    writeFakeBinary(join(binDir, options.extraBinName), {
      plain: `${options.extraBinName} 0.1.0`,
      json: { manifestVersion: 1, name: options.extraBinName, version: "0.1.0", kind: "cli", language: "rust", product: options.extraBinName },
    });
  }

  const zipPath = join(work, "plugin.zip");
  const result = spawnSync("zip", ["-rq", zipPath, "deslop-jetbrains"], {
    cwd: stagingRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) throw new Error(`zip failed: ${result.stderr}`);
  return zipPath;
}

function buildVsixZip(work, options) {
  const stagingRoot = join(work, "stage");
  const extensionRoot = join(stagingRoot, "extension");
  const binDir = join(extensionRoot, "bin", hostPlatform);
  mkdirSync(binDir, { recursive: true });

  if (options.manifest !== null) {
    const manifestSource = options.manifest ?? join(repoRoot, "deployment-toolkit.json");
    copyFileSync(manifestSource, join(extensionRoot, "deployment-toolkit.json"));
  }

  if (!options.skipBundledLsp) {
    const name = options.lspName ?? "deslop-lsp";
    const version = options.lspVersion ?? "0.1.0";
    writeFakeBinary(join(binDir, "deslop-lsp"), {
      plain: `${name} ${version}`,
      json: { manifestVersion: 1, name, version, kind: "lsp", language: "rust", product: "deslop" },
    });
  }

  if (!options.skipBundledMcp) {
    writeFakeBinary(join(binDir, "deslop-mcp"), {
      plain: "deslop-mcp 0.1.0",
      json: { manifestVersion: 1, name: "deslop-mcp", version: "0.1.0", kind: "mcp", language: "rust", product: "deslop" },
    });
  }

  if (options.extraBinName) {
    writeFakeBinary(join(binDir, options.extraBinName), {
      plain: `${options.extraBinName} 0.1.0`,
      json: { manifestVersion: 1, name: options.extraBinName, version: "0.1.0", kind: "cli", language: "rust", product: options.extraBinName },
    });
  }

  const vsixPath = join(work, "package.vsix");
  const result = spawnSync("zip", ["-rq", vsixPath, "extension"], {
    cwd: stagingRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) throw new Error(`zip failed: ${result.stderr}`);
  return vsixPath;
}

function detectHostPlatform() {
  if (process.platform === "darwin" && process.arch === "arm64") return "darwin-arm64";
  if (process.platform === "darwin" && process.arch === "x64") return "darwin-x64";
  if (process.platform === "linux" && process.arch === "x64") return "linux-x64";
  if (process.platform === "linux" && process.arch === "arm64") return "linux-arm64";
  if (process.platform === "win32" && process.arch === "x64") return "win32-x64";
  throw new Error(`unsupported host platform ${process.platform}-${process.arch}`);
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
