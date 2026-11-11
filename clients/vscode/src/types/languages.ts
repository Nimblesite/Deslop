// Language registry shared by the extension host AND the webview
// bundles — deliberately vscode-free so `webview-ui` can import it.
// Mirrors the core language set (crates/deslop-core/src/lang); the
// [FACET-MODEL] anti-drift rule (#170/#198) requires every language
// `<select>` and grouping surface to derive from this registry instead
// of hand-listing values.

import registry from "./languageRegistry.json";

const JAVASCRIPT_LANGUAGE_ID = "javascript";
const FSHARP_LANGUAGE_ID = "fsharp";

/** File extension → language id, shared with the website's language panel. */
const EXTENSION_LANGUAGE: Record<string, string> = registry.extensions;

/** Every source extension the registry resolves, without the leading dot,
 * in registry order. Derived from the one extension map so activation
 * events and file filters can be checked against it rather than
 * re-listing extensions per surface ([FACET-MODEL], #170/#198). */
export const SOURCE_EXTENSIONS: readonly string[] = Object.keys(EXTENSION_LANGUAGE);

/** Language id for a file path, derived from its extension. Unknown
 * extensions map to `"unknown"`. */
export function languageForPath(filePath: string, selected?: string | null): string {
  if (selected) return languageFamily(selected);
  const dot = filePath.lastIndexOf(".");
  if (dot < 0) return "unknown";
  return EXTENSION_LANGUAGE[filePath.slice(dot + 1).toLowerCase()] ?? "unknown";
}

/** Flavor-specific parser identities share Osprey's UI family. */
export function languageFamily(language: string): string {
  return ["osprey", "osprey-ml", "osprey-python"].includes(language) ? "osprey" : language;
}

const LANGUAGE_DISPLAY: Record<string, string> = registry.display;

/** Human display name for a language id used in group headings and
 * filter options. */
export function languageDisplayName(language: string): string {
  return LANGUAGE_DISPLAY[languageFamily(language)] ?? "Other";
}

/** Every registered language id in registry order, deduplicated (several
 * extensions share one id). Drives filter `<select>` options so a newly
 * registered language appears everywhere with zero per-surface edits. */
export const LANGUAGES: readonly string[] = [...new Set(Object.values(EXTENSION_LANGUAGE))];

/** VS Code *editor* language IDs that Deslop attaches its additive surfaces
 * to — the hover clone-card, the inlay bubble, and the LSP document sync.
 * These are editor IDs, not Deslop language ids: `.jsx`/`.tsx` surface as
 * `javascriptreact`/`typescriptreact`, and F# as `fsharp`. Single source of
 * truth so a newly supported language lights up every editor surface at once
 * instead of drifting per call site ([FACET-MODEL] anti-drift, #170/#198).
 * Mirrors the core parser registry
 * (crates/deslop-core/src/pipeline/corpus.rs::default_parsers). */
export const ANALYSED_LANGUAGE_IDS: readonly string[] = [
  "osprey",
  "osprey-ml",
  "csharp",
  "rust",
  "python",
  "dart",
  JAVASCRIPT_LANGUAGE_ID,
  "javascriptreact",
  "typescript",
  "typescriptreact",
  FSHARP_LANGUAGE_ID,
  "php",
  "go",
  "c",
];
