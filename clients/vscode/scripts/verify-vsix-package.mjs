import { mkdtempSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const vsixRoot = resolve(here, "..");
const vsixArg = process.argv[2] ?? "deslop-live.vsix";
const vsixPath = isAbsolute(vsixArg) ? vsixArg : resolve(vsixRoot, vsixArg);
const targetPlatform = process.argv[3] ?? currentPlatform();
const packageEntry = "extension/package.json";
const manifestEntry = "extension/shipwright.json";
// [REMOVE-STUB] Test-only stub identifiers that must never ship in the VSIX
// (packaging acceptance gate; see assertNoStubProvider below).
const stubTokens = [/blake3-stub/, /StubProvider/, /["']stub["']/];
const stubScanSuffixes = [".js", ".json", ".md"];

const entries = unzipText(["-Z1", vsixPath]).split("\n").filter(Boolean);
assertPackageIdentity(entries);
assertEntry(entries, manifestEntry);
assertNoEntryPrefix(entries, "extension/out/");
assertNoEntryPrefix(entries, "extension/node_modules/");
assertNoEntryPrefix(entries, "extension/--stdio/");

const manifest = JSON.parse(unzipText(["-p", vsixPath, manifestEntry]));
const components = executableComponents(manifest);
const binRoot = "extension/bin/";
const binPrefix = `${binRoot}${targetPlatform}/`;
const allBinEntries = entries.filter((entry) => entry.startsWith(binRoot) && !entry.endsWith("/"));
const binEntries = entries.filter((entry) => entry.startsWith(binPrefix) && !entry.endsWith("/"));
const foreignBinEntries = allBinEntries.filter((entry) => !entry.startsWith(binPrefix));

if (foreignBinEntries.length > 0) {
  throw new Error(
    `Platform-specific VSIX for ${targetPlatform} must contain only ${binPrefix} binaries; found ${foreignBinEntries.join(", ")}`,
  );
}

for (const component of components) {
  assertEntry(entries, `${binPrefix}${nameWithSuffix(component)}`);
}
for (const entry of binEntries) {
  verifyBundledEntry(entry, componentForEntry(entry, components));
}

assertNoStubProvider(entries);

console.log(`Verified deployment manifest and ${binEntries.length} ${targetPlatform} VSIX binaries`);

function assertPackageIdentity(entries) {
  assertEntry(entries, packageEntry);
  const packageJson = JSON.parse(unzipText(["-p", vsixPath, packageEntry]));
  if (packageJson.publisher !== "nimblesite" || packageJson.name !== "deslop-live") {
    throw new Error(
      `${vsixPath} extension id must be nimblesite.deslop-live; found ${packageJson.publisher}.${packageJson.name}`,
    );
  }
}

function verifyBundledEntry(entry, component) {
  if (!component) throw new Error(`Undeclared executable in VSIX: ${entry}`);
  const temp = mkdtempSync(join(tmpdir(), "deslop-vsix-"));
  try {
    unzipText(["-q", vsixPath, entry, "-d", temp]);
    const binaryPath = join(temp, entry);
    assertExecutable(binaryPath);
    assertVersion(binaryPath, component);
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

function componentForEntry(entry, components) {
  const fileName = entry.slice(entry.lastIndexOf("/") + 1);
  return components.find((component) => nameWithSuffix(component) === fileName);
}

function assertVersion(binaryPath, component) {
  if (targetPlatform !== currentPlatform()) return;
  // macOS security scanning of freshly compiled binaries can take ~500 ms under load;
  // 10 s is generous enough to survive a heavy parallel build without false failures.
  const result = spawnSync(binaryPath, ["--version"], { encoding: "utf8", timeout: 10_000 });
  if (result.status !== 0 || result.signal != null) {
    const detail = result.signal != null
      ? `killed by signal ${result.signal}`
      : `exit ${result.status}`;
    throw new Error(`${binaryPath} --version failed (${detail})\nstderr: ${result.stderr}`);
  }
  const first = firstLine(String(result.stdout));
  const expected = `${component.id} ${component.expectedVersion}`;
  if (first !== expected) throw new Error(`${binaryPath} reported ${first}; expected ${expected}`);
}

function assertExecutable(binaryPath) {
  if (targetPlatform.startsWith("win32")) return;
  if ((statSync(binaryPath).mode & 0o111) === 0) {
    throw new Error(`${binaryPath} is not executable`);
  }
}

function executableComponents(manifest) {
  return (manifest.components ?? []).filter((component) =>
    ["cli", "lsp", "mcp"].includes(component.kind),
  );
}

function assertEntry(entries, entry) {
  if (!entries.includes(entry)) throw new Error(`Missing ${entry} in ${vsixPath}`);
}

function assertNoEntryPrefix(entries, prefix) {
  const matches = entries.filter((entry) => entry.startsWith(prefix));
  if (matches.length > 0) throw new Error(`${vsixPath} must not include ${prefix}`);
}

// [REMOVE-STUB] The deterministic BLAKE3 stub is test-only embedding
// infrastructure. This is the packaging acceptance gate from
// docs/plans/remove-stub-provider-from-production-vsix.md: fail packaging when
// any shipped asset re-exposes the `stub` provider id, its `blake3-stub` model
// id, or the `StubProvider` type. Source maps carry the original comments but
// are excluded by .vscodeignore, so they are never VSIX entries to scan.
function assertNoStubProvider(entries) {
  assertSettingsExcludeStub();
  const scanned = entries.filter(isStubScanEntry);
  for (const entry of scanned) assertEntryHasNoStubToken(entry);
  console.log(`Verified ${scanned.length} packaged assets carry no stub provider strings`);
}

function isStubScanEntry(entry) {
  if (entry === packageEntry) return true;
  return entry.startsWith("extension/dist/") && stubScanSuffixes.some((suffix) => entry.endsWith(suffix));
}

function assertEntryHasNoStubToken(entry) {
  const content = unzipText(["-p", vsixPath, entry]);
  const hit = stubTokens.find((token) => token.test(content));
  if (hit) {
    throw new Error(`${entry} in ${vsixPath} exposes stub provider string ${hit}; the BLAKE3 stub is test-only and must not ship`);
  }
}

function assertSettingsExcludeStub() {
  const packageJson = JSON.parse(unzipText(["-p", vsixPath, packageEntry]));
  for (const [key, schema] of Object.entries(configurationProperties(packageJson))) {
    const values = [...(schema.enum ?? []), schema.default].filter((value) => typeof value === "string");
    if (values.includes("stub")) {
      throw new Error(`${packageEntry} setting ${key} offers the stub provider; production settings must exclude it`);
    }
  }
}

function configurationProperties(packageJson) {
  const configuration = packageJson?.contributes?.configuration ?? {};
  const blocks = Array.isArray(configuration) ? configuration : [configuration];
  return Object.assign({}, ...blocks.map((block) => block.properties ?? {}));
}

function unzipText(args) {
  const result = spawnSync("unzip", args, { encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(`unzip ${args.join(" ")} failed: ${String(result.stderr)}`);
  }
  return String(result.stdout);
}

function nameWithSuffix(component) {
  return `${component.binaryName}${targetPlatform.startsWith("win32") ? ".exe" : ""}`;
}

function firstLine(text) {
  const end = text.indexOf("\n");
  const line = end >= 0 ? text.slice(0, end) : text;
  return line.endsWith("\r") ? line.slice(0, -1) : line;
}

function currentPlatform() {
  if (process.platform === "darwin" && process.arch === "arm64") return "darwin-arm64";
  if (process.platform === "darwin" && process.arch === "x64") return "darwin-x64";
  if (process.platform === "linux" && process.arch === "x64") return "linux-x64";
  if (process.platform === "linux" && process.arch === "arm64") return "linux-arm64";
  if (process.platform === "win32" && process.arch === "x64") return "win32-x64";
  throw new Error(`unsupported platform ${process.platform}-${process.arch}`);
}
