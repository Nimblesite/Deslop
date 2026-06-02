import { mkdtempSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { assertNoStubProvider, PACKAGE_ENTRY } from "./stub-gate.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const vsixRoot = resolve(here, "..");
const vsixArg = process.argv[2] ?? "deslop-live.vsix";
const vsixPath = isAbsolute(vsixArg) ? vsixArg : resolve(vsixRoot, vsixArg);
const targetPlatform = process.argv[3] ?? currentPlatform();
const packageEntry = PACKAGE_ENTRY;
const manifestEntry = "extension/shipwright.json";

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

const stubScanned = assertNoStubProvider({
  entries,
  readText: (entry) => unzipText(["-p", vsixPath, entry]),
  label: vsixPath,
});
console.log(`Verified ${stubScanned.length} packaged assets carry no stub provider strings`);

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
