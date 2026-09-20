import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { FileNode, FolderNode } from "../../tree/providers";
import { ReportStore } from "../../reportStore";
import { CLUSTER_KINDS, kindTitle } from "../../types/report";
import { cluster, iconColorId, labelText, report, storeWith, tooltipText, topOffenders, withGroupBy } from "./tree.helpers";
import { KIND_THEME_COLOR } from "../../design";
import { DEFAULT_OCCURRENCE_END_BYTE, IDENTICAL_KIND, STRUCTURAL_ONLY_KIND, HIGHEST_CLUSTER_MASS, HIGH_CLUSTER_MASS, MEDIUM_CLUSTER_MASS, TIED_CLUSTER_MASS, FILE_GROUPING_MODE, MIXED_FILE_PATH, REPO_A_PATH, REPO_B_PATH, ALPHA_FILE_PATH, FIRST_CLUSTER_ID, SECOND_CLUSTER_ID, BETA_FILE_PATH, CLUSTER_ROOT_REQUIRED, HEAVY_CLUSTER_ID, WHOLE_NUMBER_MASS, TWO_DECIMAL_MASS, FOLDER_GROUPING_MODE, THIRD_ITEM_INDEX, PAIR_COUNT, FILE_ROOT_COUNT, firstRootIn } from "./tree.topOffenders.fixtures";

suite("TopOffendersProvider groups", () => {
  test("file mode roots are FileNodes sorted by max cluster weight desc", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE]
    const store = storeWith(
      report([
        cluster("rank-1-beta", HIGHEST_CLUSTER_MASS, BETA_FILE_PATH),
        cluster("rank-2-alpha", HIGH_CLUSTER_MASS, ALPHA_FILE_PATH),
        cluster("rank-3-alpha", MEDIUM_CLUSTER_MASS, ALPHA_FILE_PATH),
        cluster("rank-4-gamma", 40, "/repo/src/c/Gamma.cs"),
      ]),
    );
    const provider = topOffenders(store);

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
    const store = storeWith(
      report([
        cluster("a-only", TIED_CLUSTER_MASS, "/repo/AlphaOnly.cs"),
        cluster("b-1", TIED_CLUSTER_MASS, "/repo/BetaPair.cs"),
        cluster("b-2", TIED_CLUSTER_MASS, "/repo/BetaPair.cs"),
        cluster("c-only", TIED_CLUSTER_MASS, "/repo/CharlieOnly.cs"),
      ]),
    );
    const provider = topOffenders(store);

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
    const store = storeWith(
      report([
        cluster("error", HIGHEST_CLUSTER_MASS, REPO_A_PATH),
        cluster("middle", HIGH_CLUSTER_MASS, REPO_B_PATH),
        cluster("least", MEDIUM_CLUSTER_MASS, REPO_A_PATH),
      ]),
    );
    const provider = topOffenders(store);

    const thirdRow = provider.getChildren()[2] as vscode.TreeItem;
    assert.match(
      String(thirdRow.description ?? ""),
      /\brank\s+#3\b/,
      "third row in cluster mode is rank #3 (least)",
    );

    await withGroupBy(FILE_GROUPING_MODE, () => {
      const fileRoots = provider.getChildren();
      // A.cs (max mass 100) sits before B.cs (max mass 80). A.cs expands
      // to kind groups whose clusters keep their global ranks: #1 and #3.
      const aFile = fileRoots[0];
      assert.ok(aFile);
      const kindGroups = provider.getChildren(aFile);
      const aClusters = kindGroups.flatMap((group) => provider.getChildren(group));
      assert.ok(aClusters.length >= 2, "A.cs must expose both of its clusters under kind groups");
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
    const store = storeWith(report([cluster("c", HIGHEST_CLUSTER_MASS, MIXED_FILE_PATH)]));
    const provider = topOffenders(store);

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

  test("file mode children are clone-kind groups; only kinds present appear", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE] / [FACET-GROUP-BY-KIND]
    const store = storeWith(
      report([
        cluster(FIRST_CLUSTER_ID, HIGHEST_CLUSTER_MASS, MIXED_FILE_PATH, 0, DEFAULT_OCCURRENCE_END_BYTE, "error", 1, STRUCTURAL_ONLY_KIND),
        cluster(SECOND_CLUSTER_ID, HIGH_CLUSTER_MASS, MIXED_FILE_PATH, 0, DEFAULT_OCCURRENCE_END_BYTE, "warning", 2, IDENTICAL_KIND),
        cluster("c3", MEDIUM_CLUSTER_MASS, MIXED_FILE_PATH, 0, DEFAULT_OCCURRENCE_END_BYTE, "error", 3, IDENTICAL_KIND),
      ]),
    );
    const provider = topOffenders(store);

    await withGroupBy(FILE_GROUPING_MODE, () => {
      const [fileRoot] = provider.getChildren();
      assert.ok(fileRoot instanceof FileNode, "single root must be a FileNode");
      const groups = provider.getChildren(fileRoot);
      // [CLONE-KIND-FOLD] The engine stamps each cluster's kind, so the
      // groups are exactly the distinct stamped kinds — one group per
      // present kind, never a kind no cluster holds.
      //
      // [CLONE-KIND-LABELS] Sections follow the category display order,
      // not the mass ranking. The shape-only cluster here is deliberately
      // the heaviest in the file: [CLONE-BUCKETS-STRUCTURAL-ONLY] puts
      // shape-only "always the last category, below Similar, regardless of
      // its size or number of matches", so weight must not lift it above
      // the byte-identical copies. Kind mode orders its roots the same
      // way, so one file cannot contradict the tree beside it.
      const groupLabels = groups.map(labelText);
      assert.equal(groups.length, 2, "one group per present kind");
      assert.deepEqual(
        groupLabels.map((label) => label.split(" (")[0]),
        [kindTitle(IDENTICAL_KIND), kindTitle(STRUCTURAL_ONLY_KIND)],
        `sections follow the category display order: ${groupLabels.join(" | ")}`,
      );
      assert.ok(
        groupLabels[0]?.startsWith(kindTitle(IDENTICAL_KIND)),
        `byte-identical code leads the file, however light it is: ${groupLabels.join(" | ")}`,
      );
      assert.ok(
        groupLabels.at(-1)?.startsWith(kindTitle(STRUCTURAL_ONLY_KIND)),
        `information is last even holding the file's heaviest cluster: ${groupLabels.join(" | ")}`,
      );
      assert.match(groupLabels[0] ?? "", /\(2\)$/, "the identical group counts both identical clusters");
      assert.match(groupLabels[1] ?? "", /\(1\)$/, "the shape-only group counts its one finding");
      const [identicalGroup, structuralGroup] = groups;
      assert.ok(structuralGroup && identicalGroup);
      assert.equal(iconColorId(identicalGroup), KIND_THEME_COLOR[IDENTICAL_KIND]);
      assert.equal(iconColorId(structuralGroup), KIND_THEME_COLOR[STRUCTURAL_ONLY_KIND]);

      // Both grouping axes read the one registry, so the same repository
      // cannot order its categories differently depending on which axis
      // is active ([CLONE-KIND-LABELS]).
      assert.deepEqual(
        groupLabels.map((label) => label.split(" (")[0]),
        CLUSTER_KINDS.map(kindTitle).filter((title) =>
          groupLabels.some((label) => label.startsWith(title)),
        ),
        "file-mode sections follow the same registry order kind mode does",
      );
    });
  });

  test("file mode clusters under a bucket are sorted by weight desc and drop the file suffix", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE]
    const store = storeWith(
      report([
        cluster("hi", HIGHEST_CLUSTER_MASS, MIXED_FILE_PATH, 0, DEFAULT_OCCURRENCE_END_BYTE, "error"),
        cluster("lo", MEDIUM_CLUSTER_MASS, MIXED_FILE_PATH, 0, DEFAULT_OCCURRENCE_END_BYTE, "error"),
      ]),
    );
    const provider = topOffenders(store);

    await withGroupBy(FILE_GROUPING_MODE, () => {
      const [fileRoot] = provider.getChildren();
      assert.ok(fileRoot, "file root must exist");
      // Both clusters carry the fixture kind, so they share one kind group.
      // Flattened, they must still be worst-first and drop the parent file
      // suffix.
      const kindGroups = provider.getChildren(fileRoot);
      assert.equal(kindGroups.length, 1, "one kind group for one shared kind");
      const clusterNodes = kindGroups.flatMap((group) => provider.getChildren(group));
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
    const store = storeWith(report([cluster("only", HIGHEST_CLUSTER_MASS, "/repo/src/Mixed.cs", 0, DEFAULT_OCCURRENCE_END_BYTE, "error")]));
    const provider = topOffenders(store);

    const [clusterMode] = provider.getChildren();
    assert.ok(clusterMode);
    assert.match(tooltipText(clusterMode), /\/repo\/src\/Mixed\.cs/);

    await withGroupBy(FILE_GROUPING_MODE, () => {
      const [fileRoot] = provider.getChildren();
      assert.ok(fileRoot);
      const [kindGroup] = provider.getChildren(fileRoot);
      assert.ok(kindGroup);
      const [fileModeCluster] = provider.getChildren(kindGroup);
      assert.ok(fileModeCluster);
      assert.match(tooltipText(fileModeCluster), /\/repo\/src\/Mixed\.cs/);
    });
  });

  test("file mode occurrence leaves match cluster mode byte-for-byte", async () => {
    // [VSIX-TOP-OFFENDERS-FILE-MODE] No special-case rendering for leaves.
    const store = new ReportStore();
    const c = cluster("only", 100, "/repo/src/Mixed.cs", 7, 14, "error");
    store.setSnapshot(report([c]), 0);
    const provider = topOffenders(store);

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

  // [VSIX-TOP-OFFENDERS-FOLDER-MODE] Folder mode nests files under a
  // path-compressed folder tree; file leaves expand like file-mode roots
  // and global rank is preserved.
  test("mass prints as a whole number on cluster, file and folder rows", async () => {
    // [RANK-MASS-SUM] Mass is a count, so every tree surface prints it with
    // no decimal point — the string the CLI text report prints — never at
    // the two-decimal precision reserved for measured pair signals.
    const store = storeWith(report([cluster(HEAVY_CLUSTER_ID, WHOLE_NUMBER_MASS, ALPHA_FILE_PATH)]));
    const provider = topOffenders(store);
    const [clusterRow] = provider.getChildren();
    assert.ok(clusterRow, CLUSTER_ROOT_REQUIRED);
    const fileRow = await firstRootIn(provider, FILE_GROUPING_MODE);
    const folderRow = await firstRootIn(provider, FOLDER_GROUPING_MODE);
    const rendered = [tooltipText(clusterRow), String(fileRow.description), String(folderRow.description)];
    assert.ok(rendered[0]?.includes(`mass: \`${WHOLE_NUMBER_MASS}\``), `cluster tooltip: ${rendered[0] ?? ""}`);
    assert.equal(rendered[1], `worst mass ${WHOLE_NUMBER_MASS}`, "file row description");
    assert.equal(rendered[2], `worst mass ${WHOLE_NUMBER_MASS} · 1 file`, "folder row description");
    for (const text of rendered) {
      assert.equal(text.includes(TWO_DECIMAL_MASS), false, `a count must never print as ${TWO_DECIMAL_MASS}: ${text}`);
    }
  });

  test("folder mode builds a folder tree, impact-sorted, with global ranks", async () => {
    const store = storeWith(
      report([
        cluster("error", HIGHEST_CLUSTER_MASS, ALPHA_FILE_PATH),
        cluster("information", HIGH_CLUSTER_MASS, BETA_FILE_PATH),
        cluster("least", MEDIUM_CLUSTER_MASS, "/repo/src/a/Gamma.cs"),
      ]),
    );
    const provider = topOffenders(store);
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
});
