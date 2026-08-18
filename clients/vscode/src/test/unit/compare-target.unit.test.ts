// Unit: compare command target resolution. The context menu passes tree
// nodes, while links and webviews pass cluster ids.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";

import { compareWithCanonicalTarget } from "../../commands/register";
import { ReportStore } from "../../reportStore";
import { OccurrenceNode } from "../../tree/providers";
import { Report, ReportCluster } from "../../types/report";
import { reportWithClusters } from "./report.helpers";
import { bucketSignals } from "../signals.helpers";

async function findDiffTab(): Promise<vscode.TabInputTextDiff> {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    for (const group of vscode.window.tabGroups.all) {
      for (const tab of group.tabs) {
        if (tab.input instanceof vscode.TabInputTextDiff) return tab.input;
      }
    }
    await new Promise<void>((resolve) => {
      setTimeout(resolve, 50);
    });
  }
  throw new Error("no diff tab opened after compare target resolution");
}

function report(clusters: ReportCluster[]): Report {
  return reportWithClusters(
    clusters,
    {},
    { analysed_loc: 10, duplicated_loc: 5, duplication_percent: 50, duplicated_files: 1 },
  );
}

function cluster(
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
    occurrences: occurrences.map((occurrence) => ({ ...occurrence, hidden: false })),
    occurrences_total: 0,
    occurrences_truncated: false,
    summary: "",
    interpretation: "",
  };
}

suite("compare command targets", () => {
  test("occurrence tree rows compare their parent cluster", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-cmp-target-"));
    const canonical = path.join(dir, "Canonical.cs");
    const ignored = path.join(dir, "Ignored.cs");
    const sibling = path.join(dir, "Sibling.cs");
    const canonicalText = "public class Canonical { }\n";
    const ignoredText = "public class Ignored { }\n";
    const siblingText = "public class SelectedSibling { }\n";
    fs.writeFileSync(canonical, canonicalText, "utf8");
    fs.writeFileSync(ignored, ignoredText, "utf8");
    fs.writeFileSync(sibling, siblingText, "utf8");

    try {
      const c = cluster("c-target", [
        { path: canonical, start_byte: 0, end_byte: canonicalText.length },
        { path: ignored, start_byte: 0, end_byte: ignoredText.length },
        { path: sibling, start_byte: 0, end_byte: siblingText.length },
      ]);
      const occurrence = c.occurrences[2];
      assert.ok(occurrence);
      const store = new ReportStore();
      store.setSnapshot(report([c]), 0);

      await vscode.commands.executeCommand("workbench.action.closeAllEditors");
      await compareWithCanonicalTarget(store, new OccurrenceNode(occurrence));
      const diff = await findDiffTab();

      assert.equal(diff.original.scheme, "deslop-compare");
      assert.equal(diff.modified.scheme, "deslop-compare");
      assert.notEqual(diff.original.toString(), diff.modified.toString());

      const original = await vscode.workspace.openTextDocument(diff.original);
      const modified = await vscode.workspace.openTextDocument(diff.modified);
      assert.equal(original.getText(), canonicalText);
      assert.equal(modified.getText(), siblingText);
      assert.notEqual(modified.getText(), ignoredText);
    } finally {
      await vscode.commands.executeCommand("workbench.action.closeAllEditors");
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  test("unresolved compare targets are no-ops", async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    await compareWithCanonicalTarget(new ReportStore(), { command: "no cluster" });
  });
});
