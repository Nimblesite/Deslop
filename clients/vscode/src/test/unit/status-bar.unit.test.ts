// Unit: status bar drives + pure helpers. setAnalysing flips a flag that
// re-renders, and sameFile/shortPath are the same canonical pair used across
// the extension — kept tiny for hot-loop rendering.

import * as assert from "node:assert/strict";
import { StatusBar, sameFile, shortPath } from "../../commands/statusBar";
import { ReportStore } from "../../reportStore";
import { Report } from "../../types/report";
import { emptyReport, repoMetrics } from "./report.helpers";
import { occurrence, wireCluster } from "../cluster.helpers";

function report(): Report {
  return emptyReport({
    tool_version: "v",
    files_analysed: 5,
    cache_stats: { hits: 1, misses: 2 },
    metrics: repoMetrics({
      analysed_loc: 200,
      duplicated_loc: 50,
      duplication_percent: 25,
      clusters_total: 2,
      duplicated_files: 1,
    }),
    clusters: [
      wireCluster({
        id: "a",
        mass: 10,
        canonical_node_count: 3,
                occurrences: [occurrence("/tmp/A/Alpha.cs", 0, 10)],
      }),
    ],
  });
}

suite("statusBar", () => {
  test("setAnalysing toggles the busy state without throwing", () => {
    const store = new ReportStore();
    const bar = new StatusBar(store);
    bar.setAnalysing(true);
    bar.setAnalysing(false);
    store.setSnapshot(report(), 0);
    bar.dispose();
  });

  test("sameFile compares exact + suffix forms", () => {
    assert.equal(sameFile("/a/b/file.cs", "/a/b/file.cs"), true);
    assert.equal(sameFile("/a/b/file.cs", "file.cs"), true);
    assert.equal(sameFile("/a/b/file.cs", "/x/file.cs"), false);
  });

  test("shortPath strips everything before the last separator", () => {
    assert.equal(shortPath("/a/b/c.cs"), "c.cs");
    assert.equal(shortPath("C:\\a\\b\\c.cs"), "c.cs");
    assert.equal(shortPath("basename"), "basename");
  });
});
