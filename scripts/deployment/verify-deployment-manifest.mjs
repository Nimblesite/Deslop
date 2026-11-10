// [DEPLOY-MANIFEST] Validates shipwright.json before anything is packaged.
// [DEPLOY-MANIFEST-SOURCES] A component's sources are the order a host may
// resolve it in, so they must agree with what actually ships: no forbidden
// `cargo-bin`, a `bundled` block exactly when `bundled` is a source, the bundle
// ahead of `path`, and a bundle for every component an IDE host verifies —
// an IDE host resolves only the bundle ([DEPLOY-RESOLVER]).

import { readFileSync } from "node:fs";
import { isAbsolute, resolve } from "node:path";

/** Sources no component may declare: a source build is pinned to nothing. */
const FORBIDDEN_SOURCES = ["cargo-bin"];
/** Hosts that resolve a component only from the bundle they ship. */
const IDE_HOSTS = ["vscode", "jetbrains"];

const manifestArg = process.argv[2] ?? "shipwright.json";
const manifestPath = isAbsolute(manifestArg) ? manifestArg : resolve(manifestArg);
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));

assertEqual(manifest.manifestVersion, 1, "manifestVersion");
assertString(manifest.product?.id, "product.id");
assertSemver(manifest.product?.version, "product.version");
assertArray(manifest.components, "components");

const componentIds = new Set();
for (const component of manifest.components) {
  verifyComponent(component, componentIds, manifest.product.version);
}
for (const [hostName, host] of Object.entries(manifest.hosts ?? {})) {
  verifyHost(hostName, host, componentIds, manifest.components);
}

console.log(`${manifestPath}: valid deployment manifest`);

function verifyComponent(component, componentIds, productVersion) {
  assertString(component.id, "component.id");
  if (componentIds.has(component.id)) throw new Error(`duplicate component id ${component.id}`);
  componentIds.add(component.id);
  assertString(component.kind, `${component.id}.kind`);
  assertString(component.language, `${component.id}.language`);
  assertSemver(component.expectedVersion, `${component.id}.expectedVersion`);
  if (component.expectedVersion !== productVersion) {
    throw new Error(
      `${component.id}.expectedVersion ${component.expectedVersion} must match product.version ${productVersion}`,
    );
  }
  if (["cli", "lsp", "mcp"].includes(component.kind)) {
    assertString(component.binaryName, `${component.id}.binaryName`);
    assertEqual(component.versionCheckStrategy, "version-flag", `${component.id}.versionCheckStrategy`);
  }
  verifySources(component);
}

function verifySources(component) {
  const sources = component.sources ?? [];
  for (const forbidden of FORBIDDEN_SOURCES) {
    if (sources.includes(forbidden)) throw new Error(`${component.id}.sources must not include ${forbidden}`);
  }
  const listsBundle = sources.includes("bundled");
  if (listsBundle && !component.bundled) {
    throw new Error(`${component.id} lists the bundled source but declares no bundled block`);
  }
  if (component.bundled && !listsBundle) {
    throw new Error(`${component.id} declares a bundled block but no bundled source`);
  }
  if (listsBundle && sources.includes("path") && sources.indexOf("path") < sources.indexOf("bundled")) {
    throw new Error(`${component.id}.sources must put bundled ahead of path`);
  }
}

function verifyHost(hostName, host, componentIds, components) {
  assertArray(host.activationVerifies, `${hostName}.activationVerifies`);
  for (const id of host.activationVerifies) {
    if (!componentIds.has(id)) throw new Error(`${hostName} verifies unknown component ${id}`);
    const component = components.find((candidate) => candidate.id === id);
    if (IDE_HOSTS.includes(hostName) && !component.bundled) {
      throw new Error(`${hostName} verifies ${id}, which declares no bundled binary`);
    }
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
  if (!/^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value)) {
    throw new Error(`${label} must be a semantic version`);
  }
}
