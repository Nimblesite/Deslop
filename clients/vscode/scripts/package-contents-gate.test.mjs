// Proves the VSIX contents allow-list (package-contents-gate.mjs) rejects an
// archive carrying anything outside the extension, and passes a clean one.
// Issue #472: `extension/test-results/.last-run.json` shipped to users because
// the old gate was a deny-list of three prefixes and nothing denied a
// directory Playwright had only just started writing. Spec: vsix.md
// [DEPLOY-VSIX-PACKAGE]. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  ALLOWED_EXTENSION_DIRECTORIES,
  ALLOWED_EXTENSION_FILES,
  VSIX_METADATA_ENTRIES,
  assertDeclaredEntriesPresent,
  assertOnlyExpectedEntries,
  declaredEntries,
  isExpectedEntry,
  unexpectedEntries,
} from "./package-contents-gate.mjs";

/** The entry Playwright leaked into the shipped 0.15.0 darwin-arm64 VSIX. */
const PLAYWRIGHT_LEAK = "extension/test-results/.last-run.json";

/** A Playwright trace — what a *failing* run writes beside that file. */
const PLAYWRIGHT_TRACE = "extension/test-results/webview-smoke-renders/trace.zip";

/** Entries the old three-prefix deny-list named by hand. */
const FORMERLY_DENIED = [
  "extension/out/extension.js",
  "extension/node_modules/vscode/package.json",
  "extension/--stdio/deslop.sock",
];

/** Manifest fields that name a shipped asset, as the real manifest spells them. */
const MANIFEST_MAIN = "./dist/extension.js";
const MANIFEST_ICON = "media/logo.png";
const MAIN_ENTRY = "extension/dist/extension.js";
const ICON_ENTRY = "extension/media/logo.png";

const packageJson = { main: MANIFEST_MAIN, icon: MANIFEST_ICON };

/**
 * `unzip -Z1` of the built `deslop-live-darwin-arm64.vsix`, verbatim and in
 * archive order — including the leak. The gate has to be provable without a
 * 14 MB binary in the tree, so the real listing is the fixture.
 */
const SHIPPED_VSIX_ENTRIES = [
  "extension.vsixmanifest",
  "[Content_Types].xml",
  "extension/shipwright.json",
  "extension/package.json",
  "extension/readme.md",
  "extension/LICENSE.txt",
  "extension/media/logo.png",
  "extension/media/activity-bar.svg",
  "extension/dist/schema_doc.md",
  "extension/dist/extension.js",
  PLAYWRIGHT_LEAK,
  "extension/media/webview/report.js",
  "extension/media/webview/duplication.js",
  "extension/media/webview/cluster.js",
  "extension/bin/darwin-arm64/deslop-mcp",
  "extension/bin/darwin-arm64/deslop-lsp",
  "extension/bin/darwin-arm64/deslop",
];

const cleanEntries = SHIPPED_VSIX_ENTRIES.filter((entry) => entry !== PLAYWRIGHT_LEAK);

test("[#472] the gate rejects the VSIX that actually shipped, naming the leaked entry", () => {
  assert.deepEqual(unexpectedEntries(SHIPPED_VSIX_ENTRIES), [PLAYWRIGHT_LEAK]);
  assert.throws(
    () => assertOnlyExpectedEntries({ entries: SHIPPED_VSIX_ENTRIES, label: "deslop-live-darwin-arm64.vsix" }),
    (error) =>
      error.message.includes(PLAYWRIGHT_LEAK) &&
      error.message.includes("deslop-live-darwin-arm64.vsix") &&
      error.message.includes(".vscodeignore"),
    "the failure must name the offending entry, the archive, and the fix",
  );
});

test("[#472] the same VSIX passes once the leaked directory is gone", () => {
  assert.deepEqual(unexpectedEntries(cleanEntries), []);
  assert.deepEqual(assertOnlyExpectedEntries({ entries: cleanEntries }), cleanEntries);
  assert.equal(cleanEntries.length, SHIPPED_VSIX_ENTRIES.length - 1, "exactly one entry separates leaked from clean");
});

test("[#472] a failing Playwright run's trace is rejected too, not just the run marker", () => {
  assert.equal(isExpectedEntry(PLAYWRIGHT_TRACE), false);
  assert.throws(
    () => assertOnlyExpectedEntries({ entries: [...cleanEntries, PLAYWRIGHT_TRACE] }),
    new RegExp(PLAYWRIGHT_TRACE.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
  );
});

test("[#472] an allow-list keeps rejecting everything the old deny-list named", () => {
  for (const denied of FORMERLY_DENIED) {
    assert.equal(isExpectedEntry(denied), false, `${denied} must stay out of the package`);
    assert.throws(() => assertOnlyExpectedEntries({ entries: [...cleanEntries, denied] }));
  }
});

test("[#472] a brand-new sibling directory fails closed without anyone updating a list", () => {
  const invented = "extension/.turbo-cache/manifest.json";
  assert.equal(isExpectedEntry(invented), false);
  assert.throws(() => assertOnlyExpectedEntries({ entries: [...cleanEntries, invented] }), /outside the extension/);
});

test("the shipping surface accepts every metadata entry, named file, and directory it declares", () => {
  for (const entry of [...VSIX_METADATA_ENTRIES, ...ALLOWED_EXTENSION_FILES]) {
    assert.ok(isExpectedEntry(entry), `${entry} is part of the declared shipping surface`);
  }
  for (const directory of ALLOWED_EXTENSION_DIRECTORIES) {
    assert.ok(isExpectedEntry(`${directory}nested/asset.bin`), `${directory} contents must ship`);
    assert.ok(isExpectedEntry(directory), `${directory} directory record must be accepted`);
  }
  assert.ok(isExpectedEntry("extension/"), "the extension root directory record must be accepted");
});

test("a file at the archive root outside vsce's metadata is rejected", () => {
  assert.equal(isExpectedEntry("extension.js"), false);
  assert.equal(isExpectedEntry("shipwright.json"), false, "the manifest ships inside extension/, not at the root");
});

test("declaredEntries resolves the manifest's own asset paths into archive entries", () => {
  assert.deepEqual(declaredEntries(packageJson), [MAIN_ENTRY, ICON_ENTRY]);
  assert.deepEqual(declaredEntries({}), [], "a manifest declaring no assets contributes no entries");
});

test("assertDeclaredEntriesPresent passes the real listing and catches an over-eager ignore rule", () => {
  assert.deepEqual(assertDeclaredEntriesPresent({ entries: cleanEntries, packageJson }), [MAIN_ENTRY, ICON_ENTRY]);
  assert.throws(
    () =>
      assertDeclaredEntriesPresent({
        entries: cleanEntries.filter((entry) => entry !== ICON_ENTRY),
        packageJson,
        label: "deslop-live-darwin-arm64.vsix",
      }),
    (error) => error.message.includes(ICON_ENTRY) && error.message.includes("missing manifest-declared"),
    "dropping the declared icon must fail the package",
  );
});

// The assertions below read the real packaging configuration rather than an
// in-memory listing. The allow-list above is the backstop; these pin the two
// rules that stop the leak reaching it in the first place, so a future edit
// that deletes either one fails here with the reason attached.

import { readFileSync } from "node:fs";
import { dirname, isAbsolute, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const extensionRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

/** The repository's build-artifact directory: `clients/vscode` sits two below it. */
const repoTarget = resolve(extensionRoot, "..", "..", "target");

/**
 * Whether `child` lies strictly inside `parent`.
 *
 * A path relationship, resolved as one. Spelling it as a `"/target/"`
 * substring both passed any unrelated directory that happened to be named
 * `target` and failed on every Windows host, where the same directory is
 * spelled with backslashes.
 *
 * @param {string} parent absolute directory
 * @param {string} child absolute path to test
 * @returns {boolean} true when `child` is under `parent` and not `parent` itself
 */
function isInside(parent, child) {
  const step = relative(parent, child);
  return step.length > 0 && !step.startsWith("..") && !isAbsolute(step);
}

/** Ignore rules that must survive, and the artifact each one keeps out. */
const REQUIRED_IGNORE_RULES = [
  ["test-results/**", "Playwright traces, screenshots, videos and .last-run.json"],
  ["playwright.config.mjs", "the Playwright config itself"],
];

test("[#472] .vscodeignore keeps Playwright's output and config out of the package", () => {
  const rules = readFileSync(resolve(extensionRoot, ".vscodeignore"), "utf8")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith("#"));
  for (const [rule, keptOut] of REQUIRED_IGNORE_RULES) {
    assert.ok(rules.includes(rule), `.vscodeignore must keep ${rule} — it is what excludes ${keptOut}`);
  }
});

test("[#472] Playwright writes its output under target/, never beside the extension", async () => {
  const config = (await import("../playwright.config.mjs")).default;
  assert.equal(typeof config.outputDir, "string", "playwright.config.mjs must set an explicit outputDir");
  assert.ok(
    isAbsolute(config.outputDir),
    `Playwright outputDir ${config.outputDir} must be absolute; a relative one lands wherever the run was started from`,
  );
  assert.ok(
    !isInside(extensionRoot, config.outputDir) && config.outputDir !== extensionRoot,
    `Playwright outputDir ${config.outputDir} is inside the packaging root ${extensionRoot}; a failing run would ship its traces`,
  );
  assert.ok(
    isInside(repoTarget, config.outputDir),
    `every build artifact belongs under this repository's ${repoTarget}; got ${config.outputDir}`,
  );
});
