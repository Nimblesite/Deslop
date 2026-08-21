// Unit: the facet model ([FACET-MODEL] / [FACET-TOP-OFFENDERS-FILTER] /
// [FACET-GROUP-BY-TYPE]). Covers the shared filter slice every listing
// surface funnels through, the sanitizer's typo fallback, and the
// type-grouping mode's flat bucket roots (#258).

import * as assert from "node:assert/strict";

import {
  applyFacetFilter,
  IDENTICAL_BUCKET_VALUE,
  sanitizeFacetFilter,
} from "../../types/report";
import { buildTypeMode, getGroupNodeChildren } from "../../tree/grouping";
import { BucketGroupNode, ClusterNode } from "../../tree/nodes";
import { StatusTicker, TopOffendersProvider } from "../../tree/providers";
import { ReportStore } from "../../reportStore";
import { cluster, labelText, report, withSetting } from "./tree.helpers";

// One cluster per bucket/category combination the tests slice on, each
// carrying the global rank the engine stamped on it — worst first.
const identicalLogic = cluster("aaaaaaa1", 9, "a.cs", 0, 20, IDENTICAL_BUCKET_VALUE, undefined, 1);
const nearlyLogic = cluster("bbbbbbb2", 7, "b.cs", 0, 20, "nearly_identical", undefined, 2);
const identicalData = cluster("ccccccc3", 5, "c.dart", 0, 20, IDENTICAL_BUCKET_VALUE, "data", 3);
const ALL = [identicalLogic, nearlyLogic, identicalData];

suite("facet filter slice ([FACET-TOP-OFFENDERS-FILTER])", () => {
  test("empty filter shows all clusters", () => {
    const out = applyFacetFilter(ALL, { buckets: [], categories: [] });
    assert.deepEqual(out.map((c) => c.id), ALL.map((c) => c.id));
  });

  test("bucket axis keeps only matching clusters", () => {
    const out = applyFacetFilter(ALL, { buckets: [IDENTICAL_BUCKET_VALUE], categories: [] });
    assert.deepEqual(out.map((c) => c.id), [identicalLogic.id, identicalData.id]);
  });

  test("category axis keeps only matching clusters", () => {
    const out = applyFacetFilter(ALL, { buckets: [], categories: ["data"] });
    assert.deepEqual(out.map((c) => c.id), [identicalData.id]);
  });

  test("the two axes compose as an AND", () => {
    const out = applyFacetFilter(ALL, { buckets: [IDENTICAL_BUCKET_VALUE], categories: ["logic"] });
    assert.deepEqual(out.map((c) => c.id), [identicalLogic.id]);
  });

  test("unknown values are dropped by the sanitizer — a typo never empties the tree", () => {
    const sanitized = sanitizeFacetFilter(["not_a_bucket"], ["not_a_category"]);
    assert.deepEqual(sanitized, { buckets: [], categories: [] });
    const out = applyFacetFilter(ALL, sanitized);
    assert.equal(out.length, ALL.length, "fallback-to-all after sanitizing");
  });

  test("known values survive the sanitizer alongside dropped unknowns", () => {
    const sanitized = sanitizeFacetFilter([IDENTICAL_BUCKET_VALUE, "bogus"], ["data", "bogus"]);
    assert.deepEqual(sanitized.buckets, [IDENTICAL_BUCKET_VALUE]);
    assert.deepEqual(sanitized.categories, ["data"]);
  });
});

suite("type grouping mode ([FACET-GROUP-BY-TYPE])", () => {
  // #258: type mode groups by BUCKET, not category — every Identical
  // cluster surfaces together in one flat group, with no category or
  // file/folder sub-grouping in between.
  test("roots are one flat group per bucket present, so all Identical clusters sit together", () => {
    const roots = buildTypeMode(ALL, "impact");
    assert.equal(roots.length, 2, "identical + nearly-identical groups; absent buckets omitted");
    const [identicalGroup, nearlyGroup] = roots as [BucketGroupNode, BucketGroupNode];
    assert.ok(identicalGroup instanceof BucketGroupNode);
    assert.equal(
      labelText(identicalGroup),
      "Identical code (2)",
      "groups are labelled by the shared bucket plain title with a live count",
    );
    assert.equal(labelText(nearlyGroup), "Nearly identical code (1)");
    const identicalChildren = getGroupNodeChildren(identicalGroup) as ClusterNode[];
    assert.deepEqual(
      identicalChildren.map((node) => node.cluster.id),
      [identicalLogic.id, identicalData.id],
      "logic and data clusters share the Identical group — bucket grouping crosses categories, flat",
    );
    assert.ok(
      identicalChildren.every((node) => node instanceof ClusterNode),
      "children are cluster rows directly — no intermediate file/folder layer",
    );
  });

  test("single-bucket reports render a single group, absent buckets never render empty", () => {
    const identicalOnly = [identicalLogic, identicalData];
    const roots = buildTypeMode(identicalOnly, "impact");
    assert.equal(roots.length, 1);
    assert.equal(labelText(roots[0] as BucketGroupNode), "Identical code (2)");
  });

  test("children keep the GLOBAL rank (gaps allowed) and show their file", () => {
    const roots = buildTypeMode(ALL, "impact");
    const identicalChildren = getGroupNodeChildren(roots[0] as BucketGroupNode);
    assert.equal(identicalChildren.length, 2);
    const child = identicalChildren[1] as ClusterNode;
    assert.ok(child instanceof ClusterNode);
    assert.equal(child.rank, 3, "rank #3 from the global worst-first list, not renumbered");
    assert.ok(
      labelText(child).includes("c.dart"),
      `type-group children are roots without a file ancestor, so the file must show: ${labelText(child)}`,
    );
  });

  test("the path sort axis orders clusters inside groups by representative path", () => {
    // d.cs carries the heaviest weight so path order and impact order
    // disagree inside the Identical group — the axis must win.
    const identicalHeavy = cluster("ddddddd4", 20, "d.cs", 0, 20, IDENTICAL_BUCKET_VALUE);
    const withHeavy = [...ALL, identicalHeavy];
    const roots = buildTypeMode(withHeavy, "path");
    const identicalChildren = getGroupNodeChildren(roots[0] as BucketGroupNode) as ClusterNode[];
    assert.deepEqual(
      identicalChildren.map((node) => node.cluster.id),
      [identicalLogic.id, identicalData.id, identicalHeavy.id],
      "a.cs before c.dart before d.cs under the path axis, weight order ignored",
    );
  });
});

// [FACET-TESTING] Cross-surface consistency: the Top Offenders tree
// renders exactly the cluster-id set the shared applyFacetFilter slice
// produces — the same function the report webview and status bar use —
// with global rank gaps preserved and the filtered status row leading.
suite("facet filter cross-surface consistency", () => {
  test("filtered tree = shared slice, rank gaps kept, status row leads with clear action", async () => {
    await withSetting("topOffenders.filterBuckets", [IDENTICAL_BUCKET_VALUE], () => {
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
      const expected = applyFacetFilter(ALL, { buckets: [IDENTICAL_BUCKET_VALUE], categories: [] });
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
