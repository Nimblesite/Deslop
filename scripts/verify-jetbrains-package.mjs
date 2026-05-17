import { existsSync, mkdtempSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, isAbsolute, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const packageArg = process.argv[2] ?? latestPackage();
const packagePath = isAbsolute(packageArg) ? packageArg : resolve(packageArg);
const platform = process.argv[3] ?? currentPlatform();
const entries = unzipText(["-Z1", packagePath]).split("\n").filter(Boolean);
const root = packageRoot(entries);
const manifestEntry = `${root}/shipwright.json`;

assertEntry(entries, manifestEntry);

const manifest = JSON.parse(unzipText(["-p", packagePath, manifestEntry]));
const component = componentById(manifest, "deslop-lsp");
const lspEntry = `${root}/bin/${platform}/${nameWithSuffix(component)}`;
assertEntry(entries, lspEntry);
verifyBundledEntry(lspEntry, component);

for (const entry of binEntries(entries, root)) {
  if (!componentForEntry(entry, manifest)) throw new Error(`Undeclared JetBrains binary: ${entry}`);
}

console.log(`Verified JetBrains package ${packagePath} for ${platform}`);

function verifyBundledEntry(entry, component) {
  const temp = mkdtempSync(join(tmpdir(), "deslop-jetbrains-"));
  try {
    unzipText(["-q", packagePath, entry, "-d", temp]);
    const binaryPath = join(temp, entry);
    assertExecutable(binaryPath);
    assertVersion(binaryPath, component);
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

function assertVersion(binaryPath, component) {
  const result = spawnSync(binaryPath, ["--version"], { encoding: "utf8", timeout: 1500 });
  if (result.status !== 0) throw new Error(`${binaryPath} --version failed`);
  const expected = `${component.id} ${component.expectedVersion}`;
  const actual = firstLine(String(result.stdout));
  if (actual !== expected) throw new Error(`${binaryPath} reported ${actual}; expected ${expected}`);
}

function latestPackage() {
  const dir = resolve("clients/jetbrains/build/distributions");
  const zips = existsSync(dir) ? readdirSync(dir).filter((name) => name.endsWith(".zip")) : [];
  if (zips.length === 0) throw new Error(`No JetBrains package zip found under ${dir}`);
  return join(dir, zips.sort().at(-1));
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
  return (manifest.components ?? []).find((component) => nameWithSuffix(component) === fileName);
}

function binEntries(entries, root) {
  const prefix = `${root}/bin/${platform}/`;
  return entries.filter((entry) => entry.startsWith(prefix) && !entry.endsWith("/"));
}

function assertExecutable(binaryPath) {
  if (platform.startsWith("win32")) return;
  if ((statSync(binaryPath).mode & 0o111) === 0) throw new Error(`${binaryPath} is not executable`);
}

function assertEntry(entries, entry) {
  if (!entries.includes(entry)) throw new Error(`Missing ${entry} in ${packagePath}`);
}

function unzipText(args) {
  const result = spawnSync("unzip", args, { encoding: "utf8" });
  if (result.status !== 0) throw new Error(`unzip failed: ${String(result.stderr)}`);
  return String(result.stdout);
}

function nameWithSuffix(component) {
  return `${component.binaryName}${platform.startsWith("win32") ? ".exe" : ""}`;
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
