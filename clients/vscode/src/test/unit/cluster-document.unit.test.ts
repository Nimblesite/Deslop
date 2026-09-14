import * as assert from "node:assert/strict";
import * as vscode from "vscode";

import { clusterDocumentContent } from "../../clusterDocument";
import type { Report, ReportCluster } from "../../types/report";
import type { ClusterFixture } from "../cluster.helpers";
import { emptyReport, repoMetrics } from "./report.helpers";
import { FIXTURE_KIND, occurrence, wireCluster } from "../cluster.helpers";
import { kindTitle } from "../../types/report";

/** A mass whose two-decimal rendering (`527.00`) differs from its count. */
const WHOLE_NUMBER_MASS = 527;
const WHOLE_NUMBER_MASS_LINE = "Mass: 527";
/** The document names the cluster's clone kind by its title ([CLONE-KIND-LABELS]). */
const KIND_LINE = `Kind: ${kindTitle(FIXTURE_KIND)}`;
const TWO_DECIMAL_MASS = "527.00";

function cluster(overrides: Partial<ClusterFixture> = {}): ReportCluster {
  return wireCluster({
    id: "cluster-for-test",
    mass: WHOLE_NUMBER_MASS,
    canonical_node_count: 12,
    occurrences: [
      {
        ...occurrence("/repo/Alpha.cs", 5, 30),
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
        ...occurrence("/repo/Beta.cs", 40, 70),
        hidden: true,
      },
    ],
    occurrences_total: 4,
    ...overrides,
  });
}

function report(clusters: ReportCluster[] = [cluster()]): Report {
  return emptyReport({
    tool_version: "test",
    files_analysed: 2,
    metrics: repoMetrics({ clusters_total: clusters.length }),
    clusters,
  });
}

suite("cluster document", () => {
  test("renders authority-style cluster URIs", () => {
    const body = clusterDocumentContent(
      vscode.Uri.parse("deslop://cluster/cluster-for-test"),
      report(),
    );

    assert.ok(body.includes("# Deslop cluster cluster-for-test"));
    assert.ok(
      body.split("\n").includes(KIND_LINE),
      `the document must carry the line "${KIND_LINE}":\n${body}`,
    );
    assert.ok(body.includes("Occurrences: 4"));
    // [RANK-MASS-SUM] The document names the cluster's mass — the engine's
    // ranking metric — as the whole number it is, through the shared
    // formatter ([PRINCIPLES-ONE-CALCULATION]), never at score precision.
    assert.ok(
      body.split("\n").includes(WHOLE_NUMBER_MASS_LINE),
      `the document must carry the line "${WHOLE_NUMBER_MASS_LINE}":\n${body}`,
    );
    assert.equal(
      body.includes(TWO_DECIMAL_MASS),
      false,
      "a count must never be printed with the two-decimal signal precision",
    );
    assert.ok(body.includes("1. /repo/Alpha.cs:2:6"));
    assert.ok(body.includes("2. /repo/Beta.cs hidden"));
  });

  test("renders path-style cluster URIs", () => {
    const body = clusterDocumentContent(
      vscode.Uri.parse("deslop:/cluster/cluster-for-test"),
      report(),
    );

    assert.ok(body.includes("# Deslop cluster cluster-for-test"));
    // [FUSED-PAIR-SIGNALS] The cluster document is a cluster surface and
    // renders no pair evidence — no pair line, no signal values.
    for (const gone of ["Elected pair:", "Measured pair:", "Pair signals:", "structural", "jaccard", "embedding", "pair_agreement"]) {
      assert.equal(body.includes(gone), false, `cluster document must not render ${gone}`);
    }
  });

  test("the cluster document never carries pair signals", () => {
    const body = clusterDocumentContent(
      vscode.Uri.parse("deslop://cluster/cluster-for-test"),
      report([cluster({ occurrences: [] })]),
    );

    assert.equal(body.includes("Elected pair:"), false);
    assert.equal(body.includes("Measured pair:"), false);
    assert.equal(body.includes("Pair signals:"), false);
    assert.equal(body.includes("structural"), false);
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
