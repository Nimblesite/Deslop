import { FIXTURE_ROUTING } from "../cluster.helpers";
import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { StatusTicker, TopOffendersProvider } from "../../tree/providers";
import { ReportStore } from "../../reportStore";
import { cluster, labelText, metrics, report, storeWith, topOffenders } from "./tree.helpers";
import { HIGHEST_CLUSTER_MASS, HIGH_CLUSTER_MASS, TIED_CLUSTER_MASS, MIXED_FILE_PATH, REPO_A_PATH, REPO_B_PATH, ANALYSING_PHASE, FIRST_CLUSTER_ID, SECOND_CLUSTER_ID, FIRST_FIXTURE_PATH, PAIR_COUNT, SECOND_GENERATION, CACHE_HIT_COUNT, EXPECTED_TREE_REFRESH_COUNT, MIN_VISIBLE_NODE_COUNT, LOW_CLUSTER_WEIGHT, DIRTY_OCCURRENCE_START_BYTE, DIRTY_FILE_PATH, reportOccurrence, withOccurrences } from "./tree.topOffenders.fixtures";

suite("TopOffendersProvider lifecycle", () => {
  test("renders an Analysing… placeholder before the first report arrives", () => {
    const store = new ReportStore();
    const provider = topOffenders(store);
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  // [VSIX reactivity] The reported bug: a large codebase shows "No
  // duplication detected" while it is still being scanned. The terminal
  // clean verdict must wait until the server confirms it is idle.
  test("never claims 'No duplication detected' before the scan completes", () => {
    const store = new ReportStore();
    store.setLifecycle({ kind: ANALYSING_PHASE });
    store.setSnapshot(report([]), 0);
    const provider = topOffenders(store);
    const [only] = provider.getChildren();
    assert.ok(only, "a progress row must render while scanning");
    assert.equal(only.contextValue, "deslop.status.busy", "mid-scan empty state shows a busy row");
    assert.doesNotMatch(
      labelText(only),
      /No duplication/,
      "must not declare the codebase clean until the server reports idle",
    );
  });

  test("declares 'No duplication detected' only once the server reports idle (ready) with no clusters", () => {
    const store = storeWith(report([]));
    store.setLifecycle({ kind: "ready" });
    const provider = topOffenders(store);
    const [only] = provider.getChildren();
    assert.ok(only);
    assert.equal(only.contextValue, "deslop.status.info");
    assert.match(labelText(only), /No duplication detected/);
  });

  // [req: incremental indicator] An edit-triggered re-analysis keeps the
  // existing clusters on screen (stale > blank) and leads with a busy
  // badge so the user can see an update is in flight.
  test("leads with an 'Analysing changes…' badge during incremental re-analysis", () => {
    const store = storeWith(report([cluster(FIRST_CLUSTER_ID, HIGHEST_CLUSTER_MASS, REPO_A_PATH), cluster(SECOND_CLUSTER_ID, HIGH_CLUSTER_MASS, REPO_B_PATH)]));
    store.setLifecycle({ kind: ANALYSING_PHASE });
    const provider = topOffenders(store);
    const nodes = provider.getChildren();
    const [first] = nodes;
    assert.ok(first, "a badge row leads the list during re-analysis");
    assert.equal(first.contextValue, "deslop.status.busy", "the badge is a busy row");
    assert.match(
      labelText(first),
      /Analysing changes/,
      "the badge names the in-flight incremental work",
    );
    const labels = nodes.map(labelText);
    assert.ok(
      labels.some((l) => /A\.cs/.test(l)),
      "existing clusters stay visible during re-analysis",
    );
  });

  test("setting flip refreshes the tree", () => {
    // [VSIX-TOP-OFFENDERS-GROUPING] The provider exposes a refresh()
    // hook the activation bridge calls when the setting changes.
    const store = storeWith(report([cluster("c", HIGHEST_CLUSTER_MASS, MIXED_FILE_PATH)]));
    const provider = topOffenders(store);
    let fires = 0;
    const sub = provider.onDidChangeTreeData(() => {
      fires += 1;
    });
    try {
      provider.refresh();
      assert.ok(fires >= 1, "refresh must fire the tree-data change emitter");
    } finally {
      sub.dispose();
    }
  });

  test("getTreeItem returns the node verbatim", () => {
    const store = storeWith(report([cluster("a", LOW_CLUSTER_WEIGHT, FIRST_FIXTURE_PATH)]));
    const provider = topOffenders(store);
    const [root] = provider.getChildren();
    assert.ok(root, "root node must exist");
    assert.strictEqual(provider.getTreeItem(root), root);
  });

  test("reacts to LSP-fed store snapshots and deltas", () => {
    const store = new ReportStore();
    const ticker = new StatusTicker();
    const provider = new TopOffendersProvider(store, ticker);
    let treeRefreshes = 0;
    const sub = provider.onDidChangeTreeData(() => {
      treeRefreshes += 1;
    });

    try {
      store.setSnapshot(report([cluster("stale", 1, "/stale.cs")]), 1);
      assert.equal(treeRefreshes, 1, "snapshot must refresh the tree");
      assert.match(
        String(provider.getChildren()[0]?.description ?? ""),
        /\brank\s+#1\b.*\b\d+ copies\b/,
        "description must show rank then copy count after snapshot",
      );

      store.applyDelta({
      routing: FIXTURE_ROUTING,
        from_generation: 1,
        to_generation: SECOND_GENERATION,
        clusters_added: [cluster("fresh", TIED_CLUSTER_MASS, "/fresh.cs")],
        clusters_removed: ["stale"],
        clusters_updated: [],
    literal_findings_added: [],
    literal_findings_removed: [],
    literal_findings_updated: [],
        metrics: metrics(),
        cache_stats: { hits: CACHE_HIT_COUNT, misses: 0 },
        tool_version: "v2",
      });

      assert.equal(treeRefreshes, EXPECTED_TREE_REFRESH_COUNT, "delta must refresh the tree");
      assert.match(
        String(provider.getChildren()[0]?.description ?? ""),
        /\brank\s+#1\b.*\b\d+ copies\b/,
        "description must show rank then copy count after delta",
      );
    } finally {
      sub.dispose();
      provider.dispose();
      ticker.dispose();
    }
  });

  test("does not surface removed-cluster progress or historical counts (#128)", () => {
    const store = storeWith(
      report([
        cluster("fixed", HIGHEST_CLUSTER_MASS, "/repo/Fixed.cs"),
        cluster("next", 95, "/repo/Next.cs"),
        cluster("still", HIGH_CLUSTER_MASS, "/repo/Still.cs"),
      ]),
      1,
    );
    const provider = topOffenders(store);

    store.applyDelta({
      routing: FIXTURE_ROUTING,
      from_generation: 1,
      to_generation: SECOND_GENERATION,
      clusters_added: [],
      clusters_removed: ["fixed"],
      clusters_updated: [],
    literal_findings_added: [],
    literal_findings_removed: [],
    literal_findings_updated: [],
      metrics: metrics(),
      cache_stats: { hits: CACHE_HIT_COUNT, misses: 0 },
      tool_version: "v2",
    });

    const nodes = provider.getChildren();
    const labels = nodes.map(labelText);
    const joined = labels.join("\n");

    assert.equal(nodes.length, PAIR_COUNT, "top offenders must only show current report clusters");
    assert.match(labels[0] ?? "", /Next\.cs/, "highest remaining offender must be first");
    assert.ok(labels.some((label) => /Next\.cs/.test(label)), "next offender must remain visible");
    assert.ok(labels.some((label) => /Still\.cs/.test(label)), "remaining offender must remain visible");
    assert.doesNotMatch(joined, /no longer reported/i, "removed-cluster history is not product state");
    assert.doesNotMatch(joined, /\bremaining\b/i, "top offenders must not show historical counters");
    assert.doesNotMatch(joined, /generation\s+\d+/i, "top offenders must not expose generation summaries");
    assert.doesNotMatch(joined, /Fixed\.cs/, "removed cluster must leave the offender list");
  });

  test("dirty file edits prune stale offsets from top offenders immediately (#78)", () => {
    const dirtyOnly = withOccurrences(
      cluster("dirty-only", 100, DIRTY_FILE_PATH),
      [reportOccurrence(DIRTY_FILE_PATH, DIRTY_OCCURRENCE_START_BYTE, 20)],
    );
    const mixedSingleton = withOccurrences(
      cluster("mixed-singleton", 95, DIRTY_FILE_PATH),
      [
        reportOccurrence(DIRTY_FILE_PATH, 30, 40),
        reportOccurrence("/repo/Clean.cs", 50, 60),
      ],
    );
    const mixedPeers = withOccurrences(
      cluster("mixed-peers", 90, DIRTY_FILE_PATH),
      [
        reportOccurrence(DIRTY_FILE_PATH, 70, 80),
        reportOccurrence("/repo/CleanA.cs", 90, 100),
        reportOccurrence("/repo/CleanB.cs", 110, 120),
      ],
    );
    const clean = withOccurrences(
      cluster("clean", 80, "/repo/Other.cs"),
      [
        reportOccurrence("/repo/OtherA.cs", 130, 140),
        reportOccurrence("/repo/OtherB.cs", 150, 160),
      ],
    );
    const store = storeWith(report([dirtyOnly, mixedSingleton, mixedPeers, clean]), 9);
    const ticker = new StatusTicker();
    const provider = new TopOffendersProvider(store, ticker);
    let treeRefreshes = 0;
    const sub = provider.onDidChangeTreeData(() => {
      treeRefreshes += 1;
    });

    try {
      const before = provider.getChildren();
      assert.equal(before.length, 4, "fixture starts with four top-offender rows");
      assert.match(before.map(labelText).join("\n"), /Dirty\.cs/, "fixture must expose dirty offsets");

      store.markFileDirty(DIRTY_FILE_PATH);

      const after = provider.getChildren();
      const labels = after.map(labelText);
      const mixedNode = after.find((node) => labelText(node).includes("CleanA.cs"));

      assert.equal(treeRefreshes, 1, "dirty pruning must refresh the tree once");
      assert.equal(after.length, PAIR_COUNT, "dirty-only and singleton clusters must disappear from top offenders");
      assert.doesNotMatch(labels.join("\n"), /Dirty\.cs/, "stale dirty-file offsets must be hidden");
      assert.doesNotMatch(labels.join("\n"), /Clean\.cs/, "one-copy mixed cluster must be hidden");
      assert.ok(mixedNode, "mixed cluster must remain via its clean peer occurrences");
      // Inverted deliberately. This row used to read `rank #1` because the
      // tree numbered its own array, so hiding two stale rows promoted the
      // third cluster to "the repository's worst" — a figure the engine
      
      // [PRINCIPLES-ONE-CALCULATION]); the dirty projection hides rows, it
      // does not re-rank the repository.
      assert.match(
        String(mixedNode.description ?? ""),
        /\brank\s+#3\b/,
        "the survivor keeps the global rank the engine gave it — pruning never renumbers",
      );
      assert.equal(provider.getChildren(mixedNode).length, PAIR_COUNT, "only clean peer occurrences remain expandable");
    } finally {
      sub.dispose();
      provider.dispose();
      ticker.dispose();
    }
  });

  test("surfaces a failed lifecycle as an error status row", () => {
    const store = new ReportStore();
    store.setLifecycle({ kind: "failed", message: "crash" });
    const provider = topOffenders(store);
    const nodes = provider.getChildren();
    const errorNode = nodes.find(
      (n) => typeof n.contextValue === "string" && n.contextValue === "deslop.status.error",
    );
    assert.ok(errorNode, "top offenders must show a failed-lifecycle banner");
    assert.match(labelText(errorNode), /Stopped: crash/);
  });

  test("retains existing clusters during re-analysis — stale > blank ([VSIX-REACTIVITY-TREE])", () => {
    const store = storeWith(
      report([
        cluster(FIRST_CLUSTER_ID, HIGHEST_CLUSTER_MASS, REPO_A_PATH),
        cluster(SECOND_CLUSTER_ID, HIGH_CLUSTER_MASS, REPO_B_PATH),
      ]),
    );
    store.setLifecycle({ kind: ANALYSING_PHASE });
    const provider = topOffenders(store);
    const nodes = provider.getChildren();
    assert.ok(nodes.length >= MIN_VISIBLE_NODE_COUNT, "cluster rows must remain visible during re-analysis");
    const labels = nodes.map(labelText);
    assert.ok(labels.some((l) => /A\.cs/i.test(l) || /c1/i.test(l)), "A.cs cluster must stay visible");
    assert.ok(labels.some((l) => /B\.cs/i.test(l) || /c2/i.test(l)), "B.cs cluster must stay visible");
  });

  // [VSIX-TOP-OFFENDERS-TOOLBAR] Expand All / Collapse All rewrite the collapsible
  // state the provider returns and release on the next data change.
  test("Expand All / Collapse All set the collapsible state the provider returns; released on data change", () => {
    const store = storeWith(
      report([cluster("a", HIGHEST_CLUSTER_MASS, REPO_A_PATH), cluster("b", HIGH_CLUSTER_MASS, REPO_B_PATH)]),
      1,
    );
    const provider = topOffenders(store);
    let fires = 0;
    const sub = provider.onDidChangeTreeData(() => {
      fires += 1;
    });
    try {
      const naturalFirst = provider.getChildren()[0] as vscode.TreeItem;
      assert.equal(
        provider.getTreeItem(naturalFirst).collapsibleState,
        vscode.TreeItemCollapsibleState.Collapsed,
        "clusters default to collapsed",
      );

      provider.setBulkExpansion("expand");
      assert.ok(fires >= 1, "Expand All refreshes the tree");
      assert.equal(
        provider.getTreeItem(provider.getChildren()[0] as vscode.TreeItem).collapsibleState,
        vscode.TreeItemCollapsibleState.Expanded,
        "Expand All forces every expandable row open",
      );

      provider.setBulkExpansion("collapse");
      assert.equal(
        provider.getTreeItem(provider.getChildren()[0] as vscode.TreeItem).collapsibleState,
        vscode.TreeItemCollapsibleState.Collapsed,
        "Collapse All forces every expandable row closed",
      );

      provider.setBulkExpansion("expand");
      store.setSnapshot(
        report([cluster("a", HIGHEST_CLUSTER_MASS, REPO_A_PATH), cluster("b", HIGH_CLUSTER_MASS, REPO_B_PATH)]),
        SECOND_GENERATION,
      );
      assert.equal(
        provider.getTreeItem(provider.getChildren()[0] as vscode.TreeItem).collapsibleState,
        vscode.TreeItemCollapsibleState.Collapsed,
        "a fresh report releases the bulk override back to the natural collapsed state",
      );
    } finally {
      sub.dispose();
    }
  });
});
