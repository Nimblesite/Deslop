import { existsSync, mkdtempSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, isAbsolute, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

import { openArchive } from "../lib/zip.mjs";
import { currentPlatform, executableName } from "../release/vsix-platforms.mjs";

// [DEPLOY-JETBRAINS-PACKAGE] CI gate for the JetBrains plugin package contract.
// Verifies the shipped JetBrains plugin zip. With no path argument it checks the
// single :deslop-lsp4ij artifact (one LSP4IJ build serves Android Studio, IntelliJ
// Community, and Rider/Ultimate), which bundles shipwright.json at its root plus a
// bin/<platform>/deslop-lsp staged from the same manifest contract.
// Every binary here was just extracted into a fresh temp dir, so this is always
// a FIRST exec: macOS validates the unsigned ~30 MB file before it runs
// (Gatekeeper / `syspolicyd`), which costs hundreds of milliseconds. A tight
// budget makes packaging fail on machine load rather than on a real defect
// ([DEPLOY-RESOLVER]).
const FIRST_EXEC_TIMEOUT_MS = 30_000;

const platform = process.argv[3] ?? currentPlatform();
const explicit = process.argv[2];
const packagePaths = explicit
  ? [isAbsolute(explicit) ? explicit : resolve(explicit)]
  : defaultPackages();

for (const packagePath of packagePaths) verifyPackage(packagePath);

function verifyPackage(packagePath) {
  const archive = openArchive(packagePath);
  const entries = archive.names;
  const root = packageRoot(entries);
  const manifestEntry = `${root}/shipwright.json`;
  assertEntry(entries, manifestEntry, packagePath);

  const manifest = JSON.parse(archive.readText(manifestEntry));
  const component = componentById(manifest, "deslop-lsp");
  const lspEntry = `${root}/bin/${platform}/${executableName(component.binaryName, platform)}`;
  assertEntry(entries, lspEntry, packagePath);
  verifyBundledEntry(archive, lspEntry, component);

  for (const entry of binEntries(entries, root)) {
    if (!componentForEntry(entry, manifest)) throw new Error(`Undeclared JetBrains binary: ${entry}`);
  }
  verifyFlatClasspath(entries, root, packagePath);
  console.log(`Verified JetBrains package ${packagePath} for ${platform}`);
}

// [DEPLOY-JETBRAINS-PACKAGE] Fast structural gate for the classloader contract. The tool
// window and Tools action are declared in the main plugin.xml, so their classes must load
// from the main plugin classloader — a top-level lib/*.jar. A jar under lib/modules/ is a
// content module behind a child classloader the parent cannot see, so those extensions
// silently vanish. This flat plugin declares no <content>, so it must ship the shared UI
// jar directly under lib/ and nothing under lib/modules/. (Real IDE-level registration is
// covered by the deslop-lsp4ij integration test; this keeps the regression cheap in CI.)
function verifyFlatClasspath(entries, root, packagePath) {
  const contentModuleJars = entries.filter(
    (entry) => entry.startsWith(`${root}/lib/modules/`) && entry.endsWith(".jar"),
  );
  if (contentModuleJars.length > 0) {
    throw new Error(
      `${packagePath} bundles ${contentModuleJars.join(", ")} under lib/modules/; this flat plugin ` +
        `declares no <content>, so those classes never load and the Deslop tool window + Tools action ` +
        `silently vanish. Bundle shared code in a top-level lib/*.jar.`,
    );
  }
  const sharedUiJar = entries.find(
    (entry) => entry.startsWith(`${root}/lib/`) && /shared[^/]*\.jar$/.test(entry),
  );
  if (!sharedUiJar) {
    throw new Error(`${packagePath} is missing the shared UI jar (deslop-*shared*.jar) directly under ${root}/lib/`);
  }
}

function defaultPackages() {
  return ["deslop-lsp4ij"].map((module) => {
    const dir = resolve(`clients/jetbrains/${module}/build/distributions`);
    const zips = existsSync(dir) ? readdirSync(dir).filter((name) => name.endsWith(".zip")) : [];
    if (zips.length === 0) throw new Error(`No JetBrains package zip found under ${dir}`);
    return join(dir, zips.sort().at(-1));
  });
}

function verifyBundledEntry(archive, entry, component) {
  const temp = mkdtempSync(join(tmpdir(), "deslop-jetbrains-"));
  try {
    const binaryPath = archive.extract(entry, temp);
    assertExecutable(binaryPath);
    assertVersion(binaryPath, component);
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

function assertVersion(binaryPath, component) {
  const result = spawnSync(binaryPath, ["--version"], {
    encoding: "utf8",
    timeout: FIRST_EXEC_TIMEOUT_MS,
  });
  if (result.status !== 0) throw new Error(`${binaryPath} --version failed`);
  const expected = `${component.id} ${component.expectedVersion}`;
  const actual = firstLine(String(result.stdout));
  if (actual !== expected) throw new Error(`${binaryPath} reported ${actual}; expected ${expected}`);
}

function packageRoot(entries) {
  const root = entries[0]?.split("/")[0];
  if (!root) throw new Error("JetBrains package is empty");
  return root;
}

function componentById(manifest, id) {
  const component = (manifest.components ?? []).find((candidate) => candidate.id === id);
  if (!component) throw new Error(`Manifest is missing ${id}`);
  return component;
}

function componentForEntry(entry, manifest) {
  const fileName = basename(entry);
  return (manifest.components ?? []).find(
    (component) => executableName(component.binaryName, platform) === fileName,
  );
}

function binEntries(entries, root) {
  const prefix = `${root}/bin/${platform}/`;
  return entries.filter((entry) => entry.startsWith(prefix) && !entry.endsWith("/"));
}

function assertExecutable(binaryPath) {
  if (platform.startsWith("win32")) return;
  if ((statSync(binaryPath).mode & 0o111) === 0) throw new Error(`${binaryPath} is not executable`);
}

function assertEntry(entries, entry, packagePath) {
  if (!entries.includes(entry)) throw new Error(`Missing ${entry} in ${packagePath}`);
}

function firstLine(text) {
  const end = text.indexOf("\n");
  const line = end >= 0 ? text.slice(0, end) : text;
  return line.endsWith("\r") ? line.slice(0, -1) : line;
}

