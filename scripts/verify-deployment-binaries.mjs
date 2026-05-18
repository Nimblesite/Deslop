import { existsSync, readFileSync, statSync } from "node:fs";
import { isAbsolute, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const manifestPath = resolveArg(process.argv[2] ?? "shipwright.json");
const binDir = resolveArg(process.argv[3] ?? "target/release");
const platform = process.argv[4] ?? currentPlatform();
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));

for (const component of executableComponents(manifest)) verifyComponent(component);

console.log(`Verified deployment binaries in ${binDir} for ${platform}`);

function verifyComponent(component) {
  const binaryPath = join(binDir, nameWithSuffix(component));
  if (!existsSync(binaryPath)) throw new Error(`Missing ${component.id} at ${binaryPath}`);
  assertExecutable(binaryPath);
  assertPlainVersion(binaryPath, component);
  assertJsonVersion(binaryPath, component);
}

function assertPlainVersion(binaryPath, component) {
  const first = firstLine(run(binaryPath, ["--version"]));
  const expected = `${component.id} ${component.expectedVersion}`;
  if (first !== expected) throw new Error(`${binaryPath} reported ${first}; expected ${expected}`);
}

function assertJsonVersion(binaryPath, component) {
  const value = JSON.parse(run(binaryPath, ["--version", "--json"]));
  assertEqual(value.manifestVersion, 1, `${component.id}.manifestVersion`);
  assertEqual(value.name, component.id, `${component.id}.name`);
  assertEqual(value.version, component.expectedVersion, `${component.id}.version`);
  assertEqual(value.kind, component.kind, `${component.id}.kind`);
  assertEqual(value.language, component.language, `${component.id}.language`);
  assertEqual(value.product, manifest.product.id, `${component.id}.product`);
}

function run(binaryPath, args) {
  const result = spawnSync(binaryPath, args, { encoding: "utf8", timeout: 1500 });
  if (result.status !== 0) throw new Error(`${binaryPath} ${args.join(" ")} failed`);
  return String(result.stdout);
}

function executableComponents(manifest) {
  return (manifest.components ?? [])
    .filter((component) => ["cli", "lsp", "mcp"].includes(component.kind))
    .filter((component) => component.required !== false);
}

function assertExecutable(binaryPath) {
  if (platform.startsWith("win32")) return;
  if ((statSync(binaryPath).mode & 0o111) === 0) throw new Error(`${binaryPath} is not executable`);
}

function firstLine(text) {
  const end = text.indexOf("\n");
  const line = end >= 0 ? text.slice(0, end) : text;
  return line.endsWith("\r") ? line.slice(0, -1) : line;
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) throw new Error(`${label} must be ${expected}; got ${actual}`);
}

function nameWithSuffix(component) {
  return `${component.binaryName}${platform.startsWith("win32") ? ".exe" : ""}`;
}

function resolveArg(value) {
  return isAbsolute(value) ? value : resolve(value);
}

function currentPlatform() {
  if (process.platform === "darwin" && process.arch === "arm64") return "darwin-arm64";
  if (process.platform === "darwin" && process.arch === "x64") return "darwin-x64";
  if (process.platform === "linux" && process.arch === "x64") return "linux-x64";
  if (process.platform === "linux" && process.arch === "arm64") return "linux-arm64";
  if (process.platform === "win32" && process.arch === "x64") return "win32-x64";
  throw new Error(`unsupported platform ${process.platform}-${process.arch}`);
}
