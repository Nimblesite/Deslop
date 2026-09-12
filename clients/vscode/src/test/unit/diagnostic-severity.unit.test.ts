import * as assert from "node:assert/strict";
import {
  CLUSTER_KINDS,
  SEVERITIES,
  compareClusterRank,
  isClone,
  projectDiagnosticSeverity,
} from "../../types/report";
import { occurrence, wireCluster } from "../cluster.helpers";
import type { ClusterKind, Report, Severity } from "../../types/report";
import { extensionPackage } from "./package.helpers";

const CLONE_ID = "clone";
const INFORMATION_ID = "information";
const MASS = 42;
const RANK = 3;
const FIXTURE_PATH = "src/fixture.rs";
/** [SEVERITY-DIAGNOSTICS-STRUCTURAL-ONLY] The level that publishes nothing. */
const SILENT_LEVEL: Severity = "none";
/** [SEVERITY-DESLOP-MAP] The level a near-copy publishes by default. */
const CLONE_LEVEL: Severity = "warning";
/** [CLONE-BUCKETS-STRUCTURAL-ONLY] The one informational, non-clone category. */
const SHAPE_ONLY_KIND: ClusterKind = "structural_only";
const DUPLICATED_LOC = 12;
const ANALYSED_LOC = 240;
/** A level the engine never defaults shape-only to, so an override is proven. */
const OVERRIDE_LEVEL: Severity = "information";

/** The two settings [SEVERITY-CONFIG] defines, and the scope it requires
 * of them: `window`, so a workspace folder cannot silence its siblings. */
const DIAGNOSTICS_ENABLED_SETTING = "deslop.diagnostics.enabled";
const SEVERITY_BY_KIND_SETTING = "deslop.diagnostics.severityByKind";
const WINDOW_SCOPE = "window";

suite("diagnostic severity presentation", () => {
  test("zero-rank information sorts after ranked clones", () => {
    const clone = wireCluster({ id: CLONE_ID, occurrences: [], severity: CLONE_LEVEL, mass: MASS, rank: RANK });
    const information = wireCluster({ id: INFORMATION_ID, occurrences: [], kind: SHAPE_ONLY_KIND, severity: SILENT_LEVEL, mass: 0, rank: 0 });
    assert.deepEqual([information, clone].sort(compareClusterRank), [clone, information]);
  });
  // [SEVERITY-MODEL] Overrides change presentation only, even with diagnostics off.
  test("explicit levels preserve engine defaults, mass, ranking and source report", () => {
    const clone = wireCluster({ id: CLONE_ID, occurrences: [occurrence(FIXTURE_PATH)], severity: CLONE_LEVEL, mass: MASS, rank: RANK });
    const information = wireCluster({ id: INFORMATION_ID, occurrences: [], kind: SHAPE_ONLY_KIND, severity: SILENT_LEVEL, mass: 0, rank: 0 });
    const report = { clusters: [clone, information] } as Report;
    const projected = projectDiagnosticSeverity(report, { [SHAPE_ONLY_KIND]: OVERRIDE_LEVEL });
    assert.deepEqual(projected?.clusters, [clone, { ...information, severity: OVERRIDE_LEVEL }]);
    assert.equal(report.clusters[1]?.severity, SILENT_LEVEL);
    assert.equal(projected?.clusters[0]?.mass, MASS);
    assert.equal(projected?.clusters[0]?.rank, RANK);
    assert.equal(projectDiagnosticSeverity(report, {}), report);
    assert.equal(projectDiagnosticSeverity(null, {}), null);
  });

  // [CLONE-BUCKETS-STRUCTURAL-ONLY] "Showing it, hiding it or enabling its
  // diagnostics cannot change those figures." A severity override is a
  // presentation choice; it must not promote an informational finding to a
  // clone, nor move one duplication number the engine calculated.
  test("raising shape-only to the loudest level leaves it a non-clone and every metric untouched", () => {
    const clone = wireCluster({
      id: CLONE_ID,
      occurrences: [occurrence(FIXTURE_PATH)],
      severity: CLONE_LEVEL,
      mass: MASS,
      rank: RANK,
    });
    const information = wireCluster({
      id: INFORMATION_ID,
      occurrences: [occurrence(FIXTURE_PATH)],
      kind: SHAPE_ONLY_KIND,
      severity: SILENT_LEVEL,
      mass: 0,
      rank: 0,
    });
    const metrics = { duplicated_loc: DUPLICATED_LOC, analysed_loc: ANALYSED_LOC };
    const report = { clusters: [clone, information], metrics } as unknown as Report;

    for (const level of SEVERITIES) {
      const projected = projectDiagnosticSeverity(report, { [SHAPE_ONLY_KIND]: level });
      const shaped = projected?.clusters.find((cluster) => cluster.id === INFORMATION_ID);
      assert.ok(shaped, `the override must keep the finding visible at ${level}`);
      assert.equal(shaped.severity, level, `the user's explicit ${level} is honoured`);
      assert.equal(shaped.kind, SHAPE_ONLY_KIND, `${level} must not re-categorise the finding`);
      assert.equal(isClone(shaped), false, `${level} must not turn information into a clone`);
      assert.equal(shaped.mass, 0, `${level} must not let information claim duplicated mass`);
      assert.equal(shaped.rank, 0, `${level} must not let information claim a clone rank`);

      const projectedClone = projected?.clusters.find((cluster) => cluster.id === CLONE_ID);
      assert.equal(projectedClone?.severity, CLONE_LEVEL, `${level} on shape-only must not touch the clone`);
      assert.equal(projectedClone?.mass, MASS, `${level} must not move the clone's mass`);
      assert.equal(projectedClone?.rank, RANK, `${level} must not move the clone's rank`);
      assert.deepEqual(projected?.metrics, metrics, `${level} must not move a repository figure`);
      assert.deepEqual(
        [...(projected?.clusters ?? [])].sort(compareClusterRank).map((cluster) => cluster.id),
        [CLONE_ID, INFORMATION_ID],
        `${level} must not reorder the clone ahead of, or behind, the information`,
      );
    }
    assert.equal(report.clusters[1]?.severity, SILENT_LEVEL, "the engine's report is never mutated in place");
  });
});

// The settings manifest is the contract VS Code validates a user's
// `settings.json` against: an unlisted key or level is rejected in the
// editor before the extension ever sees it. These assertions hold the
// manifest to [SEVERITY-CONFIG] so a kind added to the engine cannot
// become unconfigurable, and an invented level cannot become settable.
suite("diagnostic settings manifest", () => {
  test("the master switch ships off, so Deslop publishes nothing until the user opts in", () => {
    // [SEVERITY-DIAGNOSTICS-GATE] `deslop.diagnostics.enabled` defaults to false.
    const property = extensionPackage().contributes.configuration.properties[DIAGNOSTICS_ENABLED_SETTING];
    assert.ok(property, `${DIAGNOSTICS_ENABLED_SETTING} must be contributed`);
    assert.equal(property.type, "boolean", "the master switch is a boolean");
    assert.equal(property.default, false, "diagnostics are opt-in");
  });

  test("severity is configurable for exactly the five categories, at exactly the five levels", () => {
    // [SEVERITY-CONFIG] "Accept only these five kind keys and the levels
    // none, hint, information, warning, error. Reject unknown keys or levels."
    const property = extensionPackage().contributes.configuration.properties[SEVERITY_BY_KIND_SETTING];
    assert.ok(property, `${SEVERITY_BY_KIND_SETTING} must be contributed`);
    assert.equal(property.type, "object", "the override map is an object");
    assert.deepEqual(property.default, {}, "no category is overridden out of the box");
    assert.equal(
      property.additionalProperties,
      false,
      "an unknown kind key must be rejected by the editor, not silently ignored",
    );

    const kinds = property.properties ?? {};
    assert.deepEqual(
      Object.keys(kinds).sort(),
      [...CLUSTER_KINDS].sort(),
      "every clone kind the extension knows must be configurable, and no other key",
    );
    for (const kind of CLUSTER_KINDS) {
      const entry = kinds[kind];
      assert.ok(entry, `${kind} must be configurable`);
      assert.equal(entry.type, "string", `${kind} takes a level name`);
      assert.deepEqual(
        [...(entry.enum ?? [])].map(String).sort(),
        [...SEVERITIES].sort(),
        `${kind} must offer exactly the five diagnostic levels`,
      );
      assert.ok(
        (entry.enum ?? []).includes(SILENT_LEVEL),
        `${kind} must be silenceable with ${SILENT_LEVEL}`,
      );
    }
  });

  test("both diagnostic settings are window-scoped, so one workspace cannot silence another", () => {
    // [SEVERITY-CONFIG] User/workspace precedence applies per entry.
    const properties = extensionPackage().contributes.configuration.properties;
    for (const setting of [DIAGNOSTICS_ENABLED_SETTING, SEVERITY_BY_KIND_SETTING]) {
      assert.equal(properties[setting]?.scope, WINDOW_SCOPE, `${setting} must be ${WINDOW_SCOPE}-scoped`);
    }
  });
});
