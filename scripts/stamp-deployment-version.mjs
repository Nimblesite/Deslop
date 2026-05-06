// Rewrites `product.version` and every component's `expectedVersion` in a
// Deslop deployment-toolkit.json to a target semver. The release workflow
// invokes this from the version-bump job so the manifest tracks the tag the
// same way Cargo.toml and clients/vscode/package.json do; without it,
// `verify-deployment-binaries.mjs` would correctly fail every release after
// the first because the binary reports the new version while the manifest
// still says the old one (DTK-MIG-DESLOP-CI-GATES, #41).

import { readFileSync, writeFileSync } from "node:fs";
import { isAbsolute, resolve } from "node:path";

const manifestArg = process.argv[2];
const versionArg = process.argv[3];
if (!manifestArg || !versionArg) {
  console.error("usage: stamp-deployment-version.mjs <manifest.json> <version>");
  process.exit(2);
}
if (!isSemver(versionArg)) {
  console.error(`refusing to stamp: '${versionArg}' is not a semantic version`);
  process.exit(1);
}

const manifestPath = isAbsolute(manifestArg) ? manifestArg : resolve(manifestArg);
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
manifest.product.version = versionArg;
for (const component of manifest.components ?? []) {
  if (typeof component.expectedVersion === "string") component.expectedVersion = versionArg;
}
writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`${manifestPath}: stamped product.version and expectedVersion → ${versionArg}`);

function isSemver(value) {
  const parts = value.split(".");
  if (parts.length !== 3) return false;
  return parts.every((part) => part.length > 0 && [...part].every((char) => char >= "0" && char <= "9"));
}
