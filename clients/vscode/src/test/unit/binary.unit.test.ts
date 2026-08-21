// Unit tests for manifest-backed binary resolution.

import * as assert from "node:assert/strict";
import {
  resolveBinary,
  resolveHostBinaries,
  loadDeploymentManifest,
  BundledBinaryMissingError,
  BinaryMissingError,
  UnsupportedPlatformError,
  BinaryVerificationError,
  type DeploymentManifest,
} from "../../binary";
import { mkdirSync, mkdtempSync, writeFileSync, chmodSync, rmSync, existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const PRODUCT_ID = "deslop";
const LSP_BINARY_NAME = "deslop-lsp";
const MCP_BINARY_NAME = "deslop-mcp";
const LSP_KIND = "lsp";
const MCP_KIND = "mcp";
const EXPECTED_VERSION = "0.1.0";
const MISMATCH_VERSION = "9.9.9";
const BUNDLED_SOURCE = "bundled";
const PATH_ENV_VAR = "PATH";
const BINARY_DIR_ENV_VAR = "DESLOP_BINARY_DIR";
const BINARY_DIRECTORY = "bin";
const DARWIN_PLATFORM = "darwin";
const LINUX_PLATFORM = "linux";
const ARM64_ARCH = "arm64";
const X64_ARCH = "x64";
const EXECUTABLE_MODE = 0o755;
const MISSING_BINARY_PATH = "/nope";
const EMPTY_PATH_ENV_VALUE = "";

function platformId(): string {
  if (process.platform === DARWIN_PLATFORM && process.arch === ARM64_ARCH) return "darwin-arm64";
  if (process.platform === DARWIN_PLATFORM && process.arch === X64_ARCH) return "darwin-x64";
  if (process.platform === LINUX_PLATFORM && process.arch === X64_ARCH) return "linux-x64";
  if (process.platform === LINUX_PLATFORM && process.arch === ARM64_ARCH) return "linux-arm64";
  if (process.platform === "win32") return "win32-x64";
  throw new Error(`unsupported ${process.platform}-${process.arch}`);
}

function writeVersionScript(filePath: string, name: string, version: string): void {
  writeFileSync(filePath, `#!/bin/sh\necho '${name} ${version}'\n`);
  chmodSync(filePath, EXECUTABLE_MODE);
}

// A binary whose FIRST exec stalls past the warm probe budget and whose
// second answers instantly — macOS Gatekeeper validating a freshly installed
// unsigned binary, reproduced with a marker file instead of wall-clock luck.
function writeFirstExecStallScript(filePath: string, name: string, version: string): void {
  const marker = `${filePath}.warm`;
  writeFileSync(
    filePath,
    `#!/bin/sh\nif [ ! -f '${marker}' ]; then touch '${marker}'; sleep 3; fi\necho '${name} ${version}'\n`,
  );
  chmodSync(filePath, EXECUTABLE_MODE);
}

function manifest(): DeploymentManifest {
  return {
    manifestVersion: 1,
    product: { id: PRODUCT_ID, version: EXPECTED_VERSION },
    components: [
      component(LSP_BINARY_NAME, LSP_KIND, "DESLOP_LSP_PATH"),
      component(MCP_BINARY_NAME, MCP_KIND, "DESLOP_MCP_PATH"),
      component(PRODUCT_ID, "cli", undefined),
    ],
    hosts: { vscode: { activationVerifies: [LSP_BINARY_NAME, MCP_BINARY_NAME] } },
  };
}

function component(id: string, kind: string, pathVar: string | undefined) {
  const env = pathVar
    ? { pathVar, dirVar: BINARY_DIR_ENV_VAR }
    : { dirVar: BINARY_DIR_ENV_VAR };
  return {
    id,
    kind,
    language: "rust",
    binaryName: id,
    expectedVersion: EXPECTED_VERSION,
    bundled: { bundlePath: "bin/${platform}/${binaryName}${exe}" },
    env,
    required: true,
  };
}

// [DEPLOY-RESOLVER]
suite("binary resolver", () => {
  // mkdtemp, not a name built from pid + clock. This suite writes shell scripts
  // and then *executes* them, and the OS temp dir is world-writable: a guessable
  // path lets another local process pre-create or symlink these entries and
  // choose what the test runner executes (js/insecure-temporary-file). mkdtemp
  // gives a 0700 dir with an unguessable suffix, as every sibling suite uses.
  const tmp = mkdtempSync(join(tmpdir(), "deslop-binary-"));
  const envDir = resolve(tmp, "env");
  const pathDir = resolve(tmp, "pathdir");
  const userDir = resolve(tmp, "user");
  const extDir = resolve(tmp, "ext");
  const bundledDir = resolve(extDir, BINARY_DIRECTORY, platformId());

  suiteSetup(() => {
    mkdirSync(envDir, { recursive: true });
    mkdirSync(pathDir, { recursive: true });
    mkdirSync(userDir, { recursive: true });
    mkdirSync(bundledDir, { recursive: true });
    writeVersionScript(resolve(envDir, LSP_BINARY_NAME), LSP_BINARY_NAME, MISMATCH_VERSION);
    writeVersionScript(resolve(envDir, MCP_BINARY_NAME), MCP_BINARY_NAME, EXPECTED_VERSION);
    writeVersionScript(resolve(pathDir, LSP_BINARY_NAME), LSP_BINARY_NAME, MISMATCH_VERSION);
    writeVersionScript(resolve(bundledDir, LSP_BINARY_NAME), LSP_BINARY_NAME, EXPECTED_VERSION);
    writeVersionScript(resolve(bundledDir, MCP_BINARY_NAME), MCP_BINARY_NAME, EXPECTED_VERSION);
    writeVersionScript(resolve(bundledDir, PRODUCT_ID), PRODUCT_ID, EXPECTED_VERSION);
    writeVersionScript(resolve(userDir, LSP_BINARY_NAME), LSP_BINARY_NAME, MISMATCH_VERSION);
  });

  suiteTeardown(() => {
    rmSync(tmp, { recursive: true, force: true });
  });

  test("user setting mismatch blocks activation", () => {
    assert.throws(
      () =>
        resolveBinary(extDir, LSP_KIND, manifest(), {
          lspPath: resolve(userDir, LSP_BINARY_NAME),
        }),
      BinaryVerificationError,
    );
  });

  // [DEPLOY-RESOLVER] A probe that never replies is INCONCLUSIVE, not a
  // mismatch. Every bundled binary in a just-installed VSIX is on its first
  // exec, and macOS validates unsigned ~30 MB files before running them; when
  // that outran the warm budget the resolver reported "version mismatch",
  // activation bailed before registerCommands, and the extension was dead
  // until reload. This pins the retry that makes first activation survive.
  test("a first exec that outruns the warm probe budget still resolves", () => {
    const stalling = resolve(userDir, "stalling-lsp");
    writeFirstExecStallScript(stalling, LSP_BINARY_NAME, EXPECTED_VERSION);

    const resolved = resolveBinary(
      extDir,
      LSP_KIND,
      manifest(),
      { lspPath: stalling },
      { [PATH_ENV_VAR]: EMPTY_PATH_ENV_VALUE },
    );

    assert.equal(
      resolved.version,
      EXPECTED_VERSION,
      "the retry must read the version the stalled first probe missed",
    );
    assert.equal(resolved.source, "user-setting", "the override must still win the candidate race");
    assert.equal(resolved.path, stalling);
    assert.equal(resolved.componentId, LSP_BINARY_NAME);
    assert.ok(
      existsSync(`${stalling}.warm`),
      "the first exec must genuinely have run and stalled — otherwise this proves nothing",
    );
  });

  test("env path mismatch blocks activation", () => {
    const env: NodeJS.ProcessEnv = { DESLOP_LSP_PATH: resolve(envDir, LSP_BINARY_NAME) };
    assert.throws(() => resolveBinary(extDir, LSP_KIND, manifest(), {}, env), /9\.9\.9/);
  });

  test("env directory mismatch blocks activation", () => {
    const env: NodeJS.ProcessEnv = { [BINARY_DIR_ENV_VAR]: envDir };
    assert.throws(() => resolveBinary(extDir, LSP_KIND, manifest(), {}, env), /env-dir/);
  });

  // [DEPLOY-RESOLVER]
  test("PATH candidates are ignored when the bundle is present", () => {
    const env: NodeJS.ProcessEnv = { [PATH_ENV_VAR]: pathDir };
    const resolved = resolveBinary(extDir, LSP_KIND, manifest(), {}, env);
    assert.equal(resolved.source, BUNDLED_SOURCE);
    assert.equal(resolved.version, EXPECTED_VERSION);
    assert.equal(resolved.path, resolve(bundledDir, LSP_BINARY_NAME));
    assert.equal(env[PATH_ENV_VAR], pathDir);
  });

  test("bundled binary resolution keeps PATH unchanged", () => {
    const env: NodeJS.ProcessEnv = { [PATH_ENV_VAR]: pathDir };
    const before = env[PATH_ENV_VAR];
    const resolved = resolveBinary(extDir, MCP_KIND, manifest(), {}, env);
    assert.equal(resolved.source, BUNDLED_SOURCE);
    assert.equal(resolved.path, resolve(bundledDir, MCP_BINARY_NAME));
    assert.equal(env[PATH_ENV_VAR], before);
  });

  // [VSIX-BUNDLED-BINARY-TESTS]
  test("bundled success resolves all VS Code activation checks", () => {
    const resolved = resolveHostBinaries(
      extDir,
      "vscode",
      manifest(),
      {},
      { [PATH_ENV_VAR]: EMPTY_PATH_ENV_VALUE },
    );
    assert.equal(resolved[LSP_BINARY_NAME]?.source, BUNDLED_SOURCE);
    assert.equal(resolved[MCP_BINARY_NAME]?.source, BUNDLED_SOURCE);
  });

  test("missing bundled binary blocks activation", () => {
    const emptyExt = resolve(tmp, "empty-ext");
    mkdirSync(resolve(emptyExt, BINARY_DIRECTORY, platformId()), { recursive: true });
    assert.throws(
      () =>
        resolveBinary(emptyExt, LSP_KIND, manifest(), {}, {
          [PATH_ENV_VAR]: EMPTY_PATH_ENV_VALUE,
        }),
      BundledBinaryMissingError,
    );
  });

  test("binary name mismatch blocks activation", () => {
    const mismatchExt = resolve(tmp, "mismatch-ext");
    const mismatchBin = resolve(mismatchExt, BINARY_DIRECTORY, platformId());
    mkdirSync(mismatchBin, { recursive: true });
    writeVersionScript(resolve(mismatchBin, LSP_BINARY_NAME), PRODUCT_ID, EXPECTED_VERSION);
    assert.throws(
      () =>
        resolveBinary(mismatchExt, LSP_KIND, manifest(), {}, {
          [PATH_ENV_VAR]: EMPTY_PATH_ENV_VALUE,
        }),
      /Found deslop 0\.1\.0/,
    );
  });

  test("bundled version mismatch blocks activation", () => {
    const staleExt = resolve(tmp, "stale-ext");
    const staleBin = resolve(staleExt, BINARY_DIRECTORY, platformId());
    mkdirSync(staleBin, { recursive: true });
    writeVersionScript(resolve(staleBin, LSP_BINARY_NAME), LSP_BINARY_NAME, MISMATCH_VERSION);
    assert.throws(
      () =>
        resolveBinary(staleExt, LSP_KIND, manifest(), {}, {
          [PATH_ENV_VAR]: EMPTY_PATH_ENV_VALUE,
        }),
      /Expected 0\.1\.0/,
    );
  });

  test("UnsupportedPlatformError has expected shape", () => {
    const err = new UnsupportedPlatformError("nosuch", "arm");
    assert.match(err.message, /no bundled binary for nosuch-arm/);
  });

  test("BundledBinaryMissingError exposes path", () => {
    const err = new BundledBinaryMissingError(MISSING_BINARY_PATH);
    assert.equal(err.binaryPath, MISSING_BINARY_PATH);
  });

  test("loadDeploymentManifest reads and parses the packaged shipwright.json", () => {
    const manifestExt = resolve(tmp, "manifest-ext");
    mkdirSync(manifestExt, { recursive: true });
    writeFileSync(resolve(manifestExt, "shipwright.json"), JSON.stringify(manifest()), "utf8");

    const loaded = loadDeploymentManifest(manifestExt);
    assert.equal(loaded.product.id, PRODUCT_ID);
    assert.equal(loaded.hosts.vscode?.activationVerifies.includes(LSP_BINARY_NAME), true);
  });

  test("a configured-but-missing override path raises BinaryMissingError", () => {
    // The user-setting candidate is a hard failure: a path the user named
    // explicitly that does not exist must abort activation, not silently
    // fall through to the bundle.
    assert.throws(
      () =>
        resolveBinary(
          extDir,
          LSP_KIND,
          manifest(),
          { lspPath: resolve(tmp, "nonexistent", LSP_BINARY_NAME) },
          { [PATH_ENV_VAR]: EMPTY_PATH_ENV_VALUE },
        ),
      (err: unknown) =>
        err instanceof BinaryMissingError && /was not found at/.test(err.message),
    );
  });
});
