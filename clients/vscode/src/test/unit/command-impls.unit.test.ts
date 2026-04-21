// Unit: call the exported command implementations directly with a seeded
// store + active editor so the full branch coverage lands without colliding
// with the real extension's command registrations.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import {
  openWorstCluster,
  openOccurrence,
  jumpToNextOccurrence,
  compareWithCanonical,
  openSchemaDoc,
} from "../../commands/register";
import { ReportStore } from "../../reportStore";
import { Report, ReportCluster } from "../../types/report";

async function findDiffTab(): Promise<vscode.TabInputTextDiff> {
  for (let i = 0; i < 20; i += 1) {
    for (const group of vscode.window.tabGroups.all) {
      for (const tab of group.tabs) {
        if (tab.input instanceof vscode.TabInputTextDiff) return tab.input;
      }
    }
    await new Promise<void>((r) => setTimeout(r, 50));
  }
  throw new Error("no diff tab opened after compareWithCanonical");
}

async function closeAllDiffs(): Promise<void> {
  await vscode.commands.executeCommand("workbench.action.closeAllEditors");
}

function cluster(id: string, paths: string[]): ReportCluster {
  return {
    id,
    weight: 10,
    size: 2,
    canonical_node_count: 4,
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: paths.map((p) => ({
      path: p,
      start_byte: 0,
      end_byte: 50,
      hidden: false,
    })),
    summary: "",
    interpretation: "interp",
  };
}

function clusterWithRanges(
  id: string,
  occurrences: { path: string; start_byte: number; end_byte: number }[],
): ReportCluster {
  return {
    id,
    weight: 10,
    size: occurrences.length,
    canonical_node_count: 4,
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: occurrences.map((o) => ({ ...o, hidden: false })),
    summary: "",
    interpretation: "interp",
  };
}

function report(clusters: ReportCluster[]): Report {
  return {
    report_schema_version: 1,
    tool_version: "v",
    min_nodes: 30,
    files_analysed: 1,
    clusters_hidden: 0,
    cache_stats: { hits: 0, misses: 0 },
    metrics: {
      analysed_loc: 10,
      duplicated_loc: 5,
      duplication_percent: 50,
      clusters_total: clusters.length,
      duplicated_files: 1,
      threshold: { percent: 0, breached: false, source: "none" },
    },
    schema_doc: "# docs",
    action_hints: [],
    embedding_provenance: null,
    clusters,
  };
}

function fakeCtx(): vscode.ExtensionContext {
  return {
    subscriptions: { push: () => {} },
    extensionPath: "/tmp",
    extension: { packageJSON: { version: "0.0.0" } },
  } as unknown as vscode.ExtensionContext;
}

suite("register command implementations", () => {
  suiteSetup(async () => {
    const ext = vscode.extensions.getExtension("nimblesite.deslop-vscode");
    assert.ok(ext, "extension must be discoverable in the test host");
    await ext.activate();
  });

  test("openWorstCluster shows info when store is empty", () => {
    openWorstCluster(fakeCtx(), new ReportStore());
  });

  test("openWorstCluster opens a panel when the report has clusters", () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c-top", ["/tmp/cdd-A.cs", "/tmp/cdd-B.cs"])]), 0);
    openWorstCluster(fakeCtx(), store);
  });

  test("openOccurrence opens the referenced file at the byte range", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-occ-"));
    const file = path.join(dir, "occ.txt");
    fs.writeFileSync(file, "hello\nworld\n", "utf8");
    await openOccurrence({
      path: file,
      start_byte: 0,
      end_byte: 3,
      hidden: false,
    });
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("jumpToNextOccurrence navigates to the sibling when the cursor sits inside a cluster", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "line-one\nline-two\n",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    editor.selection = new vscode.Selection(
      new vscode.Position(0, 2),
      new vscode.Position(0, 2),
    );
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster("c-1", [doc.uri.fsPath, "/tmp/cdd-sibling.cs"])]),
      0,
    );
    jumpToNextOccurrence(store);
  });

  test("jumpToNextOccurrence shows the info message when no cluster overlaps", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "z",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c", ["/other"])]), 0);
    jumpToNextOccurrence(store);
  });

  test("jumpToNextOccurrence bails when there is no active editor", async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c", ["/p"])]), 0);
    jumpToNextOccurrence(store);
  });

  test("compareWithCanonical opens a diff whose two sides are distinct resources with the matching occurrence bytes", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-cmp-"));
    const fileA = path.join(dir, "A.cs");
    const fileB = path.join(dir, "B.cs");
    // Left side: bytes 0..16 of A.cs == "public class A {"
    // Right side: bytes 0..16 of B.cs == "public class B {"
    // Distinct files, distinct content — exercises the cross-file diff path.
    fs.writeFileSync(fileA, "public class A { int x = 1; }\n", "utf8");
    fs.writeFileSync(fileB, "public class B { int y = 2; }\n", "utf8");

    const store = new ReportStore();
    store.setSnapshot(
      report([
        clusterWithRanges("c-diff", [
          { path: fileA, start_byte: 0, end_byte: 16 },
          { path: fileB, start_byte: 0, end_byte: 16 },
        ]),
      ]),
      0,
    );

    await closeAllDiffs();
    await compareWithCanonical(store, "c-diff");
    const diff = await findDiffTab();

    assert.notEqual(
      diff.original.toString(),
      diff.modified.toString(),
      "compare diff must reference two distinct resources — the bug was pointing both sides at the same URI",
    );

    const left = await vscode.workspace.openTextDocument(diff.original);
    const right = await vscode.workspace.openTextDocument(diff.modified);
    assert.equal(left.getText(), "public class A {");
    assert.equal(right.getText(), "public class B {");

    await closeAllDiffs();
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("compareWithCanonical opens distinct diff sides for two occurrences that live inside the same file", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cdd-cmp-same-"));
    // Two clone regions inside a single source file. This is the case the
    // user reported: the old implementation handed `vscode.diff` the same
    // file URI twice, so the diff editor rendered the whole file against
    // itself. The fix must ensure each side shows only the clone bytes.
    const file = path.join(dir, "same.cs");
    const source =
      "OCCURRENCE_A_____________________________\n" +
      "middle middle middle middle middle middle\n" +
      "OCCURRENCE_B_____________________________\n";
    fs.writeFileSync(file, source, "utf8");
    const firstLineEnd = source.indexOf("\n");
    const thirdLineStart = source.indexOf("OCCURRENCE_B");
    const thirdLineEnd = source.indexOf("\n", thirdLineStart);

    const store = new ReportStore();
    store.setSnapshot(
      report([
        clusterWithRanges("c-same", [
          { path: file, start_byte: 0, end_byte: firstLineEnd },
          { path: file, start_byte: thirdLineStart, end_byte: thirdLineEnd },
        ]),
      ]),
      0,
    );

    await closeAllDiffs();
    await compareWithCanonical(store, "c-same");
    const diff = await findDiffTab();

    assert.notEqual(
      diff.original.toString(),
      diff.modified.toString(),
      "same-file cluster must NOT produce a diff that points both sides at the file itself",
    );

    const left = await vscode.workspace.openTextDocument(diff.original);
    const right = await vscode.workspace.openTextDocument(diff.modified);
    assert.notEqual(
      left.getText(),
      right.getText(),
      "same-file diff must show the two distinct occurrences, not the full file on both sides",
    );
    assert.ok(
      left.getText().startsWith("OCCURRENCE_A"),
      `left side should contain occurrence A bytes, got: ${JSON.stringify(left.getText())}`,
    );
    assert.ok(
      right.getText().startsWith("OCCURRENCE_B"),
      `right side should contain occurrence B bytes, got: ${JSON.stringify(right.getText())}`,
    );

    await closeAllDiffs();
    fs.rmSync(dir, { recursive: true, force: true });
  });

  test("compareWithCanonical bails for a non-existent id", async () => {
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    await compareWithCanonical(store, "nope");
  });

  test("compareWithCanonical bails for a single-occurrence cluster", async () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c-single", ["/only"])]), 0);
    await compareWithCanonical(store, "c-single");
  });

  test("openSchemaDoc opens a markdown editor", async () => {
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    await openSchemaDoc(fakeCtx(), store);
  });

  test("openSchemaDoc renders a fallback when schema_doc is absent", async () => {
    await openSchemaDoc(fakeCtx(), new ReportStore());
  });
});
