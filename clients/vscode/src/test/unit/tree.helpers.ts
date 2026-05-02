// Shared factories for tree-provider unit tests.
// Non-`.test.ts` so the Mocha glob does not load this as a suite.

import * as vscode from "vscode";
import { Bucket, Report, ReportCluster } from "../../types/report";

export function cluster(
  id: string,
  weight: number,
  occurrencePath: string,
  startByte = 0,
  endByte = 20,
  bucket: Bucket = "identical",
): ReportCluster {
  return {
    id,
    weight,
    size: 2,
    canonical_node_count: 4,
    signals: bucketSignals(bucket),
    bucket,
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

export function bucketSignals(bucket: Bucket) {
  if (bucket === "nearly_identical") {
    return { structural: 0.99, token_jaccard: 0.96, embedding_cos: 0, fused: 0.96 };
  }
  if (bucket === "loosely_similar") {
    return { structural: 0.2, token_jaccard: 0.4, embedding_cos: 0, fused: 0.4 };
  }
  if (bucket === "same_behavior") {
    return { structural: 0.2, token_jaccard: 0.3, embedding_cos: 0.9, fused: 0.9 };
  }
  return { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 };
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

export function report(clusters: ReportCluster[]): Report {
  return {
    tool_version: "v",
    min_nodes: 30,
    files_analysed: 5,
    clusters_hidden: 0,
    cache_stats: { hits: 1, misses: 2 },
    metrics: {
      analysed_loc: 100,
      duplicated_loc: 10,
      duplication_percent: 10,
      clusters_total: clusters.length,
      duplicated_files: 2,
      threshold: { percent: 0, breached: false, source: "none" },
    },
    schema_doc: "docs",
    action_hints: [],
    embedding_provenance: {
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      model_version: "1",
      dimensions: 768,
    },
    clusters,
  };
}

// Save and restore the persisted Top Offenders grouping mode so a
// dispatch-style test that flips the setting cannot leak into the
// next test in the same vscode-test process.
export async function withGroupBy(
  value: "cluster" | "file",
  body: () => Promise<void> | void,
): Promise<void> {
  const cfg = vscode.workspace.getConfiguration("deslop");
  const previous = cfg.get<string>("topOffenders.groupBy", "cluster");
  await cfg.update(
    "topOffenders.groupBy",
    value,
    vscode.ConfigurationTarget.Global,
  );
  try {
    await body();
  } finally {
    await cfg.update(
      "topOffenders.groupBy",
      previous === "file" ? "file" : undefined,
      vscode.ConfigurationTarget.Global,
    );
  }
}
