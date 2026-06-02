// Per-language split for the Top Offenders tree
// ([VSIX-TOP-OFFENDERS-LANGUAGE-GROUP]). `languageForPath` mirrors the
// core `language_for_path()` in crates/deslop-core/src/render/html.rs so
// the VSIX and the HTML report agree on language ids.

import { ReportCluster } from "../types/report";
import { representativePath } from "./paths";

const EXTENSION_LANGUAGE: Record<string, string> = {
  cs: "csharp",
  rs: "rust",
  py: "python",
  dart: "dart",
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
};

/** Human display name for a language id used in group headings. */
export function languageDisplayName(language: string): string {
  return LANGUAGE_DISPLAY[language] ?? "Other";
}

/** Reads the persisted split-by-language toggle. Defaults off; unknown
 * values are treated as off — never throws. */
export function normalizeSplitByLanguage(raw: boolean | undefined): boolean {
  return raw === true;
}

/** Buckets clusters by their canonical occurrence language, preserving
 * the input worst-first order within each bucket and first-seen order
 * across buckets. Given a globally worst-first input, first-seen order
 * is also worst-weight order across languages
 * ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]). */
export function groupByLanguage(
  clusters: ReportCluster[],
): Array<{ language: string; clusters: ReportCluster[] }> {
  const order: string[] = [];
  const buckets = new Map<string, ReportCluster[]>();
  for (const cluster of clusters) {
    const language = languageForPath(representativePath(cluster));
    let bucket = buckets.get(language);
    if (!bucket) {
      bucket = [];
      buckets.set(language, bucket);
      order.push(language);
    }
    bucket.push(cluster);
  }
  return order.map((language) => ({ language, clusters: buckets.get(language) ?? [] }));
}
