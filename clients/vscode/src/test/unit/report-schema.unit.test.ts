// Unit tests for the report-schema pure helpers. The cluster wire model
// carries cluster facts and mass only ([REPORTING-CONTEXT],
// [RANK-MASS-SUM]): no clone-kind classification, no pair signals, no
// interpretation, no language. Every assertion here mirrors one clause
// of that contract.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as reportModule from "../../types/report";
import {
  SEVERITIES,
  Severity,
  applyFacetFilter,
  clusterBand,
  clusterMass,
  clusterSlug,
  occurrenceCount,
  sanitizeFacetFilter,
  severityLabel,
  type ReportCluster,
} from "../../types/report";
import { occurrence, wireCluster, type ClusterFixture } from "../cluster.helpers";

const UTF8_ENCODING = "utf8";
const FIXTURE_TEN = 10;
const FIXTURE_FORTY = 40;
const PAIR_OCCURRENCE_COUNT = 2;
const SEVERITY_COUNT = 4;
const WIRE_CLUSTER_FIELDS = [
  "id",
  "rank",
  "rank_band",
  "mass",
  "canonical_node_count",
  "occurrences",
  "occurrences_total",
  "occurrence_count",
  "occurrences_truncated",
] as const;

// A two-occurrence cluster whose mass is the fixture formula
// (canonical nodes × (occurrences − 1)), staged through the shared
// wireCluster helper so every suite agrees on the fixture contract.
function clusterWith(overrides: Partial<ClusterFixture> = {}): ReportCluster {
  return wireCluster({
    id: "a1b2c3d4e5f67890",
    rank: 1,
    rank_band: "worst",
    mass: FIXTURE_FORTY,
    canonical_node_count: FIXTURE_TEN,
    occurrences: [
      occurrence("/repo/A.cs", 0, FIXTURE_TEN),
      occurrence("/repo/B.cs", FIXTURE_TEN, FIXTURE_FORTY),
    ],
    occurrences_total: PAIR_OCCURRENCE_COUNT,
    occurrence_count: PAIR_OCCURRENCE_COUNT,
    ...overrides,
  });
}

function reportTypesPath(): string {
  const compiledRun = path.resolve(__dirname, "../../../src/types/report.ts");
  if (fs.existsSync(compiledRun)) {
    return compiledRun;
  }
  return path.resolve(__dirname, "../../types/report.ts");
}

function reportTypesSource(): string {
  return fs.readFileSync(reportTypesPath(), UTF8_ENCODING);
}

suite("report schema helpers", () => {
  // The severity cut points were once client constants; the assertions
  // that pinned their values moved with them to
  // `deslop-core::report_weight::rank_band` and its
  // `rank_band_cut_points` test. The fused cutoff has a different fate:
  // it is deleted outright — from the engine, the wire, and this client.
  // The tests below pin that no copy of either survived.
  test("the client owns neither a fused cutoff nor the severity cut points", () => {
    assert.ok(
      !("FUSED_THRESHOLD" in reportModule),
      "the reportable-confidence cutoff must exist only in the engine",
    );
    assert.ok(
      !("severityOf" in reportModule),
      "the severity cut points must exist only in the engine",
    );
    assert.ok(
      !("rankPercentile" in reportModule),
      "the rank percentile must exist only in the engine",
    );
    assert.ok(
      !("resolveBucket" in reportModule),
      "the clone-kind routing table must exist only in the engine",
    );
    assert.ok(
      !("bucketLabels" in reportModule),
      "clone-kind labels must not be spelled in the client",
    );
  });

  // The wire contract itself: the generated types are the single source
  // the extension compiles against. If a fused field ever reappears on
  // them, every admission surface regains a threshold to argue with —
  // the exact defect this cutover removed.
  test("the generated wire types carry no fused field on clusters", () => {
    const source = reportTypesSource();
    assert.doesNotMatch(source, /\bfused\b/, "no fused on the wire types");
    assert.doesNotMatch(source, /\bmeets_fused_gate\b/, "no gate flag on the wire types");
    const generated = fs.readFileSync(
      path.resolve(__dirname, "../../../src/types/wire-generated.ts"),
      UTF8_ENCODING,
    );
    assert.doesNotMatch(generated, /\bfused\b/, "no fused in the generated wire model");
    assert.doesNotMatch(
      generated,
      /\bmeets_fused_gate\b/,
      "no gate flag in the generated wire model",
    );
  });

  // [REPORTING-CONTEXT] A cluster surface renders cluster facts and mass
  // only. The cluster type must not be able to spell a bucket, pair
  // signals, an interpretation, or a language — the fields the old
  // surfaces re-derived or quoted.
  test("a wire cluster carries cluster facts and mass only", () => {
    const cluster = clusterWith();
    for (const field of WIRE_CLUSTER_FIELDS) {
      assert.ok(field in cluster, `wire cluster must carry ${field}`);
    }
    const retired = [
      "bucket",
      "signals",
      "signal_source",
      "evidence_verdict",
      "summary",
      "interpretation",
      "language",
      "weight",
      "size",
    ] as const;
    for (const field of retired) {
      assert.equal(field in cluster, false, `wire cluster must not carry ${field}`);
    }
  });

  // Every occurrence on the wire carries line bounds; the extension's
  // decoration surfaces resolve byte ranges against the editor only
  // after the pair is verified, and a fixture without lines cannot.
  test("every occurrence carries start_line and end_line", () => {
    const cluster = clusterWith();
    assert.equal(cluster.occurrences.length, PAIR_OCCURRENCE_COUNT);
    for (const item of cluster.occurrences) {
      assert.equal(typeof item.start_line, "number", "start_line must be a number");
      assert.equal(typeof item.end_line, "number", "end_line must be a number");
      assert.ok(item.end_line >= item.start_line, "end_line must not precede start_line");
    }
  });

  // [RANK-MASS-SUM] The duplicated mass is the worst-first ranking
  // metric; the client carries the engine's value verbatim.
  test("clusterMass carries the engine's mass verbatim", () => {
    assert.equal(clusterMass(clusterWith()), FIXTURE_FORTY);
    assert.equal(clusterMass(clusterWith({ mass: 1 })), 1);
  });

  // [SEVERITY-BAND] The band classifies the rank percentile. The engine
  // stamps it; an empty string (a report written before the field
  // existed) reads as the tail band.
  test("clusterBand resolves the engine's rank_band, defaulting to faint", () => {
    assert.equal(clusterBand(clusterWith({ rank_band: "worst" })), "worst");
    assert.equal(clusterBand(clusterWith({ rank_band: "top10" })), "top10");
    assert.equal(clusterBand(clusterWith({ rank_band: "mid" })), "mid");
    assert.equal(clusterBand(clusterWith({ rank_band: "faint" })), "faint");
    const legacy = clusterWith({ rank_band: "" as Severity });
    assert.equal(clusterBand(legacy), "faint", "a legacy empty band reads as faint");
  });

  // [SEVERITY-BAND] Every severity level in rank order, with a human
  // label shared by every filter surface.
  test("SEVERITIES is the complete rank-ordered band list", () => {
    assert.deepEqual(SEVERITIES, ["worst", "top10", "mid", "faint"]);
    assert.equal(SEVERITIES.length, SEVERITY_COUNT);
    assert.equal(severityLabel("worst"), "Worst 1%");
    assert.equal(severityLabel("top10"), "Top 10%");
    assert.equal(severityLabel("mid"), "Mid 40%");
    assert.equal(severityLabel("faint"), "Faint");
  });

  // [VSIX-CLUSTER-ID-CONSISTENCY] The stable display slug is the first
  // seven hex chars of the id — the same identity on every surface.
  test("clusterSlug is the id prefix shared across surfaces", () => {
    assert.equal(clusterSlug(clusterWith()), "a1b2c3d");
    assert.equal(clusterSlug(clusterWith({ id: "1234567abcd" })), "1234567");
  });

  // There is one occurrence-counting formula and it lives in Rust; the
  // client carries the engine's count verbatim ([RANK-SCORE]).
  test("occurrenceCount carries the engine's count verbatim", () => {
    assert.equal(occurrenceCount(clusterWith()), PAIR_OCCURRENCE_COUNT);
    assert.equal(
      occurrenceCount(clusterWith({ occurrence_count: FIXTURE_TEN })),
      FIXTURE_TEN,
    );
  });

  // [FACET-TOP-OFFENDERS-FILTER] Facets filter on the mass severity band
  // only; a bad persisted value must never yield an empty tree.
  test("sanitizeFacetFilter keeps only known severity bands", () => {
    assert.deepEqual(sanitizeFacetFilter(["worst", "top10"]), { severities: ["worst", "top10"] });
    assert.deepEqual(sanitizeFacetFilter([]), { severities: [] });
    assert.deepEqual(
      sanitizeFacetFilter(["identical", "worst", "banana"]),
      { severities: ["worst"] },
      "clone-kind values and typos are dropped, not kept",
    );
  });

  // [FACET-TOP-OFFENDERS-FILTER] An empty value list means "show all";
  // a non-empty list shows exactly the bands it names.
  test("applyFacetFilter slices by severity band only", () => {
    const worst = clusterWith({ id: "1111", rank_band: "worst" });
    const mid = clusterWith({ id: "2222", rank_band: "mid" });
    const faint = clusterWith({ id: "3333", rank_band: "faint" });
    const all = [worst, mid, faint];
    assert.deepEqual(applyFacetFilter(all, { severities: [] }), all);
    assert.deepEqual(applyFacetFilter(all, { severities: ["worst"] }), [worst]);
    assert.deepEqual(applyFacetFilter(all, { severities: ["mid", "faint"] }), [mid, faint]);
    assert.deepEqual(applyFacetFilter(all, { severities: ["top10"] }), []);
  });
});
