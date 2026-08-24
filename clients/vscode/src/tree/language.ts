// Per-language split for the Top Offenders tree
// ([VSIX-TOP-OFFENDERS-LANGUAGE-GROUP]). The language id is the
// engine's: it comes from the parser registry that actually parsed the
// file ([PIPELINE-LANG-TRAIT]) and rides on the cluster, so the tree
// cannot group a file into a language the analysis never used.

import { ReportCluster } from "../types/report";

// The registry itself lives in the vscode-free `types/languages` module
// so the webview bundles can share it; re-exported here so existing
// tree-side import sites keep resolving.
export { languageDisplayName, languageForPath } from "../types/languages";

/** The cluster's language id as the engine stamped it. An empty value
 * — a report written before the field existed — reads as the engine's
 * own unresolvable label rather than being re-derived from a path. */
export function clusterLanguage(cluster: ReportCluster): string {
  return cluster.language || "unknown";
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
    const language = clusterLanguage(cluster);
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
