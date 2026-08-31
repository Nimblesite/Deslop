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

const WORST_SEVERITY = "worst";
const MID_SEVERITY = "mid";
const FAINT_SEVERITY = "faint";

// One cluster per severity band the tests slice on, each carrying the
// global rank the engine stamped on it — worst first.
const worstA = cluster("aaaaaaa1", 9, "a.cs", 0, 20, WORST_SEVERITY, 1);
const worstB = cluster("bbbbbbb2", 7, "b.cs", 0, 20, WORST_SEVERITY, 2);
const midData = cluster("ccccccc3", 5, "c.dart", 0, 20, MID_SEVERITY, 3);
const ALL = [worstA, worstB, midData];

suite("facet filter slice ([FACET-TOP-OFFENDERS-FILTER])", () => {
  test("empty filter shows all clusters", () => {
    const out = applyFacetFilter(ALL, { severities: [] });
    assert.deepEqual(out.map((c) => c.id), ALL.map((c) => c.id));
  });

  test("severity axis keeps only matching clusters", () => {
    const out = applyFacetFilter(ALL, { severities: [WORST_SEVERITY] });
    assert.deepEqual(out.map((c) => c.id), [worstA.id, worstB.id]);
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
    const roots = buildSeverityMode(ALL, "impact");
    assert.equal(roots.length, 2, "worst + mid groups; absent bands omitted");
    const [worstGroup, midGroup] = roots as [SeverityGroupNode, SeverityGroupNode];
    assert.ok(worstGroup instanceof SeverityGroupNode);
    assert.equal(worstGroup.severity, WORST_SEVERITY);
    assert.equal(midGroup.severity, MID_SEVERITY);
    const worstChildren = getGroupNodeChildren(worstGroup) as ClusterNode[];
    assert.deepEqual(
      worstChildren.map((node) => node.cluster.id),
      [worstA.id, worstB.id],
      "both worst clusters share the group, flat",
    );
    assert.ok(
      worstChildren.every((node) => node instanceof ClusterNode),
      "children are cluster rows directly — no intermediate file/folder layer",
    );
  });

  test("children keep the GLOBAL rank (gaps allowed) and show their file", () => {
    const roots = buildSeverityMode(ALL, "impact");
    const worstChildren = getGroupNodeChildren(roots[0] as SeverityGroupNode);
    assert.equal(worstChildren.length, 2);
    const child = worstChildren[1] as ClusterNode;
    assert.ok(child instanceof ClusterNode);
    assert.equal(child.rank, 2, "rank #2 from the global worst-first list, not renumbered");
    assert.ok(
      labelText(child).includes("b.cs"),
      `severity-group children are roots without a file ancestor, so the file must show: ${labelText(child)}`,
    );
  });

  test("absent bands never render empty groups", () => {
    const roots = buildSeverityMode([worstA], "impact");
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
    await withSetting("topOffenders.filterSeverities", [WORST_SEVERITY], () => {
      const store = new ReportStore();
      store.setSnapshot(report(ALL), 0);
      store.setLifecycle({ kind: "ready" });
      const provider = new TopOffendersProvider(store, new StatusTicker());
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
        rows.map((node) => node.rank),
        [1, 2],
        "global ranks keep their gaps — a filtered view shows the worst band",
      );
    });
  });

  test("a filtered-empty tree shows the status row, never the clean verdict", async () => {
    await withSetting("topOffenders.filterSeverities", [FAINT_SEVERITY], () => {
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
