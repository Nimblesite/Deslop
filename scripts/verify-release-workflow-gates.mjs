// Static lint for the release workflow ([.github/workflows/release.yml]).
// Tied to DTK-MIG-DESLOP-CI-GATES (#41). The CI workflow gates on PR / merge,
// but the release workflow is what actually publishes — if it skips the
// Shipwright verifiers a tag push can ship a manifest-mismatched binary or a
// VSIX missing deployment-toolkit.json. This script asserts the release
// workflow references each Shipwright verifier and routes the VSIX through
// `npm run package` (which runs `verify:package`) rather than calling vsce
// directly. JetBrains gates are out of scope here per the active migration
// split.

import { existsSync, readFileSync } from "node:fs";
import { isAbsolute, resolve } from "node:path";

const arg = process.argv[2] ?? ".github/workflows/release.yml";
const workflowPath = isAbsolute(arg) ? arg : resolve(arg);
if (!existsSync(workflowPath)) throw new Error(`Missing release workflow at ${workflowPath}`);
const workflow = readFileSync(workflowPath, "utf8");

const requiredReferences = [
  {
    needle: "scripts/verify-deployment-manifest.mjs",
    label: "manifest validator",
    rationale: "Release must fail if deployment-toolkit.json is invalid",
  },
  {
    needle: "scripts/verify-deployment-binaries.mjs",
    label: "binary version contract verifier",
    rationale: "Release must fail if a built binary reports a version or component id that differs from the manifest",
  },
  {
    needle: "verify-vsix-package.mjs",
    label: "VSIX package verifier",
    rationale: "Release must fail if the VSIX omits deployment-toolkit.json or a manifest-listed binary, or includes an undeclared binary",
  },
];

for (const { needle, label, rationale } of requiredReferences) {
  if (!workflow.includes(needle)) {
    throw new Error(`${workflowPath} is missing the ${label} (${needle}). ${rationale}.`);
  }
}

if (workflow.includes("npx vsce package")) {
  throw new Error(
    `${workflowPath} calls 'npx vsce package' directly. Route VSIX packaging through 'npm run package' so verify:package runs against the produced artifact.`,
  );
}

console.log(`${workflowPath}: release workflow gates wired (manifest + binaries + VSIX)`);
