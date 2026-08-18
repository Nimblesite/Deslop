// Shared factories for tree-provider unit tests.
// Non-`.test.ts` so the Mocha glob does not load this as a suite.

import * as vscode from "vscode";
import { Bucket, FileMetric, RepoMetrics, Report, ReportCluster } from "../../types/report";
import { emptyReport, metrics as zeroMetrics } from "./report-store.helpers";
import { bucketSignals } from "../signals.helpers";

/** Re-exported so a tree suite needs one helper module, not two. */
export { bucketSignals };

export function cluster(
  id: string,
  weight: number,
  occurrencePath: string,
  startByte = 0,
  endByte = 20,
  bucket: Bucket = "identical",
  category?: string,
): ReportCluster {
  return {
    id,
    weight,
    size: 2,
    canonical_node_count: 4,
    signals: bucketSignals(bucket),
    bucket,
    ...(category === undefined ? {} : { category }),
    occurrences: [
      { path: occurrencePath, start_byte: startByte, end_byte: endByte, hidden: false },
      {
        path: `${occurrencePath}.other`,
        start_byte: startByte,
        end_byte: endByte,
        hidden: false,
      },
    ],
    occurrences_total: 0,
    occurrences_truncated: false,
    summary: "",
    interpretation: `dup in ${occurrencePath}`,
  };
}

export function labelText(item: vscode.TreeItem): string {
  return typeof item.label === "string" ? item.label : item.label?.label ?? "";
}

export function iconColorId(item: vscode.TreeItem): string {
  const icon = item.iconPath as vscode.ThemeIcon | undefined;
  const color = icon?.color;
  return String(color?.id ?? "");
}

export function tooltipText(item: vscode.TreeItem): string {
  if (item.tooltip instanceof vscode.MarkdownString) return item.tooltip.value;
  return String(item.tooltip ?? "");
}

/** Builds a `FileMetric` with the percentage derived from the counts. */
export function fileMetric(path: string, analysedLoc: number, duplicatedLoc: number): FileMetric {
  return {
    path,
    analysed_loc: analysedLoc,
    duplicated_loc: duplicatedLoc,
    duplication_percent: analysedLoc === 0 ? 0 : (duplicatedLoc / analysedLoc) * 100,
  };
}

/** The tree suites' populated metrics block, over the shared zero base. */
export function metrics(overrides: Partial<RepoMetrics> = {}): RepoMetrics {
  return zeroMetrics({
    analysed_loc: 100,
    duplicated_loc: 10,
    duplication_percent: 10,
    duplicated_files: 2,
    ...overrides,
  });
}

export function report(
  clusters: ReportCluster[],
  metricsOverride: Partial<RepoMetrics> = {},
): Report {
  return emptyReport({
    tool_version: "v",
    files_analysed: 5,
    cache_stats: { hits: 1, misses: 2 },
    metrics: metrics({ clusters_total: clusters.length, ...metricsOverride }),
    schema_doc: "docs",
    embedding_provenance: {
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      model_version: "1",
      dimensions: 768,
      attempted_subtrees: 0,
      indexed_subtrees: 0,
      failed_subtrees: 0,
    },
    clusters,
  });
}

// Save and restore a persisted `deslop.*` setting so a dispatch-style
// test that flips it cannot leak into the next test in the same
// vscode-test process. Restores the prior Global value (undefined
// clears it back to the package default).
export async function withSetting<T>(
  key: string,
  value: T,
  body: () => Promise<void> | void,
): Promise<void> {
  const cfg = () => vscode.workspace.getConfiguration("deslop");
  const previous = cfg().inspect<T>(key)?.globalValue;
  await cfg().update(key, value, vscode.ConfigurationTarget.Global);
  try {
    await body();
  } finally {
    await cfg().update(key, previous, vscode.ConfigurationTarget.Global);
  }
}

export function withGroupBy(
  value: "cluster" | "file" | "folder" | "type",
  body: () => Promise<void> | void,
): Promise<void> {
  return withSetting("topOffenders.groupBy", value, body);
}
