// Manifest-backed binary resolver for the VS Code host.
// Contract source:
// https://github.com/Nimblesite/Shipwright/blob/main/docs/specs/ide-extension-deployment.md#required-startup-behavior

import * as fs from "node:fs";
import * as path from "node:path";
import { spawnSync } from "node:child_process";

export type BinaryKind = "lsp" | "mcp" | "cli";

export type BinarySource = "user-setting" | "env-path" | "env-dir" | "bundled";

export type Platform =
  | "darwin-arm64"
  | "darwin-x64"
  | "linux-x64"
  | "linux-arm64"
  | "win32-x64";

export interface ResolvedBinary {
  kind: BinaryKind;
  componentId: string;
  source: BinarySource;
  path: string;
  version: string;
}

export interface BinarySettings {
  lspPath?: string;
  mcpPath?: string;
}

export interface DeploymentManifest {
  manifestVersion: number;
  product: { id: string; version: string };
  components: DeploymentComponent[];
  hosts: Record<string, HostContract | undefined>;
}

export interface DeploymentComponent {
  id: string;
  kind: string;
  language: string;
  binaryName: string;
  expectedVersion: string;
  bundled?: { bundlePath: string };
  userSetting?: string;
  env?: { pathVar?: string; dirVar?: string };
  required?: boolean;
}

export interface HostContract {
  activationVerifies: string[];
}

interface Candidate {
  source: BinarySource;
  path: string;
  hardFailure: boolean;
}

interface VersionProbe {
  name: string | null;
  version: string | null;
  raw: string;
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

export class BinaryVerificationError extends Error {
  constructor(component: DeploymentComponent, candidate: Candidate, found: string) {
    super(mismatchMessage(component, candidate, found));
    this.name = "BinaryVerificationError";
  }
}

export class BinaryMissingError extends Error {
  constructor(component: DeploymentComponent, source: BinarySource, binaryPath: string) {
    super(
      `Deslop cannot start: ${component.id} ${component.expectedVersion} was not found at ${binaryPath} from ${source}.`,
    );
    this.name = "BinaryMissingError";
  }
}

// [DEPLOY-MANIFEST] shipwright.json is the package authority for required
// components, expected versions, and host startup checks.
export function loadDeploymentManifest(extensionPath: string): DeploymentManifest {
  const manifestPath = deploymentManifestPath(extensionPath);
  const raw = fs.readFileSync(manifestPath, "utf8");
  return JSON.parse(raw) as DeploymentManifest;
}

export function resolveHostBinaries(
  extensionPath: string,
  host: string,
  manifest: DeploymentManifest,
  settings: BinarySettings = {},
  env: NodeJS.ProcessEnv = process.env,
): Record<string, ResolvedBinary> {
  const hostContract = manifest.hosts[host];
  if (!hostContract) throw new Error(`deployment manifest has no ${host} host contract`);
  return Object.fromEntries(
    hostContract.activationVerifies.map((id) => [
      id,
      resolveComponent(extensionPath, requireComponent(manifest, id), settings, env),
    ]),
  );
}

// [DEPLOY-RESOLVER] Every candidate (setting/env/bundled) must still prove
// the manifest's component id and version before use; no source is a bypass.
export function resolveBinary(
  extensionPath: string,
  kind: BinaryKind,
  manifest: DeploymentManifest,
  settings: BinarySettings = {},
  env: NodeJS.ProcessEnv = process.env,
): ResolvedBinary {
  const component = manifest.components.find((candidate) => candidate.kind === kind);
  if (!component) throw new Error(`deployment manifest has no ${kind} component`);
  return resolveComponent(extensionPath, component, settings, env);
}

function resolveComponent(
  extensionPath: string,
  component: DeploymentComponent,
  settings: BinarySettings,
  env: NodeJS.ProcessEnv,
): ResolvedBinary {
  let skippedPath: Candidate | undefined;
  for (const candidate of candidates(extensionPath, component, settings, env)) {
    const resolved = verifyCandidate(component, candidate);
    if (resolved) return resolved;
    skippedPath = candidate;
  }
  throwMissing(component, skippedPath);
}

function verifyCandidate(
  component: DeploymentComponent,
  candidate: Candidate,
): ResolvedBinary | undefined {
  if (!fs.existsSync(candidate.path)) return handleMissing(candidate, component);
  const probe = versionProbe(candidate.path);
  if (probe.name === component.id && probe.version === component.expectedVersion) {
    return resolvedBinary(component, candidate, probe.version);
  }
  if (!candidate.hardFailure) return undefined;
  throw new BinaryVerificationError(component, candidate, probeVersion(probe));
}

function handleMissing(candidate: Candidate, component?: DeploymentComponent): undefined {
  if (candidate.source === "bundled") throw new BundledBinaryMissingError(candidate.path);
  if (candidate.hardFailure && component) {
    throw new BinaryMissingError(component, candidate.source, candidate.path);
  }
  return undefined;
}

// [DEPLOY-RESOLVER] Source order: user-setting first, then the
// bundled binary (env-path/env-dir are off for VS Code's manifest).
function candidates(
  extensionPath: string,
  component: DeploymentComponent,
  settings: BinarySettings,
  env: NodeJS.ProcessEnv,
): Candidate[] {
  return [
    ...settingCandidate(component, settings),
    ...envPathCandidate(component, env),
    ...envDirCandidate(component, env),
    bundledCandidate(extensionPath, component),
  ].filter((candidate): candidate is Candidate => Boolean(candidate));
}

function settingCandidate(
  component: DeploymentComponent,
  settings: BinarySettings,
): Candidate[] {
  const configured = settingValue(component, settings);
  return configured ? [{ source: "user-setting", path: configured, hardFailure: true }] : [];
}

function envPathCandidate(component: DeploymentComponent, env: NodeJS.ProcessEnv): Candidate[] {
  const pathVar = component.env?.pathVar;
  const configured = pathVar ? nonBlank(env[pathVar]) : undefined;
  return configured ? [{ source: "env-path", path: configured, hardFailure: true }] : [];
}

function envDirCandidate(component: DeploymentComponent, env: NodeJS.ProcessEnv): Candidate[] {
  const dirVar = component.env?.dirVar;
  const configured = dirVar ? nonBlank(env[dirVar]) : undefined;
  return configured ? [candidateFromDir(component, configured, "env-dir", true)] : [];
}

function bundledCandidate(
  extensionPath: string,
  component: DeploymentComponent,
): Candidate | undefined {
  if (!component.bundled) return undefined;
  const bundlePath = component.bundled.bundlePath;
  return {
    source: "bundled",
    path: path.join(extensionPath, interpolateBundlePath(bundlePath, component)),
    hardFailure: true,
  };
}

function candidateFromDir(
  component: DeploymentComponent,
  dir: string,
  source: BinarySource,
  hardFailure: boolean,
): Candidate {
  return { source, path: path.join(dir, nameWithSuffix(component)), hardFailure };
}

function settingValue(component: DeploymentComponent, settings: BinarySettings): string | undefined {
  if (component.kind === "lsp") return nonBlank(settings.lspPath);
  if (component.kind === "mcp") return nonBlank(settings.mcpPath);
  return undefined;
}

/** Budget for a binary that has already been executed once: a warm
 * `--version` answers in single-digit milliseconds. */
const PROBE_TIMEOUT_MS = 1500;

/** Budget for the FIRST execution of a freshly written binary. macOS
 * validates an unsigned ~30 MB binary on its first exec (Gatekeeper /
 * `syspolicyd`), which costs hundreds of milliseconds and, on a loaded
 * machine, more than the warm budget — and a just-installed VSIX is in
 * exactly that state for both bundled binaries. */
const PROBE_FIRST_EXEC_TIMEOUT_MS = 30_000;

// [DEPLOY-RESOLVER] A probe that never answered is INCONCLUSIVE, not a
// mismatch. Reporting it as one turned a slow first exec into "version
// mismatch" with no commands registered — a dead extension until reload.
// Retry once on the wider budget before believing the binary is wrong.
function versionProbe(binaryPath: string): VersionProbe {
  return (
    probeOnce(binaryPath, PROBE_TIMEOUT_MS) ??
    probeOnce(binaryPath, PROBE_FIRST_EXEC_TIMEOUT_MS) ?? {
      name: null,
      version: null,
      raw: `no reply to --version within ${PROBE_FIRST_EXEC_TIMEOUT_MS}ms`,
    }
  );
}

/** True when `spawnSync` gave up on the child rather than failing to launch
 * it — the one outcome worth a second, wider attempt. */
function timedOut(error: Error | undefined): boolean {
  return error !== undefined && "code" in error && error.code === "ETIMEDOUT";
}

/** One `--version` exec. `undefined` means the child never replied inside
 * `timeout`, so the caller may retry rather than treat it as a verdict. */
function probeOnce(binaryPath: string, timeout: number): VersionProbe | undefined {
  try {
    const result = spawnSync(binaryPath, ["--version"], { encoding: "utf8", timeout });
    if (timedOut(result.error)) return undefined;
    if (result.error) return { name: null, version: null, raw: result.error.message };
    if (result.status !== 0) return { name: null, version: null, raw: String(result.stderr) };
    return parseVersionLine(firstLine(String(result.stdout)));
  } catch (err) {
    return { name: null, version: null, raw: err instanceof Error ? err.message : String(err) };
  }
}

function parseVersionLine(line: string): VersionProbe {
  const parts = line.trim().split(" ");
  if (parts.length !== 2) return { name: null, version: null, raw: line };
  return { name: parts[0] ?? null, version: parts[1] ?? null, raw: line };
}

function firstLine(text: string): string {
  const end = text.indexOf("\n");
  const line = end >= 0 ? text.slice(0, end) : text;
  return line.endsWith("\r") ? line.slice(0, -1) : line;
}

function interpolateBundlePath(template: string, component: DeploymentComponent): string {
  return template
    .replace("${platform}", currentPlatform())
    .replace("${binaryName}", component.binaryName)
    .replace("${exe}", currentPlatform().startsWith(WINDOWS_PLATFORM) ? ".exe" : "");
}

function resolvedBinary(
  component: DeploymentComponent,
  candidate: Candidate,
  version: string | null,
): ResolvedBinary {
  return {
    kind: toBinaryKind(component.kind),
    componentId: component.id,
    source: candidate.source,
    path: candidate.path,
    version: version ?? "",
  };
}

function requireComponent(manifest: DeploymentManifest, id: string): DeploymentComponent {
  const component = manifest.components.find((candidate) => candidate.id === id);
  if (!component) throw new Error(`deployment manifest has no component ${id}`);
  return component;
}

function deploymentManifestPath(extensionPath: string): string {
  const packagedPath = path.join(extensionPath, "shipwright.json");
  if (fs.existsSync(packagedPath)) return packagedPath;
  return path.resolve(extensionPath, "..", "..", "shipwright.json");
}

function throwMissing(component: DeploymentComponent, skipped?: Candidate): never {
  const suffix = skipped ? ` Last checked: ${skipped.path} from ${skipped.source}.` : "";
  throw new Error(`No matching ${component.id} ${component.expectedVersion} binary found.${suffix}`);
}

function probeVersion(probe: VersionProbe): string {
  return probe.name && probe.version ? `${probe.name} ${probe.version}` : probe.raw || "not found";
}

function mismatchMessage(
  component: DeploymentComponent,
  candidate: Candidate,
  found: string,
): string {
  return [
    `Deslop cannot start: ${component.id} version mismatch.`,
    `Expected ${component.expectedVersion} from shipwright.json.`,
    `Found ${found} at ${candidate.path} from ${candidate.source}.`,
    "Use a matching binary or clear the configured override.",
  ].join(" ");
}

function nameWithSuffix(component: DeploymentComponent): string {
  const suffix = currentPlatform().startsWith(WINDOWS_PLATFORM) ? ".exe" : "";
  return `${component.binaryName}${suffix}`;
}

function toBinaryKind(kind: string): BinaryKind {
  if (kind === "cli" || kind === "lsp" || kind === "mcp") return kind;
  throw new Error(`component kind ${kind} is not executable`);
}

function nonBlank(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  if (!trimmed) return undefined;
  return trimmed;
}

const X64_ARCHITECTURE = "x64";
const WINDOWS_PLATFORM = "win32";

/** [DEPLOY-MANIFEST] Pure platform resolution so every host arm — and the
 * unsupported-platform refusal — is unit-testable without stubbing the
 * running process. */
export function currentPlatformFor(platform: string, arch: string): Platform {
  if (platform === "darwin" && arch === "arm64") return "darwin-arm64";
  if (platform === "darwin" && arch === X64_ARCHITECTURE) return "darwin-x64";
  if (platform === "linux" && arch === X64_ARCHITECTURE) return "linux-x64";
  if (platform === "linux" && arch === "arm64") return "linux-arm64";
  if (platform === WINDOWS_PLATFORM && arch === X64_ARCHITECTURE) return "win32-x64";
  throw new UnsupportedPlatformError(platform, arch);
}

function currentPlatform(): Platform {
  return currentPlatformFor(process.platform, process.arch);
}
