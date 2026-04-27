// Unit tests for manifest-backed binary resolution.

import * as assert from "node:assert/strict";
import {
  resolveBinary,
  resolveHostBinaries,
  BundledBinaryMissingError,
  UnsupportedPlatformError,
  BinaryVerificationError,
  type DeploymentManifest,
} from "../../binary";
import { mkdirSync, writeFileSync, chmodSync, rmSync } from "node:fs";
import { delimiter, resolve } from "node:path";
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

suite("binary resolver", () => {
  const tmp = resolve(tmpdir(), `deslop-binary-${process.pid}-${Date.now()}`);
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

  test("env path mismatch blocks activation", () => {
    const env: NodeJS.ProcessEnv = { DESLOP_LSP_PATH: resolve(envDir, "deslop-lsp") };
    assert.throws(() => resolveBinary(extDir, "lsp", manifest(), {}, env), /9\.9\.9/);
  });

  test("env directory mismatch blocks activation", () => {
    const env: NodeJS.ProcessEnv = { DESLOP_BINARY_DIR: envDir };
    assert.throws(() => resolveBinary(extDir, "lsp", manifest(), {}, env), /env-dir/);
  });

  test("PATH mismatch falls back to matching bundled binary", () => {
    const env: NodeJS.ProcessEnv = { PATH: pathDir };
    const resolved = resolveBinary(extDir, "lsp", manifest(), {}, env);
    assert.equal(resolved.source, "bundled");
    assert.equal(resolved.version, "0.1.0");
    assert.ok(env["PATH"]?.split(delimiter).includes(bundledDir) ?? false);
  });

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
});
