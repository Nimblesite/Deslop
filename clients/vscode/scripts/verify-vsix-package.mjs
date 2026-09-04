import { mkdtempSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { assertNoStubProvider, PACKAGE_ENTRY } from "./stub-gate.mjs";
import { assertDeclaredEntriesPresent, assertOnlyExpectedEntries } from "./package-contents-gate.mjs";
import { currentPlatformTarget } from "./platform.mjs";
import { openArchive } from "../../../scripts/lib/zip.mjs";
import { executableName } from "../../../scripts/release/vsix-platforms.mjs";

// Verifies [DEPLOY-VSIX-PACKAGE] against the produced .vsix, not the
// staging directory, so release artifacts cannot hide manifest or binary drift.
const here = dirname(fileURLToPath(import.meta.url));
const vsixRoot = resolve(here, "..");
const vsixArg = process.argv[2] ?? "deslop-live.vsix";
const vsixPath = isAbsolute(vsixArg) ? vsixArg : resolve(vsixRoot, vsixArg);
const targetPlatform = process.argv[3] ?? currentPlatformTarget();
const packageEntry = PACKAGE_ENTRY;
const manifestEntry = "extension/shipwright.json";

const archive = openArchive(vsixPath);
const entries = archive.names;
const packageJson = readPackageJson(entries);
assertPackageIdentity(packageJson);
assertEntry(entries, manifestEntry);
// [DEPLOY-VSIX-PACKAGE] Allow-list, never a deny-list: a deny-list cannot name
// a directory that did not exist when it was written, which is how Playwright's
// `test-results/` shipped to users (#472). Anything new fails closed here.
assertOnlyExpectedEntries({ entries, label: vsixPath });
const declaredAssets = assertDeclaredEntriesPresent({ entries, packageJson, label: vsixPath });

const manifest = JSON.parse(archive.readText(manifestEntry));
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
  assertEntry(entries, `${binPrefix}${executableName(component.binaryName, targetPlatform)}`);
}
for (const entry of binEntries) {
  verifyBundledEntry(entry, componentForEntry(entry, components));
}

const stubScanned = assertNoStubProvider({
  entries,
  readText: (entry) => archive.readText(entry),
  label: vsixPath,
});
console.log(`Verified ${stubScanned.length} packaged assets carry no stub provider strings`);

console.log(
  `Verified ${entries.length} packaged entries against the shipping allow-list, `
    + `including ${declaredAssets.length} manifest-declared assets`,
);
console.log(`Verified deployment manifest and ${binEntries.length} ${targetPlatform} VSIX binaries`);

function readPackageJson(entries) {
  assertEntry(entries, packageEntry);
  return JSON.parse(archive.readText(packageEntry));
}

function assertPackageIdentity(packageJson) {
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
    const binaryPath = archive.extract(entry, temp);
    assertExecutable(binaryPath);
    assertVersion(binaryPath, component);
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

function componentForEntry(entry, components) {
  const fileName = entry.slice(entry.lastIndexOf("/") + 1);
  return components.find(
    (component) => executableName(component.binaryName, targetPlatform) === fileName,
  );
}

function assertVersion(binaryPath, component) {
  if (targetPlatform !== currentPlatformTarget()) return;
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

function firstLine(text) {
  const end = text.indexOf("\n");
  const line = end >= 0 ? text.slice(0, end) : text;
  return line.endsWith("\r") ? line.slice(0, -1) : line;
}
