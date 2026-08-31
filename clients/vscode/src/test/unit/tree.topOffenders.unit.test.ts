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
  FolderNode,
  TopOffendersProvider,
  StatusTicker,
} from "../../tree/providers";
import { openOccurrence } from "../../commands/register";
import { ReportStore } from "../../reportStore";
import { ReportCluster, ReportOccurrence } from "../../types/report";
import {
  cluster,
  iconColorId,
  labelText,
  metrics,
  report,
  tooltipText,
  withGroupBy,
  withSetting,
} from "./tree.helpers";
import { bandOf } from "../cluster.helpers";
import { SEVERITY_STYLE } from "../../tree/nodes";

const DEFAULT_OCCURRENCE_END_BYTE = 20;
const HIGHEST_CLUSTER_MASS = 100;
const HIGH_CLUSTER_MASS = 80;
const MEDIUM_CLUSTER_MASS = 60;
const TIED_CLUSTER_MASS = 50;
const FILE_GROUPING_MODE = "file";
const MISSING_LABEL = "<missing>";
const MIXED_FILE_PATH = "/repo/Mixed.cs";
const REPO_A_PATH = "/repo/A.cs";
const REPO_B_PATH = "/repo/B.cs";
const ALPHA_FILE_PATH = "/repo/src/a/Alpha.cs";
const SORT_BY_SETTING = "topOffenders.sortBy";
const PATH_SORT_MODE = "path";
const ANALYSING_PHASE = "analysing";
const FIRST_CLUSTER_ID = "c1";
const SECOND_CLUSTER_ID = "c2";
const BETA_FILE_PATH = "/repo/src/b/Beta.cs";
const FIRST_FIXTURE_PATH = "/f1";
const CLUSTER_ROOT_REQUIRED = "cluster root must exist";
const CANONICAL_OCCURRENCE_CONTEXT = "deslop.occurrenceCanonical";
const HEAVY_CLUSTER_ID = "heavy";
const HEAVY_FILE_PATH = "/repo/z.cs";
const LIGHT_CLUSTER_ID = "light";
const LIGHT_FILE_PATH = "/repo/a.cs";
const TEST_TWO = 2;
const TEST_THREE = 3;
const THIRD_ITEM_INDEX = TEST_TWO;
const FOURTH_ITEM_INDEX = TEST_THREE;
const PAIR_COUNT = TEST_TWO;
const FILE_ROOT_COUNT = TEST_THREE;
const ZERO_BASED_FOURTH_LINE = TEST_THREE;
const SECOND_GENERATION = TEST_TWO;
const CACHE_HIT_COUNT = TEST_TWO;
const EXPECTED_TREE_REFRESH_COUNT = TEST_TWO;
const MIN_VISIBLE_NODE_COUNT = TEST_TWO;
const TEST_TEN = 10;
const LOW_CLUSTER_WEIGHT = TEST_TEN;
const DIRTY_OCCURRENCE_START_BYTE = TEST_TEN;
const DIRTY_FILE_PATH = "/repo/Dirty.cs";

function reportOccurrence(
  occurrencePath: string,
  startByte = 0,
  endByte = DEFAULT_OCCURRENCE_END_BYTE,
): ReportOccurrence {
  return { path: occurrencePath, start_byte: startByte, end_byte: endByte, start_line: 1, end_line: 2, hidden: false };
}

// Re-shapes a fixture cluster around an explicit occurrence list. Every
// count moves together — `occurrence_count` is the number every surface
// displays, so a fixture that changed the list and left the engine's
// count behind would describe a cluster the engine could never publish.
function withOccurrences(
  base: ReportCluster,
  occurrences: ReportOccurrence[],
): ReportCluster {
  return {
    ...base,
    canonical_node_count: occurrences.length,
    occurrences_total: occurrences.length,
    occurrence_count: occurrences.length,
    occurrences,
  };
}

suite("TopOffendersProvider", () => {
  test("renders an Analysing… placeholder before the first report arrives", () => {
    const store = new ReportStore();
    const provider = new TopOffendersProvider(store, new StatusTicker());
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
    const provider = new TopOffendersProvider(store, new StatusTicker());
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
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    store.setLifecycle({ kind: "ready" });
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const [only] = provider.getChildren();
    assert.ok(only);
    assert.equal(only.contextValue, "deslop.status.info");
    assert.match(labelText(only), /No duplication detected/);
  });

  // [req: incremental indicator] An edit-triggered re-analysis keeps the
  // existing clusters on screen (stale > blank) and leads with a busy
  // badge so the user can see an update is in flight.
  test("leads with an 'Analysing changes…' badge during incremental re-analysis", () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster(FIRST_CLUSTER_ID, HIGHEST_CLUSTER_MASS, REPO_A_PATH), cluster(SECOND_CLUSTER_ID, HIGH_CLUSTER_MASS, REPO_B_PATH)]),
      0,
    );
    store.setLifecycle({ kind: ANALYSING_PHASE });
    const provider = new TopOffendersProvider(store, new StatusTicker());
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

  test("cluster mode (default) lists clusters worst-first with global ranks", () => {
    // [VSIX-TOP-OFFENDERS-CLUSTER-MODE] No file-keyed reordering.
    // [VSIX-TOP-OFFENDERS-RANK-GLOBAL] rank #N lives in the grey description;
    // [VSIX-TOP-OFFENDERS-CLUSTER-ID] the stable short id leads the bold label.
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("1111aaaabbbbcccc", HIGHEST_CLUSTER_MASS, BETA_FILE_PATH),
        cluster("2222aaaabbbbcccc", HIGH_CLUSTER_MASS, ALPHA_FILE_PATH),
        cluster("3333aaaabbbbcccc", MEDIUM_CLUSTER_MASS, ALPHA_FILE_PATH),
        cluster("4444aaaabbbbcccc", 40, "/repo/src/c/Gamma.cs"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const nodes = provider.getChildren();
    const labels = nodes.map(labelText);
    const descriptions = nodes.map((node) => String(node.description ?? ""));

    assert.equal(nodes.length, 4, "one top-level row must render per cluster");
    assert.ok(labels[0]?.startsWith("1111aaa "), `worst row leads with its slug, got: ${labels[0] ?? MISSING_LABEL}`);
    assert.ok(labels[1]?.startsWith("2222aaa "), `row 2 leads with its slug, got: ${labels[1] ?? MISSING_LABEL}`);
    assert.ok(labels[THIRD_ITEM_INDEX]?.startsWith("3333aaa "), `row 3 leads with its slug, got: ${labels[THIRD_ITEM_INDEX] ?? MISSING_LABEL}`);
    assert.ok(labels[FOURTH_ITEM_INDEX]?.startsWith("4444aaa "), `row 4 leads with its slug, got: ${labels[FOURTH_ITEM_INDEX] ?? MISSING_LABEL}`);
    assert.match(labels[0] ?? "", /Beta\.cs/, "row label must show the file");
    assert.match(labels[1] ?? "", /Alpha\.cs/);
    assert.match(labels[THIRD_ITEM_INDEX] ?? "", /Alpha\.cs/);
    assert.match(labels[FOURTH_ITEM_INDEX] ?? "", /Gamma\.cs/);
    assert.match(descriptions[0] ?? "", /\brank\s+#1\b/, "row 1 carries rank #1 in its description");
    assert.match(descriptions[1] ?? "", /\brank\s+#2\b/, "row 2 carries rank #2 in its description");
    assert.match(descriptions[THIRD_ITEM_INDEX] ?? "", /\brank\s+#3\b/, "row 3 carries rank #3 in its description");
    assert.match(descriptions[FOURTH_ITEM_INDEX] ?? "", /\brank\s+#4\b/, "row 4 carries rank #4 in its description");
    assert.ok(
      descriptions.every((d) => /\b\d+ copies\b/.test(d)),
      `cluster descriptions must keep the copy count; got: ${JSON.stringify(descriptions)}`,
    );
    const first = nodes[0];
    assert.ok(first, "first row must exist");
    assert.equal(first.command?.command, "deslop.openCluster");
    assert.deepEqual(
      first.command?.arguments,
      ["1111aaaabbbbcccc"],
      "command argument keeps the full 16-hex id; only the display is shortened",
    );
    assert.equal(provider.getChildren(first).length, PAIR_COUNT);
  });

  test("cluster row label leads with the stable slug, not the volatile #N rank", () => {
    // [VSIX-TOP-OFFENDERS-RANK-GLOBAL] / [VSIX-TOP-OFFENDERS-CLUSTER-MODE]
    // [VSIX-TOP-OFFENDERS-CLUSTER-ID] The stable cluster identifier is the
    // 16-hex hash; the rank #N is a volatile array-index that flips on every
    // snapshot. Putting #N in the bold label makes humans (and AI agents
    // reading the rendered tree) treat the rank as the row's identity.
    // Cluster slug leads, rank moves to the grey description with the
    // literal word "rank". Slug length is shared with the hover bubble
    // (see clusterHover.ts::clusterSlug).
    const store = new ReportStore();
    const clusterId = "1802186da488862f";
    store.setSnapshot(
      report([
        cluster(clusterId, HIGHEST_CLUSTER_MASS, "/repo/src/Worst.cs"),
        cluster("c0ffee1234567890", HIGH_CLUSTER_MASS, "/repo/src/Next.cs"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const [first, second] = provider.getChildren();
    assert.ok(first, "first cluster row must render");
    assert.ok(second, "second cluster row must render");

    const firstLabel = labelText(first);
    const firstDescription = String(first.description ?? "");
    const firstTooltip = tooltipText(first);
    const firstA11y = first.accessibilityInformation?.label ?? "";

    assert.ok(
      firstLabel.startsWith("1802186 "),
      `label must lead with the stable slug (first 7 hex chars), got: ${firstLabel}`,
    );
    assert.doesNotMatch(
      firstLabel,
      /^#\d/,
      `label must not lead with the volatile #N rank, got: ${firstLabel}`,
    );
    assert.doesNotMatch(
      firstLabel,
      /#1\b/,
      `rank #N must not appear in the bold label at all, got: ${firstLabel}`,
    );
    assert.match(
      firstDescription,
      /\brank\s+#1\b/,
      `description must spell out the word "rank" so AI consumers can't confuse it for an id, got: ${firstDescription}`,
    );
    assert.match(
      firstDescription,
      /\b2 copies\b/,
      `description must keep the copy count, got: ${firstDescription}`,
    );
    assert.match(
      firstTooltip,
      /\brank\s+#1\b/,
      `tooltip must use the word "rank", got: ${firstTooltip}`,
    );
    assert.match(
      firstA11y,
      /\brank\s+#?1\b/,
      `accessibility label must spell out "rank", got: ${firstA11y}`,
    );
    assert.match(
      firstTooltip,
      /cluster id:\s+`1802186da488862f`/,
      "tooltip must still expose the full 16-hex id for AI/cross-reference",
    );

    const secondLabel = labelText(second);
    const secondDescription = String(second.description ?? "");
    assert.ok(
      secondLabel.startsWith("c0ffee1 "),
      `second row's label must also lead with its own slug, got: ${secondLabel}`,
    );
    assert.match(
      secondDescription,
      /\brank\s+#2\b/,
      `second row's description must carry "rank #2", got: ${secondDescription}`,
    );

    assert.equal(
      first.command?.command,
      "deslop.openCluster",
      "row still navigates to the cluster",
    );
    assert.deepEqual(
      first.command?.arguments,
      [clusterId],
      "command argument keeps the full 16-hex id; display truncation is presentation-only",
    );
  });

  test("issue_47_cluster_tooltip_keeps_labeled_cluster_id_after_human_description", () => {
    const store = new ReportStore();
    const clusterId = "1802186da488862f";
    store.setSnapshot(
      report([cluster(clusterId, 48_936.95, "/repo/src/ICD10/CliE2ETests.cs")]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const [node] = provider.getChildren();
    assert.ok(node, "cluster row must render");
    assert.notEqual(
      String(node.description ?? ""),
      clusterId,
      "row description must not use the hex cluster id as the human anchor",
    );
    assert.match(
      tooltipText(node),
      /cluster id:\s+`1802186da488862f`/,
      "tooltip must keep the machine id discoverable behind a labeled cluster id field",
    );
  });

  test("file mode roots are FileNodes sorted by max cluster weight desc", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE]
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("rank-1-beta", HIGHEST_CLUSTER_MASS, BETA_FILE_PATH),
        cluster("rank-2-alpha", HIGH_CLUSTER_MASS, ALPHA_FILE_PATH),
        cluster("rank-3-alpha", MEDIUM_CLUSTER_MASS, ALPHA_FILE_PATH),
        cluster("rank-4-gamma", 40, "/repo/src/c/Gamma.cs"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    await withGroupBy(FILE_GROUPING_MODE, () => {
      const nodes = provider.getChildren();
      assert.equal(nodes.length, FILE_ROOT_COUNT, "three distinct files should produce three roots");
      const roots = nodes.filter((n): n is FileNode => n instanceof FileNode);
      assert.equal(roots.length, FILE_ROOT_COUNT, "every root in file mode must be a FileNode");
      const filenames = roots.map((root) => labelText(root));
      assert.match(filenames[0] ?? "", /Beta\.cs/, "Beta's max weight 100 wins");
      // Alpha aggregates 80+60=140 (sum) but max 80 < Beta's 100.
      assert.match(filenames[1] ?? "", /Alpha\.cs/);
      assert.match(filenames[THIRD_ITEM_INDEX] ?? "", /Gamma\.cs/);
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
        cluster("a-only", TIED_CLUSTER_MASS, "/repo/AlphaOnly.cs"),
        cluster("b-1", TIED_CLUSTER_MASS, "/repo/BetaPair.cs"),
        cluster("b-2", TIED_CLUSTER_MASS, "/repo/BetaPair.cs"),
        cluster("c-only", TIED_CLUSTER_MASS, "/repo/CharlieOnly.cs"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    await withGroupBy(FILE_GROUPING_MODE, () => {
      const labels = provider.getChildren().map(labelText);
      assert.match(labels[0] ?? "", /BetaPair\.cs/, "higher sum wins the max-tie");
      assert.match(labels[1] ?? "", /AlphaOnly\.cs/, "Alpha precedes Charlie by localeCompare on a sum tie");
      assert.match(labels[THIRD_ITEM_INDEX] ?? "", /CharlieOnly\.cs/);
    });
  });

  test("the same cluster keeps the same global rank across cluster mode and file mode", async () => {
    // [VSIX-TOP-OFFENDERS-RANK-GLOBAL] Cross-mode comparability. Rank lives
    // in the grey description so the bold label is free for the stable id.
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("worst", HIGHEST_CLUSTER_MASS, REPO_A_PATH),
        cluster("middle", HIGH_CLUSTER_MASS, REPO_B_PATH),
        cluster("least", MEDIUM_CLUSTER_MASS, REPO_A_PATH),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const thirdRow = provider.getChildren()[2] as vscode.TreeItem;
    assert.match(
      String(thirdRow.description ?? ""),
      /\brank\s+#3\b/,
      "third row in cluster mode is rank #3 (least)",
    );

    await withGroupBy(FILE_GROUPING_MODE, () => {
      const fileRoots = provider.getChildren();
      // A.cs (max mass 100) sits before B.cs (max mass 80). A.cs expands
      // to severity groups whose clusters keep their global ranks: #1 and #3.
      const aFile = fileRoots[0];
      assert.ok(aFile);
      const severityGroups = provider.getChildren(aFile);
      const aClusters = severityGroups.flatMap((group) => provider.getChildren(group));
      assert.ok(aClusters.length >= 2, "A.cs must expose both of its clusters under severity groups");
      assert.match(
        String(aClusters[0]?.description ?? ""),
        /\brank\s+#1\b/,
        "mass-100 cluster keeps global rank #1 in file mode",
      );
      const descriptions = aClusters.map((node) => String(node.description ?? ""));
      assert.ok(
        descriptions.some((description) => /\brank\s+#3\b/.test(description)),
        `mass-60 cluster keeps global rank #3 — never re-numbered within the file, got: ${descriptions.join(" | ")}`,
      );
    });
  });

  test("an unknown topOffenders.groupBy value falls back to cluster mode", async () => {
    // [VSIX-TOP-OFFENDERS-GROUPING] Defensive read; never panics.
    const store = new ReportStore();
    store.setSnapshot(report([cluster("c", HIGHEST_CLUSTER_MASS, MIXED_FILE_PATH)]), 0);
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
        previous === FILE_GROUPING_MODE ? FILE_GROUPING_MODE : undefined,
        vscode.ConfigurationTarget.Global,
      );
    }
  });

  test("file mode children are severity groups; only bands present appear", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE]
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster(FIRST_CLUSTER_ID, HIGHEST_CLUSTER_MASS, MIXED_FILE_PATH, 0, DEFAULT_OCCURRENCE_END_BYTE, "worst"),
        cluster(SECOND_CLUSTER_ID, HIGH_CLUSTER_MASS, MIXED_FILE_PATH, 0, DEFAULT_OCCURRENCE_END_BYTE, "top10"),
        cluster("c3", MEDIUM_CLUSTER_MASS, MIXED_FILE_PATH, 0, DEFAULT_OCCURRENCE_END_BYTE, "worst"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    await withGroupBy(FILE_GROUPING_MODE, () => {
      const [fileRoot] = provider.getChildren();
      assert.ok(fileRoot instanceof FileNode, "single root must be a FileNode");
      const groups = provider.getChildren(fileRoot);
      // [SEVERITY-BAND] The engine stamps rank + band from the report's
      // worst-first order, so the groups are exactly the distinct stamped
      // bands in band order — one group per present band, never a band
      // no cluster holds.
      const stamped = [FIRST_CLUSTER_ID, SECOND_CLUSTER_ID, "c3"].map((_, index) =>
        bandOf(index + 1, TEST_THREE),
      );
      const presentBands = [...new Set(stamped)];
      const groupLabels = groups.map(labelText);
      assert.equal(groups.length, presentBands.length, "one group per present band");
      for (let index = 0; index < presentBands.length; index += 1) {
        assert.match(
          groupLabels[index] ?? "",
          new RegExp(`Severity ${presentBands[index]}`),
          `group ${index} must be the ${presentBands[index]} band, in stamped band order`,
        );
      }
    });
  });

  test("file mode clusters under a bucket are sorted by weight desc and drop the file suffix", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE]
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("hi", HIGHEST_CLUSTER_MASS, MIXED_FILE_PATH, 0, DEFAULT_OCCURRENCE_END_BYTE, "worst"),
        cluster("lo", MEDIUM_CLUSTER_MASS, MIXED_FILE_PATH, 0, DEFAULT_OCCURRENCE_END_BYTE, "worst"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    await withGroupBy(FILE_GROUPING_MODE, () => {
      const [fileRoot] = provider.getChildren();
      assert.ok(fileRoot, "file root must exist");
      // [SEVERITY-BAND] Stamped bands: rank #1 → worst, rank #2 → faint,
      // so the two clusters sit in two band groups. Flattened, they must
      // still be worst-first and drop the parent file suffix.
      const severityGroups = provider.getChildren(fileRoot);
      const clusterNodes = severityGroups.flatMap((group) => provider.getChildren(group));
      assert.equal(clusterNodes.length, PAIR_COUNT);
      const labels = clusterNodes.map(labelText);
      const descriptions = clusterNodes.map((n) => String(n.description ?? ""));
      assert.match(descriptions[0] ?? "", /\brank\s+#1\b/, "mass-100 cluster comes first and carries rank #1 in its description");
      assert.match(descriptions[1] ?? "", /\brank\s+#2\b/, "mass-60 cluster carries rank #2 in its description");
      assert.doesNotMatch(labels[0] ?? "", /Mixed\.cs/, "file suffix is dropped under a parent file");
      assert.doesNotMatch(labels[1] ?? "", /Mixed\.cs/);
    });
  });

  test("tooltip preserves the full file path in both grouping modes", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE] Tooltip is mode-invariant.
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster("only", HIGHEST_CLUSTER_MASS, "/repo/src/Mixed.cs", 0, DEFAULT_OCCURRENCE_END_BYTE, "worst")]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const [clusterMode] = provider.getChildren();
    assert.ok(clusterMode);
    assert.match(tooltipText(clusterMode), /\/repo\/src\/Mixed\.cs/);

    await withGroupBy(FILE_GROUPING_MODE, () => {
      const [fileRoot] = provider.getChildren();
      assert.ok(fileRoot);
      const [severityGroup] = provider.getChildren(fileRoot);
      assert.ok(severityGroup);
      const [fileModeCluster] = provider.getChildren(severityGroup);
      assert.ok(fileModeCluster);
      assert.match(tooltipText(fileModeCluster), /\/repo\/src\/Mixed\.cs/);
    });
  });

  test("file mode occurrence leaves match cluster mode byte-for-byte", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE] No special-case rendering for leaves.
    const store = new ReportStore();
    const c = cluster("only", 100, "/repo/src/Mixed.cs", 7, 14, "worst");
    store.setSnapshot(report([c]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const [clusterRoot] = provider.getChildren();
    assert.ok(clusterRoot);
    const clusterModeOccurrences = provider.getChildren(clusterRoot);

    await withGroupBy(FILE_GROUPING_MODE, () => {
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
    store.setSnapshot(report([cluster("c", HIGHEST_CLUSTER_MASS, MIXED_FILE_PATH)]), 0);
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

  test("renders distinct accessible severity color metadata on Top Offenders rows", () => {
    // [VSIX-TOP-OFFENDERS-CATEGORY-COLORS] Severity bands drive icon colour.
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("exact", HIGHEST_CLUSTER_MASS, "/repo/src/a/Exact.cs", 0, DEFAULT_OCCURRENCE_END_BYTE, "worst"),
        cluster("near", 90, "/repo/src/b/Near.cs", 0, DEFAULT_OCCURRENCE_END_BYTE, "top10"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const [exact, near] = provider.getChildren();
    assert.ok(exact, "worst-band row must render");
    assert.ok(near, "top10 row must render");
    assert.ok(exact.iconPath instanceof vscode.ThemeIcon);
    assert.ok(near.iconPath instanceof vscode.ThemeIcon);
    assert.notEqual(iconColorId(exact), "", "worst band must carry a theme color");
    assert.notEqual(iconColorId(near), "", "top10 band must carry a theme color");
    assert.notEqual(
      iconColorId(exact),
      iconColorId(near),
      "worst and top10 bands must have distinct theme colors",
    );
    assert.match(labelText(exact), /Duplicate code/);
    assert.match(labelText(near), /Duplicate code/);
    assert.match(labelText(exact), /Exact\.cs/);
    assert.match(labelText(near), /Near\.cs/);
    assert.match(exact.accessibilityInformation?.label ?? "", /Duplicate code/);
    assert.match(near.accessibilityInformation?.label ?? "", /Duplicate code/);
    assert.match(exact.accessibilityInformation?.label ?? "", /Exact\.cs/);
    assert.match(near.accessibilityInformation?.label ?? "", /Near\.cs/);
    assert.match(tooltipText(exact), /\/repo\/src\/a\/Exact\.cs/);
    assert.match(tooltipText(near), /\/repo\/src\/b\/Near\.cs/);
  });

  test("worst-band cluster icon is red not green — green implies safe, but duplicates are worst severity", () => {
    // [VSIX-TOP-OFFENDERS-CATEGORY-COLORS] Worst band = error level, must not use green.
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster("clone", HIGHEST_CLUSTER_MASS, "/repo/src/Clone.cs", 0, DEFAULT_OCCURRENCE_END_BYTE, "worst")]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const [node] = provider.getChildren();
    assert.ok(node, "worst-band cluster must render a node");
    assert.equal(
      iconColorId(node),
      "charts.red",
      "worst-band duplicates are the highest severity — icon must be red, not green",
    );
  });

  test("no severity style uses charts.green — green is never correct for code duplication", () => {
    // [VSIX-TOP-OFFENDERS-CATEGORY-COLORS] Green implies safety/good; duplicates are never good.
    for (const [severity, style] of Object.entries(SEVERITY_STYLE)) {
      assert.notEqual(
        style.color,
        "charts.green",
        `${severity} must not use charts.green — green implies the code is in good shape`,
      );
    }
  });

  test("expanding a cluster node yields OccurrenceNode children", () => {
    const store = new ReportStore();
    const c = cluster("a", LOW_CLUSTER_WEIGHT, "/f1");
    store.setSnapshot(report([c]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const roots = provider.getChildren();
    const kids = provider.getChildren(roots[0]);
    assert.equal(kids.length, c.occurrences.length);
  });

  test("occurrence node tooltip shows parent cluster rank, category, and position (#47)", () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("a", LOW_CLUSTER_WEIGHT, FIRST_FIXTURE_PATH)]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const [root] = provider.getChildren();
    assert.ok(root, CLUSTER_ROOT_REQUIRED);
    const [first, second] = provider.getChildren(root);
    assert.ok(first, "first occurrence node must exist");
    assert.ok(second, "second occurrence node must exist");
    const tip1 = tooltipText(first);
    const tip2 = tooltipText(second);
    assert.match(tip1, /\brank\s+#1\b/, "tooltip must spell out the parent cluster rank");
    assert.match(tip1, /Duplicate code/, "tooltip must name the verdict");
    assert.match(tip1, /occurrence 1 of 2/, "tooltip must show position in cluster");
    assert.match(tip2, /occurrence 2 of 2/, "second occurrence tooltip must reflect its index");
  });

  test("compare with canonical context values hide non-actionable rows (#14)", () => {
    const store = new ReportStore();
    const singleOccurrence = withOccurrences(cluster("single", 5, "/single"), [
      reportOccurrence("/single"),
    ]);
    store.setSnapshot(
      report([cluster("multi", LOW_CLUSTER_WEIGHT, FIRST_FIXTURE_PATH), singleOccurrence]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const [multi, single] = provider.getChildren();
    assert.ok(multi, "multi-occurrence cluster root must exist");
    assert.ok(single, "single-occurrence cluster root must exist");
    assert.equal(multi.contextValue, "deslop.clusterComparable");
    assert.equal(single.contextValue, "deslop.clusterSingle");

    const [canonical, comparable] = provider.getChildren(multi);
    assert.ok(canonical, "canonical occurrence row must exist");
    assert.ok(comparable, "comparable occurrence row must exist");
    assert.equal(canonical.contextValue, CANONICAL_OCCURRENCE_CONTEXT);
    assert.equal(comparable.contextValue, "deslop.occurrence");
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
      store.setSnapshot(report([cluster("issue-8", LOW_CLUSTER_WEIGHT, occurrencePath, startByte, endByte)]), 0);
      const provider = new TopOffendersProvider(store, new StatusTicker());
      const [root] = provider.getChildren();
      assert.ok(root, CLUSTER_ROOT_REQUIRED);

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
      assert.equal(editor.selection.start.line, ZERO_BASED_FOURTH_LINE, "cursor should move to line 4");
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
    store.setSnapshot(report([cluster("a", LOW_CLUSTER_WEIGHT, FIRST_FIXTURE_PATH)]), 0);
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
      assert.match(
        String(provider.getChildren()[0]?.description ?? ""),
        /\brank\s+#1\b.*\b\d+ copies\b/,
        "description must show rank then copy count after snapshot",
      );

      store.applyDelta({
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
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("fixed", HIGHEST_CLUSTER_MASS, "/repo/Fixed.cs"),
        cluster("next", 95, "/repo/Next.cs"),
        cluster("still", HIGH_CLUSTER_MASS, "/repo/Still.cs"),
      ]),
      1,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    store.applyDelta({
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
    const store = new ReportStore();
    store.setSnapshot(report([dirtyOnly, mixedSingleton, mixedPeers, clean]), 9);
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
      // never published. Rank is the engine's ([SEVERITY-BAND],
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
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    const errorNode = nodes.find(
      (n) => typeof n.contextValue === "string" && n.contextValue === "deslop.status.error",
    );
    assert.ok(errorNode, "top offenders must show a failed-lifecycle banner");
    assert.match(labelText(errorNode), /Stopped: crash/);
  });

  test("retains existing clusters during re-analysis — stale > blank ([VSIX-REACTIVITY-TREE])", () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster(FIRST_CLUSTER_ID, HIGHEST_CLUSTER_MASS, REPO_A_PATH),
        cluster(SECOND_CLUSTER_ID, HIGH_CLUSTER_MASS, REPO_B_PATH),
      ]),
      0,
    );
    store.setLifecycle({ kind: ANALYSING_PHASE });
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.ok(nodes.length >= MIN_VISIBLE_NODE_COUNT, "cluster rows must remain visible during re-analysis");
    const labels = nodes.map(labelText);
    assert.ok(labels.some((l) => /A\.cs/i.test(l) || /c1/i.test(l)), "A.cs cluster must stay visible");
    assert.ok(labels.some((l) => /B\.cs/i.test(l) || /c2/i.test(l)), "B.cs cluster must stay visible");
  });

  // [VSIX-TOP-OFFENDERS-FOLDER-MODE] Folder mode nests files under a
  // path-compressed folder tree; file leaves expand like file-mode roots
  // and global rank is preserved.
  test("folder mode builds a folder tree, impact-sorted, with global ranks", async () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("worst", HIGHEST_CLUSTER_MASS, ALPHA_FILE_PATH),
        cluster("mid", HIGH_CLUSTER_MASS, BETA_FILE_PATH),
        cluster("least", MEDIUM_CLUSTER_MASS, "/repo/src/a/Gamma.cs"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());
    await withGroupBy("folder", () => {
      const roots = provider.getChildren();
      assert.equal(roots.length, 1, "single-child chain compresses to one root");
      const [srcFolder] = roots;
      assert.ok(srcFolder instanceof FolderNode, "folder mode roots are FolderNodes");
      assert.equal(labelText(srcFolder), "repo/src", "path compression merges repo/src");
      const [folderA, folderB] = provider.getChildren(srcFolder);
      assert.ok(folderA && folderB, "src expands to folders a and b");
      assert.equal(labelText(folderA), "a", "folder a (worst weight 100) sorts before b (80)");
      assert.equal(labelText(folderB), "b");
      const [alphaFile] = provider.getChildren(folderA);
      assert.ok(alphaFile instanceof FileNode, "folder leaves are FileNodes");
      assert.match(labelText(alphaFile), /Alpha\.cs/);
      const [bucket] = provider.getChildren(alphaFile);
      assert.ok(bucket);
      const [topCluster] = provider.getChildren(bucket);
      assert.match(
        String(topCluster?.description ?? ""),
        /rank #1/,
        "Alpha's cluster keeps its global worst-first rank in folder mode",
      );
    });
  });

  // [VSIX-TOP-OFFENDERS-SORT] The sort axis reorders file/folder roots:
  // impact is worst-first, path is alphabetical.
  test("file mode sort axis: impact is worst-first, path is alphabetical", async () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster(HEAVY_CLUSTER_ID, HIGHEST_CLUSTER_MASS, HEAVY_FILE_PATH), cluster(LIGHT_CLUSTER_ID, TIED_CLUSTER_MASS, LIGHT_FILE_PATH)]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());
    await withGroupBy(FILE_GROUPING_MODE, async () => {
      const [impactFirst] = provider.getChildren();
      assert.ok(impactFirst);
      assert.match(labelText(impactFirst), /z\.cs/, "impact: heaviest file first");
      await withSetting(SORT_BY_SETTING, PATH_SORT_MODE, () => {
        const [pathFirst] = provider.getChildren();
        assert.ok(pathFirst);
        assert.match(labelText(pathFirst), /a\.cs/, "path: alphabetically first file first");
      });
    });
  });


  // [VSIX-TOP-OFFENDERS-SORT] The sort axis reorders cluster-mode rows too —
  // impact keeps worst-first, path is alphabetical — while the global rank #N
  // stays pinned to the report's worst-first order.
  test("cluster mode sort axis reorders clusters: impact worst-first, path alphabetical (rank unchanged)", async () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster(HEAVY_CLUSTER_ID, HIGHEST_CLUSTER_MASS, HEAVY_FILE_PATH), cluster(LIGHT_CLUSTER_ID, TIED_CLUSTER_MASS, LIGHT_FILE_PATH)]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const impact = provider.getChildren();
    assert.match(labelText(impact[0] as vscode.TreeItem), /z\.cs/, "impact: heaviest cluster first");
    assert.match(labelText(impact[1] as vscode.TreeItem), /a\.cs/);

    await withSetting(SORT_BY_SETTING, PATH_SORT_MODE, () => {
      const byPath = provider.getChildren();
      assert.match(labelText(byPath[0] as vscode.TreeItem), /a\.cs/, "path: alphabetically-first cluster leads");
      assert.match(labelText(byPath[1] as vscode.TreeItem), /z\.cs/);
      const aRow = byPath.find((node) => /a\.cs/.test(labelText(node)));
      assert.match(
        String(aRow?.description ?? ""),
        /\brank\s+#2\b/,
        "the path-sorted display never renumbers the global rank — a.cs's cluster is still rank #2",
      );
    });
  });

  // [VSIX-TOP-OFFENDERS-SORT] Occurrences inside a cluster sort by the axis too,
  // but the canonical badge follows the occurrence identity (original index 0),
  // never the display position.
  test("within-cluster occurrences sort by path; canonical identity stays on the original occurrence", async () => {
    const multi = withOccurrences(cluster("multi", LOW_CLUSTER_WEIGHT, "/repo/zzz.cs"), [
      reportOccurrence("/repo/zzz.cs", 0, 9),
      reportOccurrence("/repo/aaa.cs", 0, 9),
    ]);
    const store = new ReportStore();
    store.setSnapshot(report([multi]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const [root] = provider.getChildren();
    assert.ok(root, CLUSTER_ROOT_REQUIRED);

    const impactOccurrences = provider.getChildren(root);
    assert.match(labelText(impactOccurrences[0] as vscode.TreeItem), /zzz\.cs/, "impact: canonical occurrence first");
    assert.equal(impactOccurrences[0]?.contextValue, CANONICAL_OCCURRENCE_CONTEXT);

    await withSetting(SORT_BY_SETTING, PATH_SORT_MODE, () => {
      const pathOccurrences = provider.getChildren(root);
      assert.match(
        labelText(pathOccurrences[0] as vscode.TreeItem),
        /aaa\.cs/,
        "path: the alphabetically-first occurrence is displayed first",
      );
      assert.equal(
        pathOccurrences[0]?.contextValue,
        "deslop.occurrence",
        "the alphabetically-first occurrence is NOT falsely marked canonical",
      );
      const canonicalNode = pathOccurrences.find((node) => /zzz\.cs/.test(labelText(node)));
      assert.equal(
        canonicalNode?.contextValue,
        CANONICAL_OCCURRENCE_CONTEXT,
        "canonical identity follows the original occurrence (index 0), not the display position",
      );
    });
  });

  // [VSIX-TOP-OFFENDERS-TOOLBAR] Expand All / Collapse All rewrite the collapsible
  // state the provider returns and release on the next data change.
  test("Expand All / Collapse All set the collapsible state the provider returns; released on data change", () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster("a", HIGHEST_CLUSTER_MASS, REPO_A_PATH), cluster("b", HIGH_CLUSTER_MASS, REPO_B_PATH)]),
      1,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());
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

  // [VSIX-VIEW-STATE-UI-ONLY] Sorting is a pure presentation transform: it reorders
  // the rows already in the store and never re-fetches — the generation is untouched.
  test("sorting is UI-only: flipping the axis reorders existing rows without bumping the generation", async () => {
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster(HEAVY_CLUSTER_ID, HIGHEST_CLUSTER_MASS, HEAVY_FILE_PATH), cluster(LIGHT_CLUSTER_ID, TIED_CLUSTER_MASS, LIGHT_FILE_PATH)]),
      7,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const impactOrder = provider.getChildren().map(labelText);

    await withSetting(SORT_BY_SETTING, PATH_SORT_MODE, () => {
      const pathOrder = provider.getChildren().map(labelText);
      assert.notDeepEqual(pathOrder, impactOrder, "the sort axis actually changes the displayed order");
      assert.match(pathOrder[0] ?? "", /a\.cs/, "path order leads with the alphabetically-first file");
      assert.equal(
        store.current.generation,
        7,
        "sorting must NOT bump the generation — it re-renders the same store data, never re-fetching from the LSP",
      );
    });
  });
});
