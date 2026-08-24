// Static lint for the release workflow ([.github/workflows/release.yml]).
// Implements the release-workflow side of [DEPLOY-VSIX-PACKAGE].
// Tied to DTK-MIG-DESLOP-CI-GATES (#41). The CI workflow gates on PR / merge,
// but the release workflow is what actually publishes — if it skips the
// Shipwright verifiers a tag push can ship a manifest-mismatched binary or a
// VSIX missing shipwright.json. This script asserts the release
// workflow references each Shipwright verifier and packages platform-specific
// VSIX artifacts with the VS Code `--target` contract. JetBrains gates are out
// of scope here per the active migration split.

import { existsSync, readFileSync } from "node:fs";
import { isAbsolute, resolve } from "node:path";

const arg = process.argv[2] ?? ".github/workflows/release.yml";
const workflowPath = isAbsolute(arg) ? arg : resolve(arg);
if (!existsSync(workflowPath)) throw new Error(`Missing release workflow at ${workflowPath}`);
const workflow = readFileSync(workflowPath, "utf8");

const requiredReferences = [
  {
    needle: "scripts/deployment/verify-deployment-manifest.mjs",
    label: "manifest validator",
    rationale: "Release must fail if shipwright.json is invalid",
  },
  {
    needle: "scripts/release/stamp-release-version.mjs",
    label: "build-time release version stamper",
    rationale: "Release must stamp the tag version without committing source changes",
  },
  {
    needle: "scripts/deployment/verify-deployment-binaries.mjs",
    label: "binary version contract verifier",
    rationale: "Release must fail if a built binary reports a version or component id that differs from the manifest",
  },
  {
    needle: "verify-vsix-package.mjs",
    label: "VSIX package verifier",
    rationale: "Release must fail if the VSIX omits shipwright.json or a manifest-listed binary, or includes an undeclared binary",
  },
];

for (const { needle, label, rationale } of requiredReferences) {
  if (!workflow.includes(needle)) {
    throw new Error(`${workflowPath} is missing the ${label} (${needle}). ${rationale}.`);
  }
}

if (!workflow.includes('node-version: "22.x"')) {
  throw new Error(
    `${workflowPath} must use Node 22.x for the VSIX release path, matching the VS Code platform-specific sample.`,
  );
}

for (const target of ["linux-x64", "linux-arm64", "darwin-x64", "darwin-arm64", "win32-x64"]) {
  if (!workflow.includes(`vsix_target: ${target}`)) {
    throw new Error(`${workflowPath} is missing VSIX target ${target}.`);
  }
}

if (!workflow.includes("--target ${{ matrix.vsix_target }}")) {
  throw new Error(`${workflowPath} must package VSIX artifacts with --target \${{ matrix.vsix_target }}.`);
}

if (!workflow.includes("-${{ matrix.vsix_target }}.vsix")) {
  throw new Error(`${workflowPath} must suffix VSIX filenames with the VS Code target.`);
}

if (/npx\s+vsce\s+package(?![^\n]*--target)/.test(workflow)) {
  throw new Error(`${workflowPath} calls 'npx vsce package' without --target.`);
}

console.log(`${workflowPath}: release workflow gates wired (manifest + binaries + VSIX)`);
