// Unit: MetricsProvider — the Duplication panel that replaces the former
// Focused File tree ([VSIX-METRICS-PANEL]). Drives getChildren() against
// a seeded store.

import * as assert from "node:assert/strict";
import { MetricsProvider, StatusTicker } from "../../tree/providers";
import { FileMetricNode, FolderMetricNode, MetricsHeadlineNode } from "../../tree/nodes";
import { ReportStore } from "../../reportStore";
import { fileMetric, labelText, report } from "./tree.helpers";

suite("MetricsProvider", () => {
  test("shows a spinner before the first report arrives", () => {
    const store = new ReportStore();
    const provider = new MetricsProvider(store, new StatusTicker());
    const [first] = provider.getChildren();
    assert.ok(first, "a placeholder row renders before any report");
    assert.equal(first.contextValue, "deslop.status.busy");
  });

  test("renders 'No duplication detected' when the codebase is clean", () => {
    const store = new ReportStore();
    store.setSnapshot(report([], { duplicated_loc: 0, duplication_percent: 0 }), 0);
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
