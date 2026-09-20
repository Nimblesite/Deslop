import * as assert from "node:assert/strict";
import { watchDiagnosticPresentation } from "../../diagnosticPresentation";
import { applyFacetFilter, INFORMATIONAL_FINDING } from "../../types/report";
import { ClusterNode } from "../../tree/nodes";
import { wireCluster, occurrence } from "../cluster.helpers";
import { seededStore } from "./report-store.helpers";
import { withSetting, tooltipText } from "./tree.helpers";

const CLONE_ID = "clone";
const SHAPE_ID = "shape";
const FILE = "src/fixture.rs";
const CLONE_MASS = 42;
const CLONE_RANK = 3;
const SEVERITY_SETTING = "diagnostics.severityByKind";
const MASTER_SETTING = "diagnostics.enabled";

suite("live diagnostic presentation", () => {
  // [SEVERITY-MODEL] Workspace overrides refresh all report consumers without analysis.
  test("severity facets update while diagnostics remain off", async () => {
    const clone = wireCluster({ id: CLONE_ID, occurrences: [occurrence(FILE)], severity: "warning", mass: CLONE_MASS, rank: CLONE_RANK });
    const shape = wireCluster({ id: SHAPE_ID, occurrences: [occurrence(FILE)], kind: "structural_only", severity: "none", mass: 0, rank: 0 });
    const store = seededStore([clone, shape]);
    const watcher = watchDiagnosticPresentation(store);
    const revision = store.current.revision;
    try {
      await withSetting(MASTER_SETTING, false, async () => {
        await withSetting(SEVERITY_SETTING, { nearly_identical: "error", structural_only: "information" }, () => {
          const visible = store.current.visibleReport?.clusters ?? [];
          assert.deepEqual(applyFacetFilter(visible, { severities: ["error"] }).map((cluster) => cluster.id), [CLONE_ID]);
          assert.deepEqual(applyFacetFilter(visible, { severities: ["information"] }).map((cluster) => cluster.id), [SHAPE_ID]);
          assert.deepEqual(store.current.report?.clusters, [clone, shape]);
          assert.equal(store.current.revision, revision);
        });
      });
    } finally {
      watcher.dispose();
      store.dispose();
    }
  });

  // [CLONE-TYPE-TAXONOMY] Shape similarity has no duplication mass or rank.
  test("shape-only tree rows are explicitly informational", () => {
    const shape = wireCluster({ id: SHAPE_ID, occurrences: [occurrence(FILE)], kind: "structural_only", severity: "none", mass: 0, rank: 0 });
    const node = new ClusterNode(shape);
    assert.equal(node.description, INFORMATIONAL_FINDING);
    const tooltip = tooltipText(node);
    assert.ok(tooltip.includes(INFORMATIONAL_FINDING));
    for (const claim of ["mass:", "rank #", "copies:"]) assert.equal(tooltip.includes(claim), false, claim);
  });
});
