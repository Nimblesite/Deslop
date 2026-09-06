// Unit: MetricsProvider — the Duplication panel that replaces the former
// Focused File tree ([VSIX-METRICS-PANEL]). Drives getChildren() against
// a seeded store.

import * as assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";
import { resolveWorkspaceRoot } from "../../extension";
import { MetricsProvider, StatusTicker } from "../../tree/providers";
import { FileMetricNode, FolderMetricNode, MetricsHeadlineNode } from "../../tree/nodes";
import { ReportStore } from "../../reportStore";
import { fileMetric, labelText, report } from "./tree.helpers";

/** Builds a Duplication panel over a report describing exactly one file, so
 * the path-resolution cases below differ only in the path they feed in. */
function panelForOneFile(
  metricPath: string,
  analysedLoc: number,
  duplicatedLoc: number,
): MetricsProvider {
  const store = new ReportStore();
  store.setSnapshot(
    report([], {
      duplicated_loc: duplicatedLoc,
      per_file: [fileMetric(metricPath, analysedLoc, duplicatedLoc)],
    }),
    0,
  );
  return new MetricsProvider(store, new StatusTicker());
}

/** Reads the URI a file row hands to `vscode.open` when it is clicked. */
function clickTarget(row: FileMetricNode): vscode.Uri | undefined {
  return row.command?.arguments?.[0] as vscode.Uri | undefined;
}

/** Workspace root the unit suite runs against (the csharp-small fixture),
 * read through the same resolver the extension itself uses. */
function fixtureRoot(): string {
  const root = resolveWorkspaceRoot();
  assert.ok(root, "the unit suite runs with the csharp-small fixture open");
  return root;
}

suite("MetricsProvider", () => {
  test("shows a spinner before the first report arrives", () => {
    const store = new ReportStore();
    const provider = new MetricsProvider(store, new StatusTicker());
    const [first] = provider.getChildren();
    assert.ok(first, "a placeholder row renders before any report");
    assert.equal(first.contextValue, "deslop.status.busy");
  });

  test("renders 'No duplication detected' when the codebase is clean and the scan has completed", () => {
    const store = new ReportStore();
    store.setSnapshot(report([], { duplicated_loc: 0, duplication_percent: 0 }), 0);
    // The terminal "clean" verdict is gated on a completed scan: an empty
    // report alone no longer settles the lifecycle ([VSIX reactivity]).
    store.setLifecycle({ kind: "ready" });
    const provider = new MetricsProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
    const [only] = nodes;
    assert.ok(only);
    assert.match(labelText(only), /No duplication detected/);
  });

  test("headline shows the duplication score and opens the report", () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([], {
        duplication_percent: 18.4,
        duplicated_loc: 2280,
        analysed_loc: 12400,
        clusters_total: 37,
        duplicated_files: 9,
        per_file: [fileMetric("/src/a/Alpha.cs", 100, 60)],
      }),
      0,
    );
    const provider = new MetricsProvider(store, new StatusTicker());
    const [headline] = provider.getChildren();
    assert.ok(headline instanceof MetricsHeadlineNode, "first row is the headline");
    assert.match(labelText(headline), /18\.4%/);
    assert.match(labelText(headline), /duplicated/);
    assert.match(String(headline.description ?? ""), /37 clusters across 9 files/);
    assert.equal(headline.command?.command, "deslop.openDuplicationReport");
  });

  test("headline flags a breached threshold", () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([], {
        duplication_percent: 18.4,
        duplicated_loc: 2280,
        threshold: { percent: 10, breached: true, source: "config" },
        per_file: [fileMetric("/src/a/Alpha.cs", 100, 60)],
      }),
      0,
    );
    const provider = new MetricsProvider(store, new StatusTicker());
    const [headline] = provider.getChildren();
    assert.match(String(headline?.description ?? ""), /over 10\.0% gate/);
  });

  test("headline shows the configured gate when within budget", () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([], {
        duplication_percent: 8.0,
        duplicated_loc: 800,
        threshold: { percent: 20, breached: false, source: "config" },
        per_file: [fileMetric("/src/a/Alpha.cs", 100, 8)],
      }),
      0,
    );
    const provider = new MetricsProvider(store, new StatusTicker());
    const [headline] = provider.getChildren();
    assert.match(String(headline?.description ?? ""), /within 20\.0% gate/);
  });

  test("rolls per_file into a folder tree, worst-first, expanding to files", () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([], {
        duplicated_loc: 90,
        per_file: [
          fileMetric("/src/a/Alpha.cs", 100, 60),
          fileMetric("/src/a/Beta.cs", 100, 20),
          fileMetric("/src/b/Gamma.cs", 100, 10),
        ],
      }),
      0,
    );
    const provider = new MetricsProvider(store, new StatusTicker());
    const [, src] = provider.getChildren();
    assert.ok(src instanceof FolderMetricNode, "second row is a folder rollup");
    assert.equal(labelText(src), "src");
    const [folderA, folderB] = provider.getChildren(src);
    assert.ok(folderA && folderB, "src contains folders a and b");
    assert.equal(labelText(folderA), "a", "folder a (40%) sorts before b (10%)");
    assert.match(String(folderA.description ?? ""), /40\.0% duplicated/);
    assert.equal(labelText(folderB), "b");
    const [firstFile] = provider.getChildren(folderA);
    assert.ok(firstFile instanceof FileMetricNode);
    assert.equal(labelText(firstFile), "Alpha.cs", "Alpha (60%) sorts before Beta (20%)");
  });

  test("folder percentage uses the full denominator; clean files are hidden", () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([], {
        duplicated_loc: 60,
        per_file: [
          fileMetric("/src/a/Alpha.cs", 100, 60),
          fileMetric("/src/a/Clean.cs", 100, 0),
        ],
      }),
      0,
    );
    const provider = new MetricsProvider(store, new StatusTicker());
    const [, folder] = provider.getChildren();
    assert.ok(folder instanceof FolderMetricNode);
    // 60 duplicated / 200 analysed (clean file counted in the denominator).
    assert.match(String(folder.description ?? ""), /30\.0% duplicated/);
    const files = provider.getChildren(folder);
    assert.equal(files.length, 1, "the clean file is omitted from the display");
    const [onlyFile] = files;
    assert.ok(onlyFile);
    assert.equal(labelText(onlyFile), "Alpha.cs");
  });

  // [Deslop#328] The engine renders every `per_file` path relative to the
  // scan root (`report_metrics.rs` `relative_to_scan_root`, added for #286),
  // the same form occurrence rows carry. A file row must resolve that against
  // the workspace before opening, or the click lands on a phantom path at the
  // filesystem root and VS Code offers to create the file.
  test("opens the workspace file when the metric path is scan-root-relative", async () => {
    const root = fixtureRoot();
    const provider = panelForOneFile("Alpha.cs", 100, 60);
    const fileNode = provider.getChildren().find((node) => node instanceof FileMetricNode);
    assert.ok(fileNode instanceof FileMetricNode, "the relative-path metric renders a file row");

    const target = clickTarget(fileNode);
    assert.ok(target, "the file row carries an open target");
    const opened = await vscode.workspace.openTextDocument(target);
    assert.match(
      opened.getText(),
      /public class Alpha/,
      "clicking the row opens the real fixture file",
    );

    const expected = path.join(root, "Alpha.cs");
    assert.equal(target.fsPath, expected, "the open target is the workspace file");
    assert.equal(
      fileNode.resourceUri?.fsPath,
      expected,
      "resourceUri drives the file icon and decorations, so it must resolve too",
    );
  });

  // [Deslop#328] The reported case: a file several folders deep. The row is
  // reached by expanding the folder rollup, and both the click target and the
  // decoration URI must still name the workspace file.
  test("resolves a deeply nested scan-root-relative metric path against the workspace", () => {
    const root = fixtureRoot();
    const relative = "admin/src/components/ui/avatar.tsx";
    const provider = panelForOneFile(relative, 110, 110);
    const folder = provider.getChildren().find((node) => node instanceof FolderMetricNode);
    assert.ok(folder instanceof FolderMetricNode, "nested files roll up into a folder row");
    const [fileNode] = provider.getChildren(folder);
    assert.ok(fileNode instanceof FileMetricNode, "the folder expands to the file row");
    assert.equal(labelText(fileNode), "avatar.tsx");

    const expected = path.join(root, ...relative.split("/"));
    const target = clickTarget(fileNode);
    assert.equal(target?.fsPath, expected, "the click target is the workspace file");
    assert.equal(fileNode.resourceUri?.fsPath, expected, "so is the decoration URI");
  });

  // [Deslop#328] An absolute `per_file` path must survive untouched — the
  // resolver may not prefix the workspace onto a path that already has a root.
  test("leaves an absolute metric path untouched", () => {
    const absolute = path.join(path.sep, "elsewhere", "Alpha.cs");
    const provider = panelForOneFile(absolute, 100, 60);
    const folder = provider.getChildren().find((node) => node instanceof FolderMetricNode);
    assert.ok(folder instanceof FolderMetricNode);
    const [fileNode] = provider.getChildren(folder);
    assert.ok(fileNode instanceof FileMetricNode);
    assert.equal(
      clickTarget(fileNode)?.fsPath,
      absolute,
      "an already-absolute path is used verbatim",
    );
  });

  test("surfaces a failed lifecycle as an error status row", () => {
    const store = new ReportStore();
    store.setLifecycle({ kind: "failed", message: "oh no" });
    const provider = new MetricsProvider(store, new StatusTicker());
    const errorNode = provider
      .getChildren()
      .find((node) => node.contextValue === "deslop.status.error");
    assert.ok(errorNode, "duplication panel must show a failed-lifecycle banner");
    assert.match(labelText(errorNode), /Stopped: oh no/);
  });
});
