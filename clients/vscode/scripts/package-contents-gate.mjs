// Packaging acceptance gate for the VSIX file list. Spec: vsix.md
// [DEPLOY-VSIX-PACKAGE].
//
// This gate used to be a deny-list of three prefixes, and a deny-list cannot
// name a directory that did not exist when the list was written: Playwright
// started writing `test-results/` beside the extension and its contents
// shipped to users (issue #472). The list is an allow-list instead — the
// archive may carry vsce's metadata, four named files, and three shipping
// directories, and nothing else. A new directory now fails the package
// instead of riding along inside it.
//
// The pure logic lives here so it is unit-testable without a real `.vsix`;
// `verify-vsix-package.mjs` feeds it a real archive listing and
// `package-contents-gate.test.mjs` feeds it in-memory listings. Detection
// lives here once. Everything is exact string work — never a pattern match.

/** Prefix every packaged extension file carries inside the archive. */
export const EXTENSION_ROOT = "extension/";

/** vsce's own archive metadata, written outside `extension/`. */
export const VSIX_METADATA_ENTRIES = ["extension.vsixmanifest", "[Content_Types].xml"];

/** Files that ship at the root of `extension/`, and why each one ships. */
export const ALLOWED_EXTENSION_FILES = [
  "extension/package.json", // extension manifest VS Code reads on activation
  "extension/shipwright.json", // deployment manifest [DEPLOY-VERSION-CONTRACT]
  "extension/readme.md", // Marketplace listing page
  "extension/LICENSE.txt", // license vsce copies from the repository root
];

/** Directories whose whole contents ship, and why each one ships. */
export const ALLOWED_EXTENSION_DIRECTORIES = [
  "extension/dist/", // esbuild bundle plus the generated schema doc
  "extension/media/", // icon, activity-bar glyph, webview assets
  "extension/bin/", // platform binaries, verified against the manifest
];

/**
 * True for a bare directory record on the allowed tree. vsce writes file
 * records only, but a zip that also carries directory records must not fail
 * for naming a directory whose contents are already allowed.
 */
function isAllowedDirectoryRecord(entry) {
  if (!entry.endsWith("/")) return false;
  if (entry === EXTENSION_ROOT) return true;
  return ALLOWED_EXTENSION_DIRECTORIES.some((directory) => directory.startsWith(entry));
}

/** True when `entry` belongs to the extension's declared shipping surface. */
export function isExpectedEntry(entry) {
  if (VSIX_METADATA_ENTRIES.includes(entry)) return true;
  if (ALLOWED_EXTENSION_FILES.includes(entry)) return true;
  if (ALLOWED_EXTENSION_DIRECTORIES.some((directory) => entry.startsWith(directory))) return true;
  return isAllowedDirectoryRecord(entry);
}

/** Every archive entry the packaging contract does not account for. */
export function unexpectedEntries(entries) {
  return entries.filter((entry) => !isExpectedEntry(entry));
}

/** Human-readable summary of everything a VSIX is allowed to carry. */
export function shippingSurface() {
  return [...VSIX_METADATA_ENTRIES, ...ALLOWED_EXTENSION_FILES, ...ALLOWED_EXTENSION_DIRECTORIES].join(", ");
}

/**
 * Throws when the archive carries anything outside the shipping surface.
 * `label` names the source for error messages. Returns the checked entries.
 */
export function assertOnlyExpectedEntries({ entries, label = "package" }) {
  const unexpected = unexpectedEntries(entries);
  if (unexpected.length > 0) {
    throw new Error(
      `${label} ships ${unexpected.length} entry/entries from outside the extension: ${unexpected.join(", ")}. ` +
        `A VSIX may contain only ${shippingSurface()}. Add a .vscodeignore rule, ` +
        `or extend ALLOWED_EXTENSION_FILES / ALLOWED_EXTENSION_DIRECTORIES when the file is genuinely meant to ship.`,
    );
  }
  return entries;
}

/** Turns a manifest-relative path into its archive entry name. */
function archiveEntry(relativePath) {
  const trimmed = relativePath.startsWith("./") ? relativePath.slice(2) : relativePath;
  return `${EXTENSION_ROOT}${trimmed}`;
}

/** Archive entries the extension manifest itself declares (`main`, `icon`). */
export function declaredEntries(packageJson) {
  return [packageJson?.main, packageJson?.icon]
    .filter((path) => typeof path === "string" && path.length > 0)
    .map(archiveEntry);
}

/**
 * Throws when the archive is missing an asset the manifest declares — an
 * over-eager ignore rule breaks the extension just as surely as a stray
 * artifact bloats it. Returns the declared entries on success.
 */
export function assertDeclaredEntriesPresent({ entries, packageJson, label = "package" }) {
  const declared = declaredEntries(packageJson);
  const missing = declared.filter((entry) => !entries.includes(entry));
  if (missing.length > 0) {
    throw new Error(`${label} is missing manifest-declared asset(s): ${missing.join(", ")}`);
  }
  return declared;
}
