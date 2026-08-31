// Unit tests for severity resolution. Pure functions — no VS Code needed.
//
// Severity is a single channel ([SEVERITY-MODEL], [SEVERITY-COLOR]): the
// engine-stamped mass rank band. Colour, glyph density, and the Deslop
// level are all read off `rank_band` and never re-derived. The band
// *computation* — the rank percentile and its four cut points — is pinned
// where it lives, in `deslop-core::report_weight` (`rank_band_cut_points`,
// `stamp_ranks_numbers_the_whole_report`,
// `rank_band_never_brightens_down_the_report`).

import * as assert from "node:assert/strict";
import {
  DESLOP_SEVERITIES,
  DESLOP_SEVERITY_COLOR,
  SEVERITY_DOT,
  clusterSeverity,
  deslopSeverityOf,
  resolveSeverity,
  type DeslopSeverity,
} from "../../severity";
import { SEVERITIES, clusterBand, type ReportCluster, type Severity } from "../../types/report";
import { wireCluster } from "../cluster.helpers";

const FAINT_BAND: Severity = "faint";
const SEVERITY_LEVEL_COUNT = 4;

function cluster(id: string, band: Severity = FAINT_BAND, rank = 1): ReportCluster {
  return wireCluster({
    id,
    rank,
    rank_band: band,
    mass: rank * 10,
    canonical_node_count: 2,
    occurrences: [],
  });
}

suite("severity", () => {
  test("the glyph band is read off the wire, never re-derived", () => {
    for (const band of SEVERITIES) {
      assert.equal(
        clusterBand(cluster(`c-${band}`, band)),
        band,
        `a cluster the engine banded ${band} must render ${band}`,
      );
    }
  });

  test("a band the engine never stated reads as the quietest, not as a crash", () => {
    const legacy = { ...cluster("legacy"), rank_band: "" };
    assert.equal(
      clusterBand(legacy),
      FAINT_BAND,
      "a report predating the band field must render the tail band",
    );
    const nonsense = { ...cluster("nonsense"), rank_band: "catastrophic" };
    assert.equal(
      clusterBand(nonsense),
      FAINT_BAND,
      "a band outside the known set must not leak into the glyph table",
    );
  });

  // [SEVERITY-DESLOP-MAP] The Deslop level is a pure function of the
  // mass rank band. Every band maps, in rank order, to exactly one level.
  test("deslopSeverityOf maps every band to its level", () => {
    assert.equal(deslopSeverityOf("worst"), "error");
    assert.equal(deslopSeverityOf("top10"), "warning");
    assert.equal(deslopSeverityOf("mid"), "information");
    assert.equal(deslopSeverityOf("faint"), "hint");
    assert.equal(DESLOP_SEVERITIES.length, SEVERITY_LEVEL_COUNT);
  });

  test("clusterSeverity resolves the engine's band to the level", () => {
    assert.equal(clusterSeverity(cluster("worst", "worst")), "error");
    assert.equal(clusterSeverity(cluster("faint", "faint")), "hint");
  });

  test("resolveSeverity returns the band's two channels, unmixed", () => {
    const demoted = cluster("demoted", "worst", 1);
    const resolved = resolveSeverity(demoted);
    assert.equal(resolved.level, "error", "colour is the band's level");
    assert.equal(resolved.band, "worst", "glyph density is the engine's band");
    assert.equal(
      SEVERITY_DOT[resolved.band],
      "●●",
      "the worst band renders the densest glyph",
    );
  });

  // [SEVERITY-COLOR] A cluster's colour can never imply a clone kind:
  // there is no per-bucket severity map, and no wire field a surface
  // could read one from. The cluster type cannot even spell a bucket.
  test("no surface may derive severity from a clone kind", () => {
    const sampleCluster = cluster("proven", "top10", 2) as unknown as Record<string, unknown>;
    assert.equal(
      "bucket" in sampleCluster,
      false,
      "the wire carries no clone-kind field for a severity map to hang off",
    );
    assert.equal(
      "signals" in sampleCluster,
      false,
      "the wire carries no pair signals for a severity map to hang off",
    );
    const anyLevel: DeslopSeverity = deslopSeverityOf("top10");
    assert.ok(
      DESLOP_SEVERITY_COLOR[anyLevel],
      "every level has a colour so no surface falls back to a bucket colour",
    );
  });

  // [SEVERITY-BAND] The two channels are monotonic down the ranking by
  // construction, but they are read, not computed: the fixture bands
  // disagree with rank order on purpose, and the resolver must not
  // second-guess the engine.
  test("severity follows the engine's band, not the rank position", () => {
    const topRanked = cluster("rank-1", "faint", 1);
    const bottomRanked = cluster("rank-9", "worst", 9);
    assert.equal(resolveSeverity(topRanked).level, "hint");
    assert.equal(resolveSeverity(bottomRanked).level, "error");
  });
});
