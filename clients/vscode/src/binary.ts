// Binary resolver per [VSIX-BINARY-VERSIONING].
//
// Order of resolution:
//   1. ${DESLOP_BINARY_DIR} — nightly / local builds override everything.
//   2. PATH — reachable via shell (brew tap, scoop bucket, cargo install),
//      accepted only if `<binary> --version` matches the extension version exactly.
//   3. Bundled ${extensionPath}/bin/<platform>/<binary>.
//
// When (3) wins, the resolver prepends that directory to the current VS Code
// process PATH so integrated terminals, task runners, and launch configs see
// `deslop` without the user modifying their shell profile.

import * as fs from "node:fs";
import * as path from "node:path";
import { spawnSync } from "node:child_process";

export type BinaryKind = "lsp" | "mcp" | "cli";

export type Platform =
  | "darwin-arm64"
  | "darwin-x64"
  | "linux-x64"
  | "linux-arm64"
  | "win32-x64";

export interface ResolvedBinary {
  kind: BinaryKind;
  source: "env" | "path" | "bundled";
  path: string;
  version: string | null;
}

export class UnsupportedPlatformError extends Error {
  constructor(platform: string, arch: string) {
    super(`Deslop has no bundled binary for ${platform}-${arch}.`);
    this.name = "UnsupportedPlatformError";
  }
}

export class BundledBinaryMissingError extends Error {
  constructor(public readonly binaryPath: string) {
    super(`Bundled Deslop binary not found at ${binaryPath}. Reinstall the extension.`);
    this.name = "BundledBinaryMissingError";
  }
}

const BINARY_NAMES: Record<BinaryKind, string> = {
  lsp: "deslop-lsp",
  mcp: "deslop-mcp",
  cli: "deslop",
};

export function resolveBinary(
  extensionPath: string,
  kind: BinaryKind,
  expectedVersion: string,
  env: NodeJS.ProcessEnv = process.env,
): ResolvedBinary {
  const platform = currentPlatform();
  const binName = nameWithSuffix(kind, platform);

  const fromEnv = tryEnv(env, binName);
  if (fromEnv) {
    return { kind, source: "env", path: fromEnv, version: versionOf(fromEnv) };
  }

  const fromPath = tryPath(binName, env);
  if (fromPath) {
    const ver = versionOf(fromPath);
    if (ver && ver === expectedVersion) {
      return { kind, source: "path", path: fromPath, version: ver };
    }
  }

  const bundledDir = path.join(extensionPath, "bin", platform);
  const bundledPath = path.join(bundledDir, binName);
  if (!fs.existsSync(bundledPath)) {
    throw new BundledBinaryMissingError(bundledPath);
  }
  prependToPath(env, bundledDir);
  return { kind, source: "bundled", path: bundledPath, version: versionOf(bundledPath) };
}

function tryEnv(env: NodeJS.ProcessEnv, binName: string): string | null {
  const dir = env["DESLOP_BINARY_DIR"];
  if (!dir) return null;
  const candidate = path.join(dir, binName);
  return fs.existsSync(candidate) ? candidate : null;
}

function tryPath(binName: string, env: NodeJS.ProcessEnv): string | null {
  const pathValue = env["PATH"] ?? env["Path"] ?? "";
  const sep = process.platform === "win32" ? ";" : ":";
  for (const dir of pathValue.split(sep)) {
    if (!dir) continue;
    const candidate = path.join(dir, binName);
    if (fs.existsSync(candidate)) return candidate;
  }
  return null;
}

function versionOf(binaryPath: string): string | null {
  try {
    const result = spawnSync(binaryPath, ["--version"], { encoding: "utf8", timeout: 1500 });
    if (result.status !== 0) return null;
    const out = (result.stdout + result.stderr).trim();
    const match = out.match(/\b(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)\b/);
    return match ? (match[1] ?? null) : null;
  } catch {
    return null;
  }
}

function prependToPath(env: NodeJS.ProcessEnv, dir: string): void {
  const key = process.platform === "win32" ? "Path" : "PATH";
  const existing = env[key] ?? "";
  const sep = process.platform === "win32" ? ";" : ":";
  if (existing.split(sep).includes(dir)) return;
  env[key] = existing ? `${dir}${sep}${existing}` : dir;
}

function nameWithSuffix(kind: BinaryKind, platform: Platform): string {
  const suffix = platform.startsWith("win32") ? ".exe" : "";
  return `${BINARY_NAMES[kind]}${suffix}`;
}

function currentPlatform(): Platform {
  const p = process.platform;
  const a = process.arch;
  if (p === "darwin" && a === "arm64") return "darwin-arm64";
  if (p === "darwin" && a === "x64") return "darwin-x64";
  if (p === "linux" && a === "x64") return "linux-x64";
  if (p === "linux" && a === "arm64") return "linux-arm64";
  if (p === "win32" && a === "x64") return "win32-x64";
  throw new UnsupportedPlatformError(p, a);
}
