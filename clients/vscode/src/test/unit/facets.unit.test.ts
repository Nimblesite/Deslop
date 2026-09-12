// Unit: the facet model ([FACET-MODEL] / [FACET-TOP-OFFENDERS-FILTER] /
// [FACET-GROUP-BY-KIND]). Covers the shared filter slice every listing
// surface funnels through, the sanitizer's typo fallback, and the
// kind-grouping mode's flat roots (#258, re-stated on the clone kind).

import * as assert from "node:assert/strict";

import {
  applyFacetFilter,
  kindTitle,
  sanitizeFacetFilter,
} from "../../types/report";
import { buildKindMode, getGroupNodeChildren } from "../../tree/grouping";
import { ClusterNode, KindGroupNode } from "../../tree/nodes";
import { cluster, labelText, report, storeWith, topOffenders, withSetting } from "./tree.helpers";
import { stampRanks } from "../cluster.helpers";

const WORST_SEVERITY = "error";
const MID_SEVERITY = "information";
const FAINT_SEVERITY = "hint";

// Independent diagnostic levels prove filtering preserves global rank gaps.
const STAMPED_COUNT = 20;
const FIXTURE_SEVERITIES = [WORST_SEVERITY, "warning"] as const;
const ALL = stampRanks(
  Array.from({ length: STAMPED_COUNT }, (_, index) =>
    cluster(`cluster${String(index + 1).padStart(2, "0")}`, STAMPED_COUNT - index, `f${index + 1}.cs`, 0, 20, FIXTURE_SEVERITIES[index] ?? MID_SEVERITY, index + 1),
  ),
);
const RANK_ONE_ID = "cluster01";
const RANK_TWO_ID = "cluster02";

const IDENTICAL_KIND = "identical";
const NEARLY_IDENTICAL_KIND = "nearly_identical";
const LOOSELY_SIMILAR_KIND = "loosely_similar";

// Category grouping is independent of mass and diagnostic severity.
const KIND_GROUPED = stampRanks([
  cluster("aaaaaaa1", 9, "a.cs", 0, 20, WORST_SEVERITY, 1, NEARLY_IDENTICAL_KIND),
  cluster("bbbbbbb2", 7, "b.cs", 0, 20, MID_SEVERITY, 2, IDENTICAL_KIND),
  cluster("ccccccc3", 5, "c.dart", 0, 20, MID_SEVERITY, 3, LOOSELY_SIMILAR_KIND),
  cluster("ddddddd4", 3, "d.rs", 0, 20, FAINT_SEVERITY, 4, IDENTICAL_KIND),
]);

suite("facet filter slice ([FACET-TOP-OFFENDERS-FILTER])", () => {
  test("empty filter shows all clusters", () => {
    const out = applyFacetFilter(ALL, { severities: [] });
    assert.deepEqual(out.map((c) => c.id), ALL.map((c) => c.id));
  });

  test("severity axis keeps only matching clusters", () => {
    const out = applyFacetFilter(ALL, { severities: [WORST_SEVERITY] });
    assert.deepEqual(out.map((c) => c.id), [RANK_ONE_ID]);
    assert.deepEqual(
      out.map((c) => c.severity),
      [WORST_SEVERITY],
    );
  });

  test("unknown values are dropped by the sanitizer — a typo never empties the tree", () => {
    const sanitized = sanitizeFacetFilter(["not_a_severity"]);
    assert.deepEqual(sanitized, { severities: [] });
    const out = applyFacetFilter(ALL, sanitized);
    assert.equal(out.length, ALL.length, "fallback-to-all after sanitizing");
  });

  test("known values survive the sanitizer alongside dropped unknowns", () => {
    const sanitized = sanitizeFacetFilter([WORST_SEVERITY, "bogus"]);
    assert.deepEqual(sanitized.severities, [WORST_SEVERITY]);
  });
});

suite("clone-kind grouping mode ([FACET-GROUP-BY-KIND])", () => {
  // Kind mode groups by the engine's clone kind — every identical cluster
  // surfaces together in one flat group, strongest kind first, with no
  // file/folder sub-grouping in between ([FACET-GROUP-BY-KIND]).
  test("roots are one flat group per kind present, strongest first, so all identical clusters sit together", () => {
    const roots = buildKindMode(KIND_GROUPED, "impact");
    assert.equal(roots.length, 3, "identical + nearly identical + loosely similar groups; absent kinds omitted");
    const [identicalGroup, nearGroup, looseGroup] = roots as [KindGroupNode, KindGroupNode, KindGroupNode];
    assert.ok(identicalGroup instanceof KindGroupNode);
    assert.equal(identicalGroup.kind, IDENTICAL_KIND);
    assert.equal(nearGroup.kind, NEARLY_IDENTICAL_KIND);
    assert.equal(looseGroup.kind, LOOSELY_SIMILAR_KIND);
    assert.ok(
      labelText(identicalGroup).startsWith(kindTitle(IDENTICAL_KIND)),
      `the group is titled by its kind: ${labelText(identicalGroup)}`,
    );
    const identicalChildren = getGroupNodeChildren(identicalGroup) as ClusterNode[];
    assert.deepEqual(
      identicalChildren.map((node) => node.cluster.id),
      ["bbbbbbb2", "ddddddd4"],
      "the identical group is flat and holds every identical cluster, worst-first",
    );
    const looseChildren = getGroupNodeChildren(looseGroup) as ClusterNode[];
    assert.deepEqual(
      looseChildren.map((node) => node.cluster.id),
      ["ccccccc3"],
      "the loosely similar cluster sits alone in its group",
    );
    assert.ok(
      identicalChildren.every((node) => node instanceof ClusterNode),
      "children are cluster rows directly — no intermediate file/folder layer",
    );
  });

  test("children keep the GLOBAL rank (gaps allowed) and show their file", () => {
    const roots = buildKindMode(KIND_GROUPED, "impact");
    const identicalChildren = getGroupNodeChildren(roots[0] as KindGroupNode);
    assert.equal(identicalChildren.length, 2);
    const child = identicalChildren[1] as ClusterNode;
    assert.ok(child instanceof ClusterNode);
    assert.equal(child.rank, 4, "rank #4 from the global worst-first list, not renumbered to a group-local #2");
    assert.ok(
      labelText(child).includes("d.rs"),
      `kind-group children are roots without a file ancestor, so the file must show: ${labelText(child)}`,
    );
  });

  test("absent kinds never render empty groups", () => {
    const soleNear = KIND_GROUPED[0];
    assert.ok(soleNear, "fixture: the stamped report carries a nearly identical cluster");
    const roots = buildKindMode([soleNear], "impact");
    assert.equal(roots.length, 1);
    assert.equal((roots[0] as KindGroupNode).kind, NEARLY_IDENTICAL_KIND);
  });
});

// [FACET-TESTING] Cross-surface consistency: the Top Offenders tree
// renders exactly the cluster-id set the shared applyFacetFilter slice
// produces — the same function the report webview and status bar use —
// with global rank gaps preserved and the filtered status row leading.
suite("facet filter cross-surface consistency", () => {
  test("filtered tree = shared slice, rank gaps kept, status row leads with clear action", async () => {
    const worstSetting = "topOffenders.filterSeverities";
    const store = storeWith(report(ALL));
    store.setLifecycle({ kind: "ready" });
    const provider = topOffenders(store);

    await withSetting(worstSetting, [WORST_SEVERITY], () => {
      const nodes = provider.getChildren();

      const [statusRow] = nodes;
      assert.ok(statusRow, "the filtered status row must lead the tree");
      assert.equal(
        labelText(statusRow),
        "Filtered: Error — Clear filter",
        "the status row names the active facet with the shared severity label",
      );
      assert.equal(
        statusRow.command?.command,
        "deslop.topOffenders.clearFilter",
        "the clear action is bound on the status row",
      );

      const rows = nodes.filter((node): node is ClusterNode => node instanceof ClusterNode);
      const expected = applyFacetFilter(ALL, { severities: [WORST_SEVERITY] });
      assert.deepEqual(
        rows.map((node) => node.cluster.id),
        expected.map((c) => c.id),
        "tree renders exactly the shared slice's cluster-id set",
      );
      assert.deepEqual(
        rows.map((node) => node.cluster.id),
        [RANK_ONE_ID],
        "the worst band holds only the report's rank #1 cluster",
      );
      assert.deepEqual(
        rows.map((node) => node.rank),
        [1],
        "global ranks render unrenumbered under the worst-band filter",
      );
    });

    
    // rank #1 is absent — a gap proves ranks stay global, never renumbered.
    await withSetting(worstSetting, [WORST_SEVERITY, "warning"], () => {
      const nodes = provider.getChildren();
      const rows = nodes.filter((node): node is ClusterNode => node instanceof ClusterNode);
      const expected = applyFacetFilter(ALL, { severities: [WORST_SEVERITY, "warning"] });
      assert.deepEqual(
        rows.map((node) => node.cluster.id),
        expected.map((c) => c.id),
        "tree renders exactly the shared slice's cluster-id set",
      );
      assert.deepEqual(
        rows.map((node) => node.cluster.id),
        [RANK_ONE_ID, RANK_TWO_ID],
        "widening the band adds exactly the top-10 row",
      );
      assert.deepEqual(
        rows.map((node) => node.rank),
        [1, 2],
        "global ranks keep their order across the widened filter",
      );
    });
  });

  test("a filtered-empty tree shows the status row, never the clean verdict", async () => {
    // A two-cluster report stamps only worst + faint, so a top-10 filter
    // matches nothing — the empty-but-filtered state under test.
    const small = stampRanks([
      cluster("aaaaaaa1", 9, "a.cs", 0, 20, WORST_SEVERITY, 1),
      cluster("ddddddd4", 3, "d.rs", 0, 20, FAINT_SEVERITY, 2),
    ]);
    await withSetting("topOffenders.filterSeverities", ["warning"], () => {
      const store = storeWith(report(small));
      store.setLifecycle({ kind: "ready" });
      const provider = topOffenders(store);
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
