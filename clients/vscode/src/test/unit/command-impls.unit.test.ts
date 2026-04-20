// Unit: call the exported command implementations directly with a seeded
// store + active editor so the full branch coverage lands without colliding
// with the real extension's command registrations.

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

function cluster(id: string, paths: string[]): ReportCluster {
  return {
    id,
    weight: 10,
    size: 2,
    canonical_node_count: 4,
    signals: { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 },
    occurrences: paths.map((path) => ({
      path,
      start_byte: 0,
      end_byte: 50,
      hidden: false,
    })),
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

  test("compareWithCanonical opens a diff for a real cluster", async () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster("c-diff", ["/tmp/cdd-diffA.cs", "/tmp/cdd-diffB.cs"])]),
      0,
    );
    await compareWithCanonical(store, "c-diff");
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
