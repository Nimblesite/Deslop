// Unit tests for the binary resolver. Covers every branch of the env→PATH→bundled
// decision tree without needing VS Code.

import * as assert from "node:assert/strict";
import {
  resolveBinary,
  BundledBinaryMissingError,
  UnsupportedPlatformError,
} from "../../src/binary";
import { mkdirSync, writeFileSync, chmodSync, rmSync } from "node:fs";
import { resolve } from "node:path";
import { tmpdir } from "node:os";

function platformId(): string {
  if (process.platform === "darwin" && process.arch === "arm64") return "darwin-arm64";
  if (process.platform === "darwin" && process.arch === "x64") return "darwin-x64";
  if (process.platform === "linux" && process.arch === "x64") return "linux-x64";
  if (process.platform === "linux" && process.arch === "arm64") return "linux-arm64";
  if (process.platform === "win32") return "win32-x64";
  throw new Error(`unsupported ${process.platform}-${process.arch}`);
}

function writeVersionScript(path: string, version: string): void {
  writeFileSync(path, `#!/bin/sh\necho 'codededup ${version}'\n`);
  chmodSync(path, 0o755);
}

suite("binary resolver", () => {
  const tmp = resolve(tmpdir(), `codededup-binary-${process.pid}-${Date.now()}`);
  const envDir = resolve(tmp, "env");
  const pathDir = resolve(tmp, "pathdir");
  const extDir = resolve(tmp, "ext");
  const bundledDir = resolve(extDir, "bin", platformId());

  suiteSetup(() => {
    mkdirSync(envDir, { recursive: true });
    mkdirSync(pathDir, { recursive: true });
    mkdirSync(bundledDir, { recursive: true });
    writeVersionScript(resolve(envDir, "codededup-lsp"), "9.9.9");
    writeVersionScript(resolve(pathDir, "codededup-lsp"), "0.1.0");
    writeVersionScript(resolve(bundledDir, "codededup-lsp"), "0.1.0");
    writeVersionScript(resolve(envDir, "codededup-mcp"), "9.9.9");
    writeVersionScript(resolve(bundledDir, "codededup-mcp"), "0.1.0");
    writeVersionScript(resolve(bundledDir, "codededup"), "0.1.0");
  });

  suiteTeardown(() => {
    try {
      rmSync(tmp, { recursive: true, force: true });
    } catch {
      /* ignore */
    }
  });

  test("CODEDEDUP_BINARY_DIR wins over PATH + bundled", () => {
    const env = { ...process.env, CODEDEDUP_BINARY_DIR: envDir };
    const resolved = resolveBinary(extDir, "lsp", "0.1.0", env);
    assert.equal(resolved.source, "env");
    assert.equal(resolved.kind, "lsp");
    assert.ok(resolved.path.startsWith(envDir));
  });

  test("PATH wins when version matches", () => {
    const env: NodeJS.ProcessEnv = { ...process.env, PATH: pathDir };
    delete env["CODEDEDUP_BINARY_DIR"];
    const resolved = resolveBinary(extDir, "lsp", "0.1.0", env);
    assert.equal(resolved.source, "path");
    assert.equal(resolved.version, "0.1.0");
  });

  test("PATH falls back to bundled when version mismatches", () => {
    const env = { ...process.env, PATH: pathDir } as NodeJS.ProcessEnv;
    delete env["CODEDEDUP_BINARY_DIR"];
    const resolved = resolveBinary(extDir, "lsp", "9.9.9", env);
    assert.equal(resolved.source, "bundled");
    assert.ok(env["PATH"]?.split(":").includes(bundledDir) || env["PATH"]?.split(";").includes(bundledDir));
  });

  test("bundled used when neither env nor PATH has the binary", () => {
    const env = { ...process.env, PATH: "/nonexistent" } as NodeJS.ProcessEnv;
    delete env["CODEDEDUP_BINARY_DIR"];
    const resolved = resolveBinary(extDir, "lsp", "0.1.0", env);
    assert.equal(resolved.source, "bundled");
  });

  test("bundled-missing throws BundledBinaryMissingError", () => {
    const env = { ...process.env, PATH: "" } as NodeJS.ProcessEnv;
    delete env["CODEDEDUP_BINARY_DIR"];
    const emptyExt = resolve(tmp, "empty-ext");
    mkdirSync(resolve(emptyExt, "bin", platformId()), { recursive: true });
    assert.throws(
      () => resolveBinary(emptyExt, "lsp", "0.1.0", env),
      BundledBinaryMissingError,
    );
  });

  test("mcp resolves alongside lsp", () => {
    const env = { ...process.env, CODEDEDUP_BINARY_DIR: envDir };
    const resolved = resolveBinary(extDir, "mcp", "0.1.0", env);
    assert.equal(resolved.kind, "mcp");
    assert.equal(resolved.source, "env");
  });

  test("cli kind resolves to bundled codededup", () => {
    const env = { ...process.env, PATH: "" } as NodeJS.ProcessEnv;
    delete env["CODEDEDUP_BINARY_DIR"];
    const resolved = resolveBinary(extDir, "cli", "0.1.0", env);
    assert.equal(resolved.kind, "cli");
    assert.equal(resolved.source, "bundled");
  });

  test("UnsupportedPlatformError has expected shape", () => {
    const err = new UnsupportedPlatformError("nosuch", "arm");
    assert.match(err.message, /no bundled binary for nosuch-arm/);
  });

  test("BundledBinaryMissingError exposes path", () => {
    const err = new BundledBinaryMissingError("/nope");
    assert.equal(err.binaryPath, "/nope");
  });

  test("env dir without binary falls through to PATH", () => {
    const emptyEnv = resolve(tmp, "empty-env");
    mkdirSync(emptyEnv, { recursive: true });
    const env = { ...process.env, CODEDEDUP_BINARY_DIR: emptyEnv, PATH: pathDir };
    const resolved = resolveBinary(extDir, "lsp", "0.1.0", env);
    assert.equal(resolved.source, "path");
  });

  test("prependToPath is idempotent", () => {
    const env = { ...process.env, PATH: "" } as NodeJS.ProcessEnv;
    delete env["CODEDEDUP_BINARY_DIR"];
    resolveBinary(extDir, "lsp", "0.1.0", env);
    const first = env["PATH"];
    resolveBinary(extDir, "lsp", "0.1.0", env);
    assert.equal(env["PATH"], first);
  });
});
