// Language registry shared by the extension host AND the webview
// bundles — deliberately vscode-free so `webview-ui` can import it.
// Mirrors the core language set (crates/deslop-core/src/lang); the
// [FACET-MODEL] anti-drift rule (#170/#198) requires every language
// `<select>` and grouping surface to derive from this registry instead
// of hand-listing values.

const EXTENSION_LANGUAGE: Record<string, string> = {
  cs: "csharp",
  rs: "rust",
  py: "python",
  dart: "dart",
  js: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  jsx: "javascript",
  ts: "typescript",
  tsx: "tsx",
  fs: "fsharp",
  fsx: "fsharp",
  php: "php",
};

/** Language id for a file path, derived from its extension. Unknown
 * extensions map to `"unknown"`. */
export function languageForPath(filePath: string): string {
  const dot = filePath.lastIndexOf(".");
  if (dot < 0) return "unknown";
  return EXTENSION_LANGUAGE[filePath.slice(dot + 1).toLowerCase()] ?? "unknown";
}

const LANGUAGE_DISPLAY: Record<string, string> = {
  csharp: "C#",
  rust: "Rust",
  python: "Python",
  dart: "Dart",
  javascript: "JavaScript",
  typescript: "TypeScript",
  tsx: "TSX",
  fsharp: "F#",
  php: "PHP",
};

/** Human display name for a language id used in group headings and
 * filter options. */
export function languageDisplayName(language: string): string {
  return LANGUAGE_DISPLAY[language] ?? "Other";
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
  "csharp",
  "rust",
  "python",
  "dart",
  "javascript",
  "javascriptreact",
  "typescript",
  "typescriptreact",
  "fsharp",
  "php",
];
