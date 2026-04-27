// Unit: TopOffendersProvider. Drives getChildren() against a seeded
// store. Spec coverage:
//   [VSIX-TOP-OFFENDERS-GROUPING]
//   [VSIX-TOP-OFFENDERS-CLUSTER-MODE]
//   [VSIX-TOP-OFFENDERS-FILE-MODE]
//   [VSIX-TOP-OFFENDERS-RANK-GLOBAL]

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";
import {
  FileNode,
  TopOffendersProvider,
  StatusTicker,
} from "../../tree/providers";
import { openOccurrence } from "../../commands/register";
import { ReportStore } from "../../reportStore";
import { ReportOccurrence } from "../../types/report";
import {
  cluster,
  iconColorId,
  labelText,
  report,
  tooltipText,
  withGroupBy,
} from "./tree.helpers";
import { CATEGORY_STYLE } from "../../tree/nodes";

suite("TopOffendersProvider", () => {
  test("renders an Analysing… placeholder before the first report arrives", () => {
    const store = new ReportStore();
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  test("renders a 'no duplication' placeholder when the report is empty", () => {
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  test("cluster mode (default) lists clusters worst-first with global ranks", () => {
    // [VSIX-TOP-OFFENDERS-CLUSTER-MODE] No file-keyed reordering.
    // [VSIX-TOP-OFFENDERS-RANK-GLOBAL] #N is the report's worst-first rank.
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("rank-1-beta", 100, "/repo/src/b/Beta.cs"),
        cluster("rank-2-alpha", 80, "/repo/src/a/Alpha.cs"),
        cluster("rank-3-alpha", 60, "/repo/src/a/Alpha.cs"),
        cluster("rank-4-gamma", 40, "/repo/src/c/Gamma.cs"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const nodes = provider.getChildren();
    const labels = nodes.map(labelText);
    const descriptions = nodes.map((node) => String(node.description ?? ""));

    assert.equal(nodes.length, 4, "one top-level row must render per cluster");
    assert.match(labels[0] ?? "", /#1\b/, "Beta keeps global rank #1 at the top");
    assert.match(labels[1] ?? "", /#2\b/, "Alpha's first cluster keeps global rank #2");
    assert.match(labels[2] ?? "", /#3\b/, "Alpha's second cluster keeps global rank #3");
    assert.match(labels[3] ?? "", /#4\b/, "Gamma keeps global rank #4");
    assert.match(labels[0] ?? "", /Beta\.cs/, "row label must show the file");
    assert.match(labels[1] ?? "", /Alpha\.cs/);
    assert.match(labels[2] ?? "", /Alpha\.cs/);
    assert.match(labels[3] ?? "", /Gamma\.cs/);
    assert.deepEqual(descriptions, [
      "rank-1-beta",
      "rank-2-alpha",
      "rank-3-alpha",
      "rank-4-gamma",
    ]);
    const first = nodes[0];
    assert.ok(first, "first row must exist");
    assert.equal(first.command?.command, "deslop.openCluster");
    assert.deepEqual(first.command?.arguments, ["rank-1-beta"]);
    assert.equal(provider.getChildren(first).length, 2);
  });

  test("file mode roots are FileNodes sorted by max cluster weight desc", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE]
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("rank-1-beta", 100, "/repo/src/b/Beta.cs"),
        cluster("rank-2-alpha", 80, "/repo/src/a/Alpha.cs"),
        cluster("rank-3-alpha", 60, "/repo/src/a/Alpha.cs"),
        cluster("rank-4-gamma", 40, "/repo/src/c/Gamma.cs"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    await withGroupBy("file", () => {
      const nodes = provider.getChildren();
      assert.equal(nodes.length, 3, "three distinct files should produce three roots");
      const roots = nodes.filter((n): n is FileNode => n instanceof FileNode);
      assert.equal(roots.length, 3, "every root in file mode must be a FileNode");
      const filenames = roots.map((root) => labelText(root));
      assert.match(filenames[0] ?? "", /Beta\.cs/, "Beta's max weight 100 wins");
      // Alpha aggregates 80+60=140 (sum) but max 80 < Beta's 100.
      assert.match(filenames[1] ?? "", /Alpha\.cs/);
      assert.match(filenames[2] ?? "", /Gamma\.cs/);
      // Cluster count noun reflects multiplicity.
      assert.match(filenames[0] ?? "", /1 cluster/);
      assert.match(filenames[1] ?? "", /2 clusters/);
    });
  });

  test("file mode tiebreaks equal-max files by sum-of-weights then path localeCompare", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE] Both files share max weight 50; the
    // file with the higher sum (50+50 = 100) wins over the one with sum 50.
    // The third file ties on max + sum and is ordered by localeCompare.
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("a-only", 50, "/repo/AlphaOnly.cs"),
        cluster("b-1", 50, "/repo/BetaPair.cs"),
        cluster("b-2", 50, "/repo/BetaPair.cs"),
        cluster("c-only", 50, "/repo/CharlieOnly.cs"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    await withGroupBy("file", () => {
      const labels = provider.getChildren().map(labelText);
      assert.match(labels[0] ?? "", /BetaPair\.cs/, "higher sum wins the max-tie");
      assert.match(labels[1] ?? "", /AlphaOnly\.cs/, "Alpha precedes Charlie by localeCompare on a sum tie");
      assert.match(labels[2] ?? "", /CharlieOnly\.cs/);
    });
  });

  test("the same cluster keeps the same global #N rank across cluster mode and file mode", async () => {
    // [VSIX-TOP-OFFENDERS-RANK-GLOBAL] Cross-mode comparability.
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("worst", 100, "/repo/A.cs"),
        cluster("middle", 80, "/repo/B.cs"),
        cluster("least", 60, "/repo/A.cs"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const clusterModeLabel = labelText(provider.getChildren()[2] as vscode.TreeItem);
    assert.match(clusterModeLabel, /^#3\b/, "third row in cluster mode is rank #3 (least)");

    await withGroupBy("file", () => {
      const fileRoots = provider.getChildren();
      // A.cs (max 100) sits before B.cs (max 80). Inside A.cs there's
      // one bucket group with two clusters: ranks #1 and #3.
      const aFile = fileRoots[0];
      assert.ok(aFile);
      const [aBucket] = provider.getChildren(aFile);
      assert.ok(aBucket);
      const aClusters = provider.getChildren(aBucket).map(labelText);
      assert.match(aClusters[0] ?? "", /^#1\b/, "weight-100 cluster keeps global rank #1 in file mode");
      assert.match(aClusters[1] ?? "", /^#3\b/, "weight-60 cluster keeps global rank #3 — never re-numbered within the file");
    });
  });

  test("an unknown topOffenders.groupBy value falls back to cluster mode", async () => {
    // [VSIX-TOP-OFFENDERS-GROUPING] Defensive read; never panics.
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c", 100, "/repo/Mixed.cs")]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const cfg = vscode.workspace.getConfiguration("deslop");
    const previous = cfg.get<string>("topOffenders.groupBy", "cluster");
    try {
      // Inspect — does the schema accept an unknown value? VS Code tolerates
      // out-of-enum writes via the API so the runtime fallback can be
      // exercised. If the write rejects, skip the assertion silently.
      try {
        await cfg.update("topOffenders.groupBy", "weird-value", vscode.ConfigurationTarget.Global);
      } catch {
        return;
      }
      const [first] = provider.getChildren();
      assert.ok(first, "fallback render must produce a cluster row");
      assert.doesNotMatch(
        labelText(first),
        /1 cluster\b/,
        "fallback must not enter file mode (which would render a FileNode whose label contains '1 cluster')",
      );
    } finally {
      await cfg.update(
        "topOffenders.groupBy",
        previous === "file" ? "file" : undefined,
        vscode.ConfigurationTarget.Global,
      );
    }
  });

  test("file mode children are bucket groups; only buckets present appear", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE]
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("c1", 100, "/repo/Mixed.cs", 0, 20, "identical"),
        cluster("c2", 80, "/repo/Mixed.cs", 0, 20, "nearly_identical"),
        cluster("c3", 60, "/repo/Mixed.cs", 0, 20, "identical"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    await withGroupBy("file", () => {
      const [fileRoot] = provider.getChildren();
      assert.ok(fileRoot instanceof FileNode, "single root must be a FileNode");
      const groups = provider.getChildren(fileRoot);
      assert.equal(groups.length, 2, "only Identical and Nearly identical groups present");
      const groupLabels = groups.map(labelText);
      assert.match(groupLabels[0] ?? "", /Identical code \(2\)/, "Identical group has 2 clusters and the higher max weight (100)");
      assert.match(groupLabels[1] ?? "", /Nearly identical code \(1\)/);
    });
  });

  test("file mode clusters under a bucket are sorted by weight desc and drop the file suffix", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE]
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("hi", 100, "/repo/Mixed.cs", 0, 20, "identical"),
        cluster("lo", 60, "/repo/Mixed.cs", 0, 20, "identical"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    await withGroupBy("file", () => {
      const [fileRoot] = provider.getChildren();
      assert.ok(fileRoot, "file root must exist");
      const [bucketGroup] = provider.getChildren(fileRoot);
      assert.ok(bucketGroup, "bucket group must exist");
      const clusterNodes = provider.getChildren(bucketGroup);
      assert.equal(clusterNodes.length, 2);
      const labels = clusterNodes.map(labelText);
      assert.match(labels[0] ?? "", /^#1\b/, "weight-100 cluster comes first and keeps global rank #1");
      assert.match(labels[1] ?? "", /^#2\b/, "weight-60 cluster keeps global rank #2");
      assert.doesNotMatch(labels[0] ?? "", /Mixed\.cs/, "file suffix is dropped under a parent file");
      assert.doesNotMatch(labels[1] ?? "", /Mixed\.cs/);
    });
  });

  test("tooltip preserves the full file path in both grouping modes", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE] Tooltip is mode-invariant.
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster("only", 100, "/repo/src/Mixed.cs", 0, 20, "identical")]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const [clusterMode] = provider.getChildren();
    assert.ok(clusterMode);
    assert.match(tooltipText(clusterMode), /\/repo\/src\/Mixed\.cs/);

    await withGroupBy("file", () => {
      const [fileRoot] = provider.getChildren();
      assert.ok(fileRoot);
      const [bucketGroup] = provider.getChildren(fileRoot);
      assert.ok(bucketGroup);
      const [fileModeCluster] = provider.getChildren(bucketGroup);
      assert.ok(fileModeCluster);
      assert.match(tooltipText(fileModeCluster), /\/repo\/src\/Mixed\.cs/);
    });
  });

  test("file mode occurrence leaves match cluster mode byte-for-byte", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE] No special-case rendering for leaves.
    const store = new ReportStore();
    const c = cluster("only", 100, "/repo/src/Mixed.cs", 7, 14, "identical");
    store.setSnapshot(report([c]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const [clusterRoot] = provider.getChildren();
    assert.ok(clusterRoot);
    const clusterModeOccurrences = provider.getChildren(clusterRoot);

    await withGroupBy("file", () => {
      const [fileRoot] = provider.getChildren();
      assert.ok(fileRoot);
      const [bucketGroup] = provider.getChildren(fileRoot);
      assert.ok(bucketGroup);
      const [fileClusterRoot] = provider.getChildren(bucketGroup);
      assert.ok(fileClusterRoot);
      const fileModeOccurrences = provider.getChildren(fileClusterRoot);
      assert.equal(fileModeOccurrences.length, clusterModeOccurrences.length);
      assert.equal(
        labelText(fileModeOccurrences[0] as vscode.TreeItem),
        labelText(clusterModeOccurrences[0] as vscode.TreeItem),
      );
      assert.equal(
        String(fileModeOccurrences[0]?.description ?? ""),
        String(clusterModeOccurrences[0]?.description ?? ""),
      );
      assert.equal(
        fileModeOccurrences[0]?.command?.command,
        clusterModeOccurrences[0]?.command?.command,
      );
    });
  });

  test("setting flip refreshes the tree", () => {
    // [VSIX-TOP-OFFENDERS-GROUPING] The provider exposes a refresh()
    // hook the activation bridge calls when the setting changes.
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c", 100, "/repo/Mixed.cs")]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
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

  test("renders distinct accessible category color metadata on Top Offenders rows", () => {
    // [VSIX-TOP-OFFENDERS-CATEGORY-COLORS]
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("exact", 100, "/repo/src/a/Exact.cs", 0, 20, "identical"),
        cluster("near", 90, "/repo/src/b/Near.cs", 0, 20, "nearly_identical"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const [exact, near] = provider.getChildren();
    assert.ok(exact, "exact duplicate row must render");
    assert.ok(near, "near duplicate row must render");
    assert.ok(exact.iconPath instanceof vscode.ThemeIcon);
    assert.ok(near.iconPath instanceof vscode.ThemeIcon);
    assert.notEqual(iconColorId(exact), "", "exact duplicate must carry a theme color");
    assert.notEqual(iconColorId(near), "", "near duplicate must carry a theme color");
    assert.notEqual(
      iconColorId(exact),
      iconColorId(near),
      "exact and near duplicate categories must have distinct theme colors",
    );
    assert.match(labelText(exact), /Identical code/);
    assert.match(labelText(near), /Nearly identical code/);
    assert.match(labelText(exact), /Exact\.cs/);
    assert.match(labelText(near), /Near\.cs/);
    assert.match(exact.accessibilityInformation?.label ?? "", /Identical code/);
    assert.match(near.accessibilityInformation?.label ?? "", /Nearly identical code/);
    assert.match(exact.accessibilityInformation?.label ?? "", /Exact\.cs/);
    assert.match(near.accessibilityInformation?.label ?? "", /Near\.cs/);
    assert.match(tooltipText(exact), /\/repo\/src\/a\/Exact\.cs/);
    assert.match(tooltipText(near), /\/repo\/src\/b\/Near\.cs/);
  });

  test("identical cluster icon is red not green — green implies safe, but identical code is worst severity", () => {
    // [VSIX-TOP-OFFENDERS-CATEGORY-COLORS] Identical = error level, must not use green.
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster("clone", 100, "/repo/src/Clone.cs", 0, 20, "identical")]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const [node] = provider.getChildren();
    assert.ok(node, "identical cluster must render a node");
    assert.equal(
      iconColorId(node),
      "charts.red",
      "identical clones are the highest severity — icon must be red, not green",
    );
  });

  test("no category style uses charts.green — green is never correct for code duplication", () => {
    // [VSIX-TOP-OFFENDERS-CATEGORY-COLORS] Green implies safety/good; duplicates are never good.
    for (const [bucket, style] of Object.entries(CATEGORY_STYLE)) {
      assert.notEqual(
        style.color,
        "charts.green",
        `${bucket} must not use charts.green — green implies the code is in good shape`,
      );
    }
  });

  test("expanding a cluster node yields OccurrenceNode children", () => {
    const store = new ReportStore();
    const c = cluster("a", 10, "/f1");
    store.setSnapshot(report([c]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const roots = provider.getChildren();
    const kids = provider.getChildren(roots[0]);
    assert.equal(kids.length, c.occurrences.length);
  });

  test("occurrence row reports and opens the exact file, line, and column", async () => {
    // [VSIX-ACTIVITY-BAR] Issue #8: tree occurrence rows must show
    // path:line:column, not machine-oriented start_byte..end_byte.
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "deslop-issue-8-tree-"));
    const occurrencePath = path.join(dir, "ChatProtocol.cs");
    const source = "namespace Demo;\n\npublic sealed class ChatProtocol {\n    void Send() {}\n}\n";
    const startByte = Buffer.byteLength(source.slice(0, source.indexOf("void Send")), "utf8");
    const endByte = startByte + Buffer.byteLength("void Send", "utf8");
    fs.writeFileSync(occurrencePath, source, "utf8");

    try {
      const store = new ReportStore();
      store.setSnapshot(report([cluster("issue-8", 10, occurrencePath, startByte, endByte)]), 0);
      const provider = new TopOffendersProvider(store, new StatusTicker());
      const [root] = provider.getChildren();
      assert.ok(root, "cluster root must exist");

      const [occurrence] = provider.getChildren(root);
      assert.ok(occurrence, "occurrence child must exist");
      const label = typeof occurrence.label === "string"
        ? occurrence.label
        : occurrence.label?.label ?? "";
      const description = String(occurrence.description ?? "");
      const rendered = `${label} ${description}`;

      assert.ok(occurrence.command, "occurrence row must be tappable");
      assert.equal(occurrence.command.command, "deslop.openOccurrence");
      const commandArguments = occurrence.command.arguments;
      assert.ok(commandArguments, "occurrence command must carry arguments");
      const argument = commandArguments[0] as ReportOccurrence | undefined;
      assert.ok(argument, "occurrence command must carry the occurrence payload");

      await openOccurrence(argument);

      const editor = vscode.window.activeTextEditor;
      assert.ok(editor, "tapping the occurrence must open an editor");
      assert.equal(editor.document.uri.fsPath, occurrencePath);
      assert.equal(editor.selection.start.line, 3, "cursor should move to line 4");
      assert.equal(editor.selection.start.character, 4, "cursor should move to column 5");
      assert.equal(editor.selection.end.character, 13, "selection should cover the occurrence");

      assert.deepEqual(
        {
          hasFileName: /ChatProtocol\.cs/.test(rendered),
          hasLineAndColumn: /ChatProtocol\.cs:4:5/.test(rendered) ||
            /line\s+4,\s*column\s+5/i.test(rendered),
          exposesRawByteRange: new RegExp(`\\b${startByte}\\.\\.${endByte}\\b`).test(rendered),
          usesByteTerminology: /\bbytes?\b/i.test(rendered),
        },
        {
          hasFileName: true,
          hasLineAndColumn: true,
          exposesRawByteRange: false,
          usesByteTerminology: false,
        },
        `occurrence row must report the same human target it navigates to, got: ${rendered}`,
      );
    } finally {
      await vscode.commands.executeCommand("workbench.action.closeActiveEditor");
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  test("getTreeItem returns the node verbatim", () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("a", 10, "/f1")]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
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
      assert.equal(String(provider.getChildren()[0]?.description ?? ""), "stale");

      store.applyDelta({
        from_generation: 1,
        to_generation: 2,
        clusters_added: [cluster("fresh", 50, "/fresh.cs")],
        clusters_removed: ["stale"],
        clusters_updated: [],
        cache_stats: { hits: 2, misses: 0 },
        tool_version: "v2",
      });

      assert.equal(treeRefreshes, 2, "delta must refresh the tree");
      assert.equal(String(provider.getChildren()[0]?.description ?? ""), "fresh");
    } finally {
      sub.dispose();
      provider.dispose();
      ticker.dispose();
    }
  });

  test("surfaces a failed lifecycle as an error status row", () => {
    const store = new ReportStore();
    store.setLifecycle({ kind: "failed", message: "crash" });
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    const errorNode = nodes.find(
      (n) => typeof n.contextValue === "string" && n.contextValue === "deslop.status.error",
    );
    assert.ok(errorNode, "top offenders must show a failed-lifecycle banner");
    assert.match(labelText(errorNode), /Stopped: crash/);
  });
});
