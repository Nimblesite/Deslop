// Resolves the bundled LSP / MCP binary for the current platform.
// Per [VSIX-BUNDLE]: darwin-arm64, darwin-x64, linux-x64, linux-arm64, win32-x64.
// No download-on-activate — if the binary is missing, we fail fast.

import * as path from "node:path";
import * as fs from "node:fs";

export type BinaryKind = "lsp" | "mcp";

type Platform = "darwin-arm64" | "darwin-x64" | "linux-x64" | "linux-arm64" | "win32-x64";

export class UnsupportedPlatformError extends Error {
  constructor(platform: string, arch: string) {
    super(`CodeDedup has no bundled binary for ${platform}-${arch}.`);
    this.name = "UnsupportedPlatformError";
  }
}

export class BundledBinaryMissingError extends Error {
  constructor(public readonly binaryPath: string) {
    super(`Bundled CodeDedup binary not found at ${binaryPath}. Reinstall the extension.`);
    this.name = "BundledBinaryMissingError";
  }
}

export function resolveBinary(extensionPath: string, kind: BinaryKind): string {
  const platform = currentPlatform();
  const suffix = platform.startsWith("win32") ? ".exe" : "";
  const binName = kind === "lsp" ? "codededup-lsp" : "codededup-mcp";
  const binaryPath = path.join(extensionPath, "bin", platform, `${binName}${suffix}`);
  if (!fs.existsSync(binaryPath)) {
    throw new BundledBinaryMissingError(binaryPath);
  }
  return binaryPath;
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
