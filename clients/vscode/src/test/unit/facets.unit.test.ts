// Unit: the facet model ([FACET-MODEL] / [FACET-TOP-OFFENDERS-FILTER] /
// [FACET-GROUP-BY-SEVERITY]). Covers the shared filter slice every
// listing surface funnels through, the sanitizer's typo fallback, and the
// severity-grouping mode's flat band roots (#258, re-stated on severity).

import * as assert from "node:assert/strict";

import {
  applyFacetFilter,
  sanitizeFacetFilter,
} from "../../types/report";
import { buildSeverityMode, getGroupNodeChildren } from "../../tree/grouping";
import { ClusterNode, SeverityGroupNode } from "../../tree/nodes";
import { StatusTicker, TopOffendersProvider } from "../../tree/providers";
import { ReportStore } from "../../reportStore";
import { cluster, labelText, report, withSetting } from "./tree.helpers";
import { stampRanks } from "../cluster.helpers";

const WORST_SEVERITY = "worst";
const MID_SEVERITY = "mid";
const FAINT_SEVERITY = "faint";

// A twenty-cluster report: the engine's stamping gives rank 1 the worst
// band, rank 2 the top-10 band, ranks 3–10 mid, and 11–20 faint — enough
// distinct bands to prove a filtered view keeps global rank gaps.
const STAMPED_COUNT = 20;
const ALL = stampRanks(
  Array.from({ length: STAMPED_COUNT }, (_, index) =>
    cluster(`cluster${String(index + 1).padStart(2, "0")}`, STAMPED_COUNT - index, `f${index + 1}.cs`, 0, 20, MID_SEVERITY, index + 1),
  ),
);
const RANK_ONE_ID = "cluster01";
const RANK_TWO_ID = "cluster02";

// A small report for the grouping-mode suite, with the same engine
// stamping: bands come out worst / mid / mid / faint for ranks 1–4.
const GROUPED = stampRanks([
  cluster("aaaaaaa1", 9, "a.cs", 0, 20, WORST_SEVERITY, 1),
  cluster("bbbbbbb2", 7, "b.cs", 0, 20, MID_SEVERITY, 2),
  cluster("ccccccc3", 5, "c.dart", 0, 20, MID_SEVERITY, 3),
  cluster("ddddddd4", 3, "d.rs", 0, 20, FAINT_SEVERITY, 4),
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
      out.map((c) => c.rank_band),
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

suite("severity grouping mode ([FACET-GROUP-BY-SEVERITY])", () => {
  // #258: severity mode groups by BAND — every worst cluster surfaces
  // together in one flat group, with no file/folder sub-grouping in
  // between ([FACET-GROUP-BY-SEVERITY]).
  test("roots are one flat group per band present, so all worst clusters sit together", () => {
    const roots = buildSeverityMode(GROUPED, "impact");
    assert.equal(roots.length, 3, "worst + mid + faint groups; absent bands omitted");
    const [worstGroup, midGroup, faintGroup] = roots as [
      SeverityGroupNode,
      SeverityGroupNode,
      SeverityGroupNode,
    ];
    assert.ok(worstGroup instanceof SeverityGroupNode);
    assert.equal(worstGroup.severity, WORST_SEVERITY);
    assert.equal(midGroup.severity, MID_SEVERITY);
    assert.equal(faintGroup.severity, FAINT_SEVERITY);
    const worstChildren = getGroupNodeChildren(worstGroup) as ClusterNode[];
    assert.deepEqual(
      worstChildren.map((node) => node.cluster.id),
      ["aaaaaaa1"],
      "the worst group is flat and holds every worst-band cluster",
    );
    const faintChildren = getGroupNodeChildren(faintGroup) as ClusterNode[];
    assert.deepEqual(
      faintChildren.map((node) => node.cluster.id),
      ["ccccccc3", "ddddddd4"],
      "both faint clusters share the faint group, flat",
    );
    assert.ok(
      worstChildren.every((node) => node instanceof ClusterNode),
      "children are cluster rows directly — no intermediate file/folder layer",
    );
  });

  test("children keep the GLOBAL rank (gaps allowed) and show their file", () => {
    const roots = buildSeverityMode(GROUPED, "impact");
    const faintChildren = getGroupNodeChildren(roots[2] as SeverityGroupNode);
    assert.equal(faintChildren.length, 2);
    const child = faintChildren[0] as ClusterNode;
    assert.ok(child instanceof ClusterNode);
    assert.equal(child.rank, 3, "rank #3 from the global worst-first list, not renumbered to a group-local #1");
    assert.ok(
      labelText(child).includes("c.dart"),
      `severity-group children are roots without a file ancestor, so the file must show: ${labelText(child)}`,
    );
  });

  test("absent bands never render empty groups", () => {
    const soleWorst = GROUPED[0];
    assert.ok(soleWorst, "fixture: the stamped report carries a worst-band cluster");
    const roots = buildSeverityMode([soleWorst], "impact");
    assert.equal(roots.length, 1);
    assert.equal((roots[0] as SeverityGroupNode).severity, WORST_SEVERITY);
  });
});

// [FACET-TESTING] Cross-surface consistency: the Top Offenders tree
// renders exactly the cluster-id set the shared applyFacetFilter slice
// produces — the same function the report webview and status bar use —
// with global rank gaps preserved and the filtered status row leading.
suite("facet filter cross-surface consistency", () => {
  test("filtered tree = shared slice, rank gaps kept, status row leads with clear action", async () => {
    const worstSetting = "topOffenders.filterSeverities";
    const store = new ReportStore();
    store.setSnapshot(report(ALL), 0);
    store.setLifecycle({ kind: "ready" });
    const provider = new TopOffendersProvider(store, new StatusTicker());

    await withSetting(worstSetting, [WORST_SEVERITY], () => {
      const nodes = provider.getChildren();

      const [statusRow] = nodes;
      assert.ok(statusRow, "the filtered status row must lead the tree");
      assert.equal(
        labelText(statusRow),
        "Filtered: Worst 1% — Clear filter",
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

    // Widening the filter to the top-10 band must surface rank #2 while
    // rank #1 is absent — a gap proves ranks stay global, never renumbered.
    await withSetting(worstSetting, [WORST_SEVERITY, "top10"], () => {
      const nodes = provider.getChildren();
      const rows = nodes.filter((node): node is ClusterNode => node instanceof ClusterNode);
      const expected = applyFacetFilter(ALL, { severities: [WORST_SEVERITY, "top10"] });
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
    await withSetting("topOffenders.filterSeverities", ["top10"], () => {
      const store = new ReportStore();
      store.setSnapshot(report(small), 0);
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
