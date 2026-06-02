// Packaging acceptance gate for the embedding "stub" provider.
//
// The deterministic BLAKE3 stub (`provider_id` "stub", `model_id` "blake3-stub",
// type `StubProvider`) is test-only embedding infrastructure gated behind the
// deslop-core `test-support` Cargo feature. It must never reach a shipped VSIX:
// not in the settings enum, not in any bundled asset. Spec: fusion.md
// [FUSION-EMBED-PROVIDER] and vsix.md [VSIX-EMBED-PICKER].
//
// This module holds the pure detection logic so it is unit-testable without a
// real `.vsix`. `verify-vsix-package.mjs` injects an unzip-backed reader;
// `stub-gate.test.mjs` injects an in-memory reader. Detection lives here once.

/** VSIX entry holding the extension manifest. */
export const PACKAGE_ENTRY = "extension/package.json";

/**
 * Forbidden strings. The `stub` provider id is only matched when quoted so a
 * stray comment word (or a minifier symbol) cannot trip the gate; the two
 * identifiers are distinctive enough to match bare.
 */
export const STUB_TOKENS = [/blake3-stub/, /StubProvider/, /["']stub["']/];

/** Shipped text assets worth scanning; binaries and source maps are skipped. */
export const STUB_SCAN_SUFFIXES = [".js", ".json", ".md"];

/** Returns the first forbidden token present in `content`, or `null`. */
export function findStubToken(content) {
  return STUB_TOKENS.find((token) => token.test(content)) ?? null;
}

/** Flattens `contributes.configuration` (object or array form) to properties. */
function configurationProperties(packageJson) {
  const configuration = packageJson?.contributes?.configuration ?? {};
  const blocks = Array.isArray(configuration) ? configuration : [configuration];
  return Object.assign({}, ...blocks.map((block) => block.properties ?? {}));
}

/** Returns the first setting key that offers `stub` as a value, or `null`. */
export function findStubSettingKey(packageJson) {
  for (const [key, schema] of Object.entries(configurationProperties(packageJson))) {
    const values = [...(schema.enum ?? []), schema.default].filter((value) => typeof value === "string");
    if (values.includes("stub")) return key;
  }
  return null;
}

/** True when `entry` is a shipped text asset the gate should scan. */
export function isStubScanEntry(entry) {
  if (entry === PACKAGE_ENTRY) return true;
  return entry.startsWith("extension/dist/") && STUB_SCAN_SUFFIXES.some((suffix) => entry.endsWith(suffix));
}

/**
 * Throws if any scanned asset re-exposes the stub provider. `readText(entry)`
 * returns the text content of a VSIX entry; `label` names the source for error
 * messages. Returns the list of scanned entries on success.
 */
export function assertNoStubProvider({ entries, readText, label = "package" }) {
  const settingKey = findStubSettingKey(JSON.parse(readText(PACKAGE_ENTRY)));
  if (settingKey) {
    throw new Error(`${PACKAGE_ENTRY} setting ${settingKey} offers the stub provider; production settings must exclude it`);
  }
  const scanned = entries.filter(isStubScanEntry);
  for (const entry of scanned) {
    const hit = findStubToken(readText(entry));
    if (hit) {
      throw new Error(`${entry} in ${label} exposes stub provider string ${hit}; the BLAKE3 stub is test-only and must not ship`);
    }
  }
  return scanned;
}
