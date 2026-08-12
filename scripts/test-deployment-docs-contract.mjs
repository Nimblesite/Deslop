// Deployment-documentation contract tests. [DEPLOY-CI-GATES]
//
// Docs that hand a user an absolute path into the installed VSIX are a published
// contract every bit as much as the release workflow is: an MCP client is
// required to launch the bundled binary by absolute path, so a path that does
// not resolve is the same class of defect as a manifest that does not match its
// archive. These tests fail when the documented path drifts from the layout the
// release workflow actually produces.

import { readFileSync, readdirSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { runContractSuite } from "./contract-suite.mjs";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const workflow = readFileSync(resolve(repoRoot, ".github/workflows/release.yml"), "utf8");

// VS Code appends the target triple to the extension directory for any VSIX
// published with `--target`, so the documented directory must carry it too.
const EXTENSION_DIRECTORY = "nimblesite.deslop-live-<VERSION>";
const SKIPPED_DIRECTORIES = new Set(["node_modules", "target", ".git", "dist", "out", "_site", ".deslop-cache"]);
const TEXT_EXTENSIONS = [".md", ".ts", ".mjs", ".json", ".njk", ".yml", ".rs", ".toml"];

const tests = [
  releaseWorkflowPublishesPlatformSpecificVsixArtifacts,
  documentedExtensionDirectoryCarriesItsPlatformTarget,
];

runContractSuite(tests, "deployment docs contract");

// The premise of the second test. If the release workflow ever stops publishing
// per-target VSIXes, VS Code would drop the suffix and the un-suffixed path in
// the docs would become correct — so assert the premise rather than leaving the
// second test asserting something that quietly ceased to be true.
function releaseWorkflowPublishesPlatformSpecificVsixArtifacts() {
  if (!workflow.includes("--target ${{ matrix.vsix_target }}")) {
    throw new Error(
      "release workflow no longer packages per-target VSIXes; the documented extension directory suffix must be revisited",
    );
  }
}

// A platform-specific VSIX unpacks to `<publisher>.<name>-<version>-<target>`,
// so a documented path that goes straight from the version to `/bin` resolves to
// nothing on every platform — which is exactly what shipped.
function documentedExtensionDirectoryCarriesItsPlatformTarget() {
  const offenders = filesUnder(repoRoot).flatMap(unsuffixedReferences);
  if (offenders.length > 0) {
    throw new Error(
      `the installed extension directory is ${EXTENSION_DIRECTORY}-<platform>; these paths omit the target and resolve to nothing:\n  ${offenders.join("\n  ")}`,
    );
  }
}

function unsuffixedReferences(file) {
  const text = readFileSync(file, "utf8");
  const references = [];
  for (let at = text.indexOf(EXTENSION_DIRECTORY); at >= 0; at = text.indexOf(EXTENSION_DIRECTORY, at + 1)) {
    const following = text.slice(at + EXTENSION_DIRECTORY.length);
    if (following.startsWith("/") || following.startsWith("\\")) {
      references.push(`${relative(repoRoot, file)}: …${EXTENSION_DIRECTORY}${following.slice(0, 24)}`);
    }
  }
  return references;
}

function filesUnder(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    if (entry.name.startsWith(".") && entry.name !== ".claude") return [];
    if (SKIPPED_DIRECTORIES.has(entry.name)) return [];
    const child = join(directory, entry.name);
    if (entry.isDirectory()) return filesUnder(child);
    return TEXT_EXTENSIONS.some((extension) => entry.name.endsWith(extension)) ? [child] : [];
  });
}
