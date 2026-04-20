// E2E: binary resolver prefers ${CODEDEDUP_BINARY_DIR}, prepends bundled to PATH,
// and matches versions before accepting a PATH-installed binary.

import * as assert from "node:assert/strict";
import { resolveBinary } from "../../src/binary";
import { mkdirSync, writeFileSync, chmodSync, rmSync } from "node:fs";
import { resolve } from "node:path";
import { tmpdir } from "node:os";

suite("binary resolver", () => {
  const tmp = resolve(tmpdir(), `codededup-binary-test-${process.pid}`);
  const envDir = resolve(tmp, "env");
  const extDir = resolve(tmp, "ext", "bin", platformId());

  suiteSetup(() => {
    mkdirSync(envDir, { recursive: true });
    mkdirSync(extDir, { recursive: true });
    writeFileSync(resolve(envDir, "codededup-lsp"), "#!/bin/sh\necho 'codededup 9.9.9'\n");
    chmodSync(resolve(envDir, "codededup-lsp"), 0o755);
    writeFileSync(resolve(extDir, "codededup-lsp"), "#!/bin/sh\necho 'codededup 0.1.0'\n");
    chmodSync(resolve(extDir, "codededup-lsp"), 0o755);
  });
  suiteTeardown(() => {
    try { rmSync(tmp, { recursive: true, force: true }); } catch { /* ignore */ }
  });

  test("CODEDEDUP_BINARY_DIR wins", () => {
    const env = { ...process.env, CODEDEDUP_BINARY_DIR: envDir };
    const resolved = resolveBinary(resolve(tmp, "ext"), "lsp", "0.1.0", env);
    assert.equal(resolved.source, "env");
    assert.match(resolved.path, /env/);
  });

  test("falls back to bundled when PATH version mismatches", () => {
    const env = { ...process.env, CODEDEDUP_BINARY_DIR: "", PATH: envDir } as NodeJS.ProcessEnv;
    delete env["CODEDEDUP_BINARY_DIR"];
    const resolved = resolveBinary(resolve(tmp, "ext"), "lsp", "0.1.0", env);
    assert.equal(resolved.source, "bundled");
    assert.match(env["PATH"] ?? "", /ext\/bin/);
  });
});

function platformId(): string {
  if (process.platform === "darwin" && process.arch === "arm64") return "darwin-arm64";
  if (process.platform === "darwin" && process.arch === "x64") return "darwin-x64";
  if (process.platform === "linux" && process.arch === "x64") return "linux-x64";
  if (process.platform === "linux" && process.arch === "arm64") return "linux-arm64";
  if (process.platform === "win32") return "win32-x64";
  throw new Error(`unsupported ${process.platform}-${process.arch}`);
}
