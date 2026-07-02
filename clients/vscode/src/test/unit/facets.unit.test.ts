// Unit: the facet model ([FACET-MODEL] / [FACET-TOP-OFFENDERS-FILTER] /
// [FACET-GROUP-BY-TYPE]). Covers the shared filter slice every listing
// surface funnels through, the sanitizer's typo fallback, and the
// type-grouping mode's category roots.

import * as assert from "node:assert/strict";

import {
  applyFacetFilter,
  sanitizeFacetFilter,
} from "../../types/report";
import {
  buildRankIndex,
  buildTypeMode,
  getGroupNodeChildren,
} from "../../tree/grouping";
import { ClusterNode, TypeGroupNode } from "../../tree/nodes";
import { StatusTicker, TopOffendersProvider } from "../../tree/providers";
import { ReportStore } from "../../reportStore";
import { cluster, labelText, report, withSetting } from "./tree.helpers";

// One cluster per bucket/category combination the tests slice on.
const identicalLogic = cluster("aaaaaaa1", 9, "a.cs", 0, 20, "identical");
const nearlyLogic = cluster("bbbbbbb2", 7, "b.cs", 0, 20, "nearly_identical");
const identicalData = cluster("ccccccc3", 5, "c.dart", 0, 20, "identical", "data");
const ALL = [identicalLogic, nearlyLogic, identicalData];

suite("facet filter slice ([FACET-TOP-OFFENDERS-FILTER])", () => {
  test("empty filter shows all clusters", () => {
    const out = applyFacetFilter(ALL, { buckets: [], categories: [] });
    assert.deepEqual(out.map((c) => c.id), ALL.map((c) => c.id));
  });

  test("bucket axis keeps only matching clusters", () => {
    const out = applyFacetFilter(ALL, { buckets: ["identical"], categories: [] });
    assert.deepEqual(out.map((c) => c.id), [identicalLogic.id, identicalData.id]);
  });

  test("category axis keeps only matching clusters", () => {
    const out = applyFacetFilter(ALL, { buckets: [], categories: ["data"] });
    assert.deepEqual(out.map((c) => c.id), [identicalData.id]);
  });

  test("the two axes compose as an AND", () => {
    const out = applyFacetFilter(ALL, { buckets: ["identical"], categories: ["logic"] });
    assert.deepEqual(out.map((c) => c.id), [identicalLogic.id]);
  });

  test("unknown values are dropped by the sanitizer — a typo never empties the tree", () => {
    const sanitized = sanitizeFacetFilter(["not_a_bucket"], ["not_a_category"]);
    assert.deepEqual(sanitized, { buckets: [], categories: [] });
    const out = applyFacetFilter(ALL, sanitized);
    assert.equal(out.length, ALL.length, "fallback-to-all after sanitizing");
  });

  test("known values survive the sanitizer alongside dropped unknowns", () => {
    const sanitized = sanitizeFacetFilter(["identical", "bogus"], ["data", "bogus"]);
    assert.deepEqual(sanitized.buckets, ["identical"]);
    assert.deepEqual(sanitized.categories, ["data"]);
  });
});

suite("type grouping mode ([FACET-GROUP-BY-TYPE])", () => {
  const severities = new Map([[identicalLogic.id, "worst" as const]]);

  test("roots are one group per category present, registry order, empty omitted", () => {
    const roots = buildTypeMode(ALL, severities, buildRankIndex(ALL), "impact");
    assert.equal(roots.length, 2, "logic and data groups, nothing for absent categories");
    const [logicGroup, dataGroup] = roots as [TypeGroupNode, TypeGroupNode];
    assert.ok(logicGroup instanceof TypeGroupNode);
    assert.equal(
      labelText(logicGroup),
      "Code clones (2)",
      "the chip-less logic category uses the plain group title with a live count",
    );
    assert.equal(
      labelText(dataGroup),
      "data table (1)",
      "chip-carrying categories are labelled by the shared chip",
    );
  });

  test("logic-only reports render a single group", () => {
    const logicOnly = [identicalLogic, nearlyLogic];
    const roots = buildTypeMode(logicOnly, severities, buildRankIndex(logicOnly), "impact");
    assert.equal(roots.length, 1);
    assert.equal(labelText(roots[0] as TypeGroupNode), "Code clones (2)");
  });

  test("children keep the GLOBAL rank (gaps allowed) and show their file", () => {
    const rankIndex = buildRankIndex(ALL);
    const roots = buildTypeMode(ALL, severities, rankIndex, "impact");
    const dataChildren = getGroupNodeChildren(roots[1] as TypeGroupNode);
    assert.equal(dataChildren.length, 1);
    const child = dataChildren[0] as ClusterNode;
    assert.ok(child instanceof ClusterNode);
    assert.equal(child.rank, 3, "rank #3 from the global worst-first list, not renumbered");
    assert.ok(
      labelText(child).includes("c.dart"),
      `type-group children are roots without a file ancestor, so the file must show: ${labelText(child)}`,
    );
  });

  test("the path sort axis orders clusters inside groups by representative path", () => {
    const roots = buildTypeMode(ALL, severities, buildRankIndex(ALL), "path");
    const logicChildren = getGroupNodeChildren(roots[0] as TypeGroupNode) as ClusterNode[];
    assert.deepEqual(
      logicChildren.map((node) => node.cluster.id),
      [identicalLogic.id, nearlyLogic.id],
      "a.cs before b.cs under the path axis",
    );
  });
});

// [FACET-TESTING] Cross-surface consistency: the Top Offenders tree
// renders exactly the cluster-id set the shared applyFacetFilter slice
// produces — the same function the report webview and status bar use —
// with global rank gaps preserved and the filtered status row leading.
suite("facet filter cross-surface consistency", () => {
  test("filtered tree = shared slice, rank gaps kept, status row leads with clear action", async () => {
    await withSetting("topOffenders.filterBuckets", ["identical"], () => {
      const store = new ReportStore();
      store.setSnapshot(report(ALL), 0);
      store.setLifecycle({ kind: "ready" });
      const provider = new TopOffendersProvider(store, new StatusTicker());
      const nodes = provider.getChildren();

      const [statusRow] = nodes;
      assert.ok(statusRow, "the filtered status row must lead the tree");
      assert.equal(
        labelText(statusRow),
        "Filtered: Identical code — Clear filter",
        "the status row names the active facet with the shared plain title",
      );
      assert.equal(
        statusRow.command?.command,
        "deslop.topOffenders.clearFilter",
        "the clear action is bound on the status row",
      );

      const rows = nodes.filter((node): node is ClusterNode => node instanceof ClusterNode);
      const expected = applyFacetFilter(ALL, { buckets: ["identical"], categories: [] });
      assert.deepEqual(
        rows.map((node) => node.cluster.id),
        expected.map((c) => c.id),
        "tree renders exactly the shared slice's cluster-id set",
      );
      assert.deepEqual(
        rows.map((node) => node.rank),
        [1, 3],
        "global ranks keep their gaps — a filtered view legitimately shows #1, #3",
      );
    });
  });

  test("a filtered-empty tree shows the status row, never the clean verdict", async () => {
    await withSetting("topOffenders.filterBuckets", ["same_behavior"], () => {
      const store = new ReportStore();
      store.setSnapshot(report(ALL), 0);
      store.setLifecycle({ kind: "ready" });
      const provider = new TopOffendersProvider(store, new StatusTicker());
      const nodes = provider.getChildren();
      assert.equal(nodes.length, 1, "only the filtered status row renders");
      const [statusRow] = nodes;
      assert.ok(statusRow);
      assert.match(labelText(statusRow), /^Filtered: /);
      assert.ok(
        !nodes.some((node) => labelText(node).includes("No duplication detected")),
        "a filtered-empty tree must never be mistakable for the clean state",
      );
    });
  });
});
