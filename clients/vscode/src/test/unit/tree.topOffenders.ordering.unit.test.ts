import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { cluster, labelText, report, storeWith, topOffenders, withGroupBy, withSavedSetting } from "./tree.helpers";
import { HIGHEST_CLUSTER_MASS, TIED_CLUSTER_MASS, FILE_GROUPING_MODE, SORT_BY_SETTING, PATH_SORT_MODE, CLUSTER_ROOT_REQUIRED, CANONICAL_OCCURRENCE_CONTEXT, HEAVY_CLUSTER_ID, HEAVY_FILE_PATH, LIGHT_CLUSTER_ID, LIGHT_FILE_PATH, LOW_CLUSTER_WEIGHT, reportOccurrence, withOccurrences } from "./tree.topOffenders.fixtures";

suite("TopOffendersProvider ordering", () => {
  // [VSIX-TOP-OFFENDERS-SORT] Saved path preferences cannot override weight order.
  test("file mode stays highest weight first even with an old path-sort setting", async () => {
    const store = storeWith(report([cluster(HEAVY_CLUSTER_ID, HIGHEST_CLUSTER_MASS, HEAVY_FILE_PATH), cluster(LIGHT_CLUSTER_ID, TIED_CLUSTER_MASS, LIGHT_FILE_PATH)]));
    const provider = topOffenders(store);
    await withGroupBy(FILE_GROUPING_MODE, async () => {
      const [impactFirst] = provider.getChildren();
      assert.ok(impactFirst);
      assert.match(labelText(impactFirst), /z\.cs/, "impact: heaviest file first");
      await withSavedSetting(SORT_BY_SETTING, PATH_SORT_MODE, () => {
        const [pathFirst] = provider.getChildren();
        assert.ok(pathFirst);
        assert.match(labelText(pathFirst), /z\.cs/, "an old path setting must keep the heaviest file first");
      });
    });
  });

  // [VSIX-TOP-OFFENDERS-SORT] Both display order and engine ranks remain stable.
  test("cluster mode preserves highest weight first and global ranks under old settings", async () => {
    const store = storeWith(report([cluster(HEAVY_CLUSTER_ID, HIGHEST_CLUSTER_MASS, HEAVY_FILE_PATH), cluster(LIGHT_CLUSTER_ID, TIED_CLUSTER_MASS, LIGHT_FILE_PATH)]));
    const provider = topOffenders(store);

    const impact = provider.getChildren();
    assert.match(labelText(impact[0] as vscode.TreeItem), /z\.cs/, "impact: heaviest cluster first");
    assert.match(labelText(impact[1] as vscode.TreeItem), /a\.cs/);

    await withSavedSetting(SORT_BY_SETTING, PATH_SORT_MODE, () => {
      const byPath = provider.getChildren();
      assert.match(labelText(byPath[0] as vscode.TreeItem), /z\.cs/, "the heaviest cluster always leads");
      assert.match(labelText(byPath[1] as vscode.TreeItem), /a\.cs/);
      const aRow = byPath.find((node) => /a\.cs/.test(labelText(node)));
      assert.match(
        String(aRow?.description ?? ""),
        /\brank\s+#2\b/,
        "saved path preferences never renumber the global rank — a.cs's cluster is still rank #2",
      );
    });
  });

  // [VSIX-TOP-OFFENDERS-SORT] Canonical order and identity remain stable.
  test("within-cluster occurrences keep canonical order under old path-sort settings", async () => {
    const multi = withOccurrences(cluster("multi", LOW_CLUSTER_WEIGHT, "/repo/zzz.cs"), [
      reportOccurrence("/repo/zzz.cs", 0, 9),
      reportOccurrence("/repo/aaa.cs", 0, 9),
    ]);
    const store = storeWith(report([multi]));
    const provider = topOffenders(store);
    const [root] = provider.getChildren();
    assert.ok(root, CLUSTER_ROOT_REQUIRED);

    const impactOccurrences = provider.getChildren(root);
    assert.match(labelText(impactOccurrences[0] as vscode.TreeItem), /zzz\.cs/, "impact: canonical occurrence first");
    assert.equal(impactOccurrences[0]?.contextValue, CANONICAL_OCCURRENCE_CONTEXT);

    await withSavedSetting(SORT_BY_SETTING, PATH_SORT_MODE, () => {
      const pathOccurrences = provider.getChildren(root);
      assert.match(
        labelText(pathOccurrences[0] as vscode.TreeItem),
        /zzz\.cs/,
        "canonical occurrence is always displayed first",
      );
      assert.equal(
        pathOccurrences[0]?.contextValue,
        CANONICAL_OCCURRENCE_CONTEXT,
        "the canonical row keeps its identity",
      );
      const canonicalNode = pathOccurrences.find((node) => /zzz\.cs/.test(labelText(node)));
      assert.equal(
        canonicalNode?.contextValue,
        CANONICAL_OCCURRENCE_CONTEXT,
        "canonical identity follows the original occurrence (index 0), not the display position",
      );
    });
  });

  // [VSIX-VIEW-STATE-UI-ONLY] Retired settings cannot trigger re-analysis.
  test("retired sort settings cannot reorder rows or bump the generation", async () => {
    const store = storeWith(
      report([cluster(HEAVY_CLUSTER_ID, HIGHEST_CLUSTER_MASS, HEAVY_FILE_PATH), cluster(LIGHT_CLUSTER_ID, TIED_CLUSTER_MASS, LIGHT_FILE_PATH)]),
      7,
    );
    const provider = topOffenders(store);
    const impactOrder = provider.getChildren().map(labelText);

    await withSavedSetting(SORT_BY_SETTING, PATH_SORT_MODE, () => {
      const pathOrder = provider.getChildren().map(labelText);
      assert.deepEqual(pathOrder, impactOrder, "retired sort settings leave the displayed order unchanged");
      assert.match(pathOrder[0] ?? "", /z\.cs/, "weight order always leads with the heaviest file");
      assert.equal(
        store.current.generation,
        7,
        "sorting must NOT bump the generation — it re-renders the same store data, never re-fetching from the LSP",
      );
    });
  });
});
