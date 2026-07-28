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

function platformId(): string {
  if (process.platform === "darwin" && process.arch === "arm64") return "darwin-arm64";
  if (process.platform === "darwin" && process.arch === "x64") return "darwin-x64";
  if (process.platform === "linux" && process.arch === "x64") return "linux-x64";
  if (process.platform === "linux" && process.arch === "arm64") return "linux-arm64";
  if (process.platform === "win32") return "win32-x64";
  throw new Error(`unsupported ${process.platform}-${process.arch}`);
}

function writeVersionScript(filePath: string, name: string, version: string): void {
  writeFileSync(filePath, `#!/bin/sh\necho '${name} ${version}'\n`);
  chmodSync(filePath, 0o755);
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
  chmodSync(filePath, 0o755);
}

function manifest(): DeploymentManifest {
  return {
    manifestVersion: 1,
    product: { id: "deslop", version: "0.1.0" },
    components: [
      component("deslop-lsp", "lsp", "DESLOP_LSP_PATH"),
      component("deslop-mcp", "mcp", "DESLOP_MCP_PATH"),
      component("deslop", "cli", undefined),
    ],
    hosts: { vscode: { activationVerifies: ["deslop-lsp", "deslop-mcp"] } },
  };
}

function component(id: string, kind: string, pathVar: string | undefined) {
  const env = pathVar
    ? { pathVar, dirVar: "DESLOP_BINARY_DIR" }
    : { dirVar: "DESLOP_BINARY_DIR" };
  return {
    id,
    kind,
    language: "rust",
    binaryName: id,
    expectedVersion: "0.1.0",
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
  const bundledDir = resolve(extDir, "bin", platformId());

  suiteSetup(() => {
    mkdirSync(envDir, { recursive: true });
    mkdirSync(pathDir, { recursive: true });
    mkdirSync(userDir, { recursive: true });
    mkdirSync(bundledDir, { recursive: true });
    writeVersionScript(resolve(envDir, "deslop-lsp"), "deslop-lsp", "9.9.9");
    writeVersionScript(resolve(envDir, "deslop-mcp"), "deslop-mcp", "0.1.0");
    writeVersionScript(resolve(pathDir, "deslop-lsp"), "deslop-lsp", "9.9.9");
    writeVersionScript(resolve(bundledDir, "deslop-lsp"), "deslop-lsp", "0.1.0");
    writeVersionScript(resolve(bundledDir, "deslop-mcp"), "deslop-mcp", "0.1.0");
    writeVersionScript(resolve(bundledDir, "deslop"), "deslop", "0.1.0");
    writeVersionScript(resolve(userDir, "deslop-lsp"), "deslop-lsp", "9.9.9");
  });

  suiteTeardown(() => {
    rmSync(tmp, { recursive: true, force: true });
  });

  test("user setting mismatch blocks activation", () => {
    assert.throws(
      () =>
        resolveBinary(extDir, "lsp", manifest(), {
          lspPath: resolve(userDir, "deslop-lsp"),
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
    writeFirstExecStallScript(stalling, "deslop-lsp", "0.1.0");

    const resolved = resolveBinary(extDir, "lsp", manifest(), { lspPath: stalling }, { PATH: "" });

    assert.equal(
      resolved.version,
      "0.1.0",
      "the retry must read the version the stalled first probe missed",
    );
    assert.equal(resolved.source, "user-setting", "the override must still win the candidate race");
    assert.equal(resolved.path, stalling);
    assert.equal(resolved.componentId, "deslop-lsp");
    assert.ok(
      existsSync(`${stalling}.warm`),
      "the first exec must genuinely have run and stalled — otherwise this proves nothing",
    );
  });

  test("env path mismatch blocks activation", () => {
    const env: NodeJS.ProcessEnv = { DESLOP_LSP_PATH: resolve(envDir, "deslop-lsp") };
    assert.throws(() => resolveBinary(extDir, "lsp", manifest(), {}, env), /9\.9\.9/);
  });

  test("env directory mismatch blocks activation", () => {
    const env: NodeJS.ProcessEnv = { DESLOP_BINARY_DIR: envDir };
    assert.throws(() => resolveBinary(extDir, "lsp", manifest(), {}, env), /env-dir/);
  });

  // [DEPLOY-RESOLVER]
  test("PATH candidates are ignored when the bundle is present", () => {
    const env: NodeJS.ProcessEnv = { PATH: pathDir };
    const resolved = resolveBinary(extDir, "lsp", manifest(), {}, env);
    assert.equal(resolved.source, "bundled");
    assert.equal(resolved.version, "0.1.0");
    assert.equal(resolved.path, resolve(bundledDir, "deslop-lsp"));
    assert.equal(env["PATH"], pathDir);
  });

  test("bundled binary resolution keeps PATH unchanged", () => {
    const env: NodeJS.ProcessEnv = { PATH: pathDir };
    const before = env["PATH"];
    const resolved = resolveBinary(extDir, "mcp", manifest(), {}, env);
    assert.equal(resolved.source, "bundled");
    assert.equal(resolved.path, resolve(bundledDir, "deslop-mcp"));
    assert.equal(env["PATH"], before);
  });

  // [VSIX-BUNDLED-BINARY-TESTS]
  test("bundled success resolves all VS Code activation checks", () => {
    const resolved = resolveHostBinaries(extDir, "vscode", manifest(), {}, { PATH: "" });
    assert.equal(resolved["deslop-lsp"]?.source, "bundled");
    assert.equal(resolved["deslop-mcp"]?.source, "bundled");
  });

  test("missing bundled binary blocks activation", () => {
    const emptyExt = resolve(tmp, "empty-ext");
    mkdirSync(resolve(emptyExt, "bin", platformId()), { recursive: true });
    assert.throws(
      () => resolveBinary(emptyExt, "lsp", manifest(), {}, { PATH: "" }),
      BundledBinaryMissingError,
    );
  });

  test("binary name mismatch blocks activation", () => {
    const mismatchExt = resolve(tmp, "mismatch-ext");
    const mismatchBin = resolve(mismatchExt, "bin", platformId());
    mkdirSync(mismatchBin, { recursive: true });
    writeVersionScript(resolve(mismatchBin, "deslop-lsp"), "deslop", "0.1.0");
    assert.throws(
      () => resolveBinary(mismatchExt, "lsp", manifest(), {}, { PATH: "" }),
      /Found deslop 0\.1\.0/,
    );
  });

  test("bundled version mismatch blocks activation", () => {
    const staleExt = resolve(tmp, "stale-ext");
    const staleBin = resolve(staleExt, "bin", platformId());
    mkdirSync(staleBin, { recursive: true });
    writeVersionScript(resolve(staleBin, "deslop-lsp"), "deslop-lsp", "9.9.9");
    assert.throws(
      () => resolveBinary(staleExt, "lsp", manifest(), {}, { PATH: "" }),
      /Expected 0\.1\.0/,
    );
  });

  test("UnsupportedPlatformError has expected shape", () => {
    const err = new UnsupportedPlatformError("nosuch", "arm");
    assert.match(err.message, /no bundled binary for nosuch-arm/);
  });

  test("BundledBinaryMissingError exposes path", () => {
    const err = new BundledBinaryMissingError("/nope");
    assert.equal(err.binaryPath, "/nope");
  });

  test("loadDeploymentManifest reads and parses the packaged shipwright.json", () => {
    const manifestExt = resolve(tmp, "manifest-ext");
    mkdirSync(manifestExt, { recursive: true });
    writeFileSync(resolve(manifestExt, "shipwright.json"), JSON.stringify(manifest()), "utf8");

    const loaded = loadDeploymentManifest(manifestExt);
    assert.equal(loaded.product.id, "deslop");
    assert.equal(loaded.hosts["vscode"]?.activationVerifies.includes("deslop-lsp"), true);
  });

  test("a configured-but-missing override path raises BinaryMissingError", () => {
    // The user-setting candidate is a hard failure: a path the user named
    // explicitly that does not exist must abort activation, not silently
    // fall through to the bundle.
    assert.throws(
      () =>
        resolveBinary(
          extDir,
          "lsp",
          manifest(),
          { lspPath: resolve(tmp, "nonexistent", "deslop-lsp") },
          { PATH: "" },
        ),
      (err: unknown) =>
        err instanceof BinaryMissingError && /was not found at/.test(err.message),
    );
  });
});
