import * as assert from "node:assert/strict";
import * as vscode from "vscode";

import { clusterDocumentContent } from "../../clusterDocument";
import type { Report, ReportCluster } from "../../types/report";

function cluster(overrides: Partial<ReportCluster> = {}): ReportCluster {
  return {
    id: "cluster-for-test",
    weight: 12.345,
    size: 2,
    canonical_node_count: 12,
    bucket: "identical",
    signals: {
      structural: 1,
      token_jaccard: 0.875,
      embedding_cos: 0.25,
      fused: 0.9,
    },
    occurrences: [
      {
        path: "/repo/Alpha.cs",
        start_byte: 5,
        end_byte: 30,
        hidden: false,
        displayLocation: {
          label: "/repo/Alpha.cs:2:6",
          line: 2,
          column: 6,
          description: "line 2, column 6",
          commandTitle: "Open /repo/Alpha.cs at line 2, column 6",
        },
      },
      {
        path: "/repo/Beta.cs",
        start_byte: 40,
        end_byte: 70,
        hidden: true,
      },
    ],
    occurrences_total: 4,
    occurrences_truncated: false,
    summary: "",
    interpretation: "",
    ...overrides,
  };
}

function report(clusters: ReportCluster[] = [cluster()]): Report {
  return {
    report_schema_version: 1,
    tool_version: "test",
    min_nodes: 30,
    files_analysed: 2,
    clusters_hidden: 0,
    cache_stats: { hits: 0, misses: 0 },
    metrics: {
      analysed_loc: 0,
      duplicated_loc: 0,
      duplication_percent: 0,
      clusters_total: clusters.length,
      duplicated_files: 0,
      threshold: { percent: 0, breached: false, source: "none" },
    },
    schema_doc: "",
    action_hints: [],
    embedding_provenance: null,
    clusters,
  };
}

suite("cluster document", () => {
  test("renders authority-style cluster URIs", () => {
    const body = clusterDocumentContent(
      vscode.Uri.parse("deslop://cluster/cluster-for-test"),
      report(),
    );

    assert.ok(body.includes("# Deslop cluster cluster-for-test"));
    assert.ok(body.includes("Occurrences: 4"));
    assert.ok(body.includes("Weight: 12.35"));
    assert.ok(body.includes("structural 1.00"));
    assert.ok(body.includes("jaccard 0.88"));
    assert.ok(body.includes("embedding 0.25"));
    assert.ok(body.includes("1. /repo/Alpha.cs:2:6"));
    assert.ok(body.includes("2. /repo/Beta.cs hidden"));
  });

  test("renders path-style cluster URIs", () => {
    const body = clusterDocumentContent(
      vscode.Uri.parse("deslop:/cluster/cluster-for-test"),
      report(),
    );

    assert.ok(body.includes("# Deslop cluster cluster-for-test"));
  });

  test("renders invalid URI diagnostics", () => {
    const uri = vscode.Uri.parse("file:///tmp/cluster-for-test");
    const body = clusterDocumentContent(uri, report());

    assert.ok(body.includes("Unable to parse cluster id"));
    assert.ok(body.includes("Expected deslop://cluster/<id>."));
  });

  test("renders missing cluster diagnostics", () => {
    const body = clusterDocumentContent(
      vscode.Uri.parse("deslop://cluster/missing"),
      report([]),
    );

    assert.ok(body.includes("# Deslop cluster missing"));
    assert.ok(body.includes("not present in the current report snapshot"));
  });

  test("renders missing report diagnostics", () => {
    const body = clusterDocumentContent(
      vscode.Uri.parse("deslop://cluster/cluster-for-test"),
      null,
    );

    assert.ok(body.includes("# Deslop cluster cluster-for-test"));
    assert.ok(body.includes("Refresh the Deslop report"));
  });
});
