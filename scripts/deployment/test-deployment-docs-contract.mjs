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

const repoRoot = fileURLToPath(new URL("../..", import.meta.url));
const workflow = readFileSync(resolve(repoRoot, ".github/workflows/release.yml"), "utf8");

// VS Code appends the target triple to the extension directory for any VSIX
// published with `--target`, so the documented directory must carry it too.
const EXTENSION_DIRECTORY = "nimblesite.deslop-live-<VERSION>";
const SKIPPED_DIRECTORIES = new Set(["node_modules", "target", ".git", "dist", "out", "_site", ".deslop-cache"]);
const TEXT_EXTENSIONS = [".md", ".ts", ".mjs", ".json", ".njk", ".yml", ".rs", ".toml"];

const tests = [
  releaseWorkflowPublishesPlatformSpecificVsixArtifacts,
  documentedExtensionDirectoryCarriesItsPlatformTarget,
  installerSnippetContractRunsWhereverTheSnippetCanChange,
];

let failed = 0;
for (const test of tests) {
  try {
    test();
    console.log(`ok ${test.name}`);
  } catch (error) {
    failed++;
    console.error(`not ok ${test.name}`);
    console.error(`  ${error instanceof Error ? error.message : String(error)}`);
  }
}

if (failed > 0) {
  console.error(`\n${failed} deployment docs contract test(s) failed`);
  process.exit(1);
}
console.log(`\n${tests.length} deployment docs contract tests passed`);

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

// [DEPLOY-DOCS-INSTALLER-FAILCLOSED] routing contract. The fail-closed
// installer snippet lives on the published site pages, and CI classifies a
// PR touching only those pages as `site=true, code=false` — skipping the
// `ci` job where `make lint` runs the snippet's contract test. The security
// test must therefore also be a step of the `site` job, or a site-only PR
// can regress the exact snippet the test was written to protect. The
// routing half asserts the classifier and the site job's gate still wire an
// installer-page-only change to that step: `site/**` changes set
// `site=true`, and the site job runs exactly when they do.
function installerSnippetContractRunsWhereverTheSnippetCanChange() {
  const ciWorkflow = readFileSync(resolve(repoRoot, ".github/workflows/ci.yml"), "utf8");
  const makefile = readFileSync(resolve(repoRoot, "Makefile"), "utf8");
  const runner = "node --test scripts/deployment/installer-snippet.test.mjs";
  if (!makefile.includes(runner)) {
    throw new Error("make lint no longer runs the installer snippet contract; code PRs would stop covering it");
  }
  requireInOrder(ciWorkflow, ["grep -qE '^site/'", 'echo "site=true"'],
    "the classifier no longer maps site/** changes to site=true; an installer-page-only PR would skip the site job entirely");
  const siteJob = ciWorkflow.indexOf("\n  site:");
  const securityJob = ciWorkflow.indexOf("\n  security:");
  if (siteJob < 0 || securityJob < 0) {
    throw new Error("ci.yml no longer declares the site/security jobs this contract anchors on; update the contract");
  }
  const siteJobBody = ciWorkflow.slice(siteJob, securityJob);
  if (!siteJobBody.includes("if: needs.changes.outputs.site == 'true'")) {
    throw new Error("the site job no longer gates on the classifier's site output; the routing this contract proves has changed");
  }
  if (!siteJobBody.includes(runner)) {
    throw new Error(
      "the site job does not run the installer snippet contract; a site-only PR could change the published installer without the fail-closed test running",
    );
  }
}

// Asserts each needle appears, and after the previous one — the shape of the
// classifier's "match then emit" bash without parsing it.
function requireInOrder(text, needles, message) {
  let at = 0;
  for (const needle of needles) {
    at = text.indexOf(needle, at);
    if (at < 0) throw new Error(`${message} (missing: ${needle})`);
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
