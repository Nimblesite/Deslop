// Unit: ClusterHoverProvider.provideHover — [VSIX-HOVER-PROVIDER].
// Picks the highest-ranked cluster whose occurrence byte range contains the
// cursor and renders the shared hover card; returns null when nothing in the
// visible projection covers the position. Runs under vscode-test so a real
// TextDocument backs the byte→position mapping.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { ClusterHoverProvider } from "../../decorations/clusterHoverProvider";
import { ReportStore } from "../../reportStore";
import { Report, ReportCluster } from "../../types/report";
import { reportWithClusters } from "./report.helpers";
import { wireCluster } from "../cluster.helpers";
import { signalsWith } from "../signals.helpers";

function reportWith(clusters: ReportCluster[]): Report {
  return reportWithClusters(clusters);
}

function clusterAt(path: string, startByte: number, endByte: number): ReportCluster {
  return wireCluster({
    id: `${path}:${startByte}:${endByte}`,
    weight: 9,
    size: 2,
    canonical_node_count: 3,
    bucket: "same_behavior",
    signals: signalsWith("same_behavior", {
      structural: 0.1,
      token_jaccard: 0.2,
      shape: 0.2,
      embedding_cos: 0.9,
    }),
    occurrences: [{ path, start_byte: startByte, end_byte: endByte, hidden: false }],
    summary: "summary",
    interpretation: "interp",
  });
}

async function openDoc(content: string): Promise<vscode.TextDocument> {
  return await vscode.workspace.openTextDocument({ content, language: "plaintext" });
}

suite("cluster hover provider", () => {
  test("provideHover renders the shared card for a cluster covering the cursor", async () => {
    const doc = await openDoc("hello world");
    const store = new ReportStore();
    store.setSnapshot(reportWith([clusterAt(doc.uri.fsPath, 0, 11)]), 0);
    const provider = new ClusterHoverProvider(store);

    const hover = provider.provideHover(doc, new vscode.Position(0, 2));
    assert.ok(hover, "the cluster's byte range covers the cursor — a hover must be returned");
    const card = hover.contents[0];
    assert.ok(card instanceof vscode.MarkdownString, "the hover card is a rendered MarkdownString");
    // The editor hover uses the compact layout (showCategory:false): no
    // taxonomy label, but the canonical line and the View-cluster action.
    assert.match(card.value, /Canonical/, "card shows the canonical occurrence section");
    assert.match(card.value, /command:deslop\.openCluster/, "card carries the open-cluster action");
  });

  test("provideHover returns null when no occurrence covers the cursor", async () => {
    const doc = await openDoc("hello world");
    const store = new ReportStore();
    // Range covers only "hello" (bytes 0-5); the cursor sits past it.
    store.setSnapshot(reportWith([clusterAt(doc.uri.fsPath, 0, 5)]), 0);
    const provider = new ClusterHoverProvider(store);

    assert.equal(provider.provideHover(doc, new vscode.Position(0, 9)), null);
  });

  test("provideHover returns null when a different file holds the only cluster", async () => {
    const doc = await openDoc("hello world");
    const store = new ReportStore();
    store.setSnapshot(reportWith([clusterAt("/some/other/file.cs", 0, 11)]), 0);
    const provider = new ClusterHoverProvider(store);

    assert.equal(provider.provideHover(doc, new vscode.Position(0, 2)), null);
  });

  test("provideHover returns null when the visible report is empty", async () => {
    const doc = await openDoc("hello world");
    const provider = new ClusterHoverProvider(new ReportStore());

    assert.equal(provider.provideHover(doc, new vscode.Position(0, 0)), null);
  });
});
