// Unit: canonical cluster file command. Keeps the focused coverage out of the
// older command-impls file, which is already over the repo line budget.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";

import { openCanonicalOccurrence } from "../../commands/register";
import { canonicalOccurrenceForCluster } from "../../commands/treeMenus";
import { ClusterNode } from "../../tree/providers";
import { ReportCluster } from "../../types/report";
import { bucketSignals } from "../signals.helpers";

function clusterWithRanges(
  id: string,
  occurrences: { path: string; start_byte: number; end_byte: number }[],
): ReportCluster {
  return {
    id,
    weight: 10,
    size: occurrences.length,
    canonical_node_count: 4,
    bucket: "identical",
    signals: bucketSignals("identical"),
    occurrences: occurrences.map((o) => ({ ...o, hidden: false })),
    occurrences_total: 0,
    occurrences_truncated: false,
    summary: "",
    interpretation: "interp",
  };
}

function clusterNodeFor(c: ReportCluster): ClusterNode {
  return new ClusterNode(c, 1, "mid");
}

suite("canonical file command", () => {
  test("canonicalOccurrenceForCluster returns the first occurrence", () => {
    const c = clusterWithRanges("c-canonical", [
      { path: "src/canonical.cs", start_byte: 10, end_byte: 20 },
      { path: "src/sibling.cs", start_byte: 30, end_byte: 40 },
    ]);
    const first = c.occurrences[0];
    assert.ok(first, "cluster must have a canonical occurrence");
    assert.strictEqual(canonicalOccurrenceForCluster(clusterNodeFor(c)), first);
  });

  test("openCanonicalOccurrence opens the first occurrence at its line and column", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-canon-"));
    const canonical = path.join(dir, "Canonical.cs");
    const sibling = path.join(dir, "Sibling.cs");
    const source = "zero\n  canonical target\n";
    const startByte = Buffer.byteLength("zero\n  ", "utf8");
    const endByte = startByte + Buffer.byteLength("canonical", "utf8");
    fs.writeFileSync(canonical, source, "utf8");
    fs.writeFileSync(sibling, "sibling target\n", "utf8");
    const c = clusterWithRanges("c-open-canonical", [
      { path: canonical, start_byte: startByte, end_byte: endByte },
      { path: sibling, start_byte: 0, end_byte: 7 },
    ]);

    try {
      await openCanonicalOccurrence(clusterNodeFor(c));
      const editor = vscode.window.activeTextEditor;
      assert.ok(editor, "canonical command must open an editor");
      assert.equal(editor.document.uri.fsPath, canonical);
      assert.equal(editor.selection.start.line, 1);
      assert.equal(editor.selection.start.character, 2);
      assert.equal(editor.selection.end.character, 11);
    } finally {
      await vscode.commands.executeCommand("workbench.action.closeAllEditors");
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  test("openCanonicalOccurrence is a no-op for an empty cluster", async () => {
    const c = clusterWithRanges("c-empty", []);
    await openCanonicalOccurrence(clusterNodeFor(c));
  });
});
