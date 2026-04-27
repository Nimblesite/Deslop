import { readFileSync } from "node:fs";
import { isAbsolute, resolve } from "node:path";

const manifestArg = process.argv[2] ?? "deployment-toolkit.json";
const manifestPath = isAbsolute(manifestArg) ? manifestArg : resolve(manifestArg);
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));

assertEqual(manifest.manifestVersion, 1, "manifestVersion");
assertString(manifest.product?.id, "product.id");
assertSemver(manifest.product?.version, "product.version");
assertArray(manifest.components, "components");

const componentIds = new Set();
for (const component of manifest.components) verifyComponent(component, componentIds);
for (const [hostName, host] of Object.entries(manifest.hosts ?? {})) {
  verifyHost(hostName, host, componentIds);
}

console.log(`${manifestPath}: valid deployment manifest`);

function verifyComponent(component, componentIds) {
  assertString(component.id, "component.id");
  if (componentIds.has(component.id)) throw new Error(`duplicate component id ${component.id}`);
  componentIds.add(component.id);
  assertString(component.kind, `${component.id}.kind`);
  assertString(component.language, `${component.id}.language`);
  assertSemver(component.expectedVersion, `${component.id}.expectedVersion`);
  if (["cli", "lsp", "mcp"].includes(component.kind)) {
    assertString(component.binaryName, `${component.id}.binaryName`);
    assertEqual(component.versionCheckStrategy, "version-flag", `${component.id}.versionCheckStrategy`);
  }
}

function verifyHost(hostName, host, componentIds) {
  assertArray(host.activationVerifies, `${hostName}.activationVerifies`);
  for (const id of host.activationVerifies) {
    if (!componentIds.has(id)) throw new Error(`${hostName} verifies unknown component ${id}`);
  }
}

function assertString(value, label) {
  if (typeof value !== "string" || value.length === 0) throw new Error(`${label} must be a string`);
}

function assertArray(value, label) {
  if (!Array.isArray(value)) throw new Error(`${label} must be an array`);
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) throw new Error(`${label} must be ${expected}; got ${actual}`);
}

function assertSemver(value, label) {
  assertString(value, label);
  const parts = value.split(".");
  if (parts.length !== 3 || parts.some((part) => !allDigits(part))) {
    throw new Error(`${label} must be a semantic version`);
  }
}

function allDigits(value) {
  return value.length > 0 && [...value].every((char) => char >= "0" && char <= "9");
}
