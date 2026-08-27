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
