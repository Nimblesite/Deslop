// Unit tests for severity resolution. Pure functions — no VS Code needed.
//
// Both severity channels are the engine's ([SEVERITY-MODEL],
// [SEVERITY-COLOR], [SEVERITY-BAND]): colour maps the engine's bucket
// label, glyph density reads the engine's `rank_band`. What is asserted
// here is that the client reads them and never re-derives them. The
// band *computation* — the rank percentile and its four cut points — is
// pinned where it lives, in `deslop-core::report_weight`
// (`rank_band_cut_points`, `stamp_ranks_numbers_the_whole_report`,
// `rank_band_never_brightens_down_the_report`).

import * as assert from "node:assert/strict";
import {
  DESLOP_SEVERITY_COLOR,
  SEVERITY_DOT,
  clusterSeverity,
  deslopSeverityOf,
  resolveSeverity,
} from "../../severity";
import {
  BUCKETS,
  Bucket,
  DESLOP_SEVERITIES,
  ReportCluster,
  SEVERITIES,
  Severity,
  clusterBand,
  isActNow,
} from "../../types/report";
import { wireCluster } from "../cluster.helpers";
import { signalsWith } from "../signals.helpers";

const IDENTICAL_BUCKET: Bucket = "identical";
const FAINT_BAND: Severity = "faint";
const HINT_SEVERITY = "hint" as const;
const ERROR_SEVERITY = "error" as const;

function cluster(
  id: string,
  fused = 0,
  bucket: Bucket = IDENTICAL_BUCKET,
  band: Severity = FAINT_BAND,
  rank = 1,
): ReportCluster {
  return wireCluster({
    id,
    rank,
    rank_band: band,
    weight: 0,
    size: 0,
    canonical_node_count: 0,
    bucket,
    signals: signalsWith(IDENTICAL_BUCKET, { fused }),
    occurrences: [],
  });
}

suite("severity", () => {
  test("the glyph band is read off the wire, never re-derived", () => {
    for (const band of SEVERITIES) {
      assert.equal(
        clusterBand(cluster(`c-${band}`, 1, IDENTICAL_BUCKET, band)),
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

  test("resolveSeverity returns the engine's two channels, unmixed", () => {
    const demoted = cluster("demoted", 0.31, "structural_only", "worst");
    const resolved = resolveSeverity(demoted);
    assert.equal(resolved.level, HINT_SEVERITY, "colour is the bucket's, not the rank's");
    assert.equal(resolved.band, "worst", "glyph density is the engine's band");
    assert.equal(
      SEVERITY_DOT[resolved.band],
      "●●",
      "so a demoted family renders as a muted ●● — high impact, low confidence",
    );
  });

  // DEFECT D — restored, and moved onto the channel [SEVERITY-COLOR] gives
  // it. The complaint was always right: a large shape-only family that sorts
  // first was painted the loudest thing in the editor while the engine said
  // "verify before extracting". The mechanism was wrong. D demanded the
  // *percentile band* answer for the bucket, and the band cannot: it is
  // monotonic down the ranking by construction, so painting rank 1 faint
  // paints every cluster below it faint too, which D's own `notEqual`
  // forbids. That contradiction is not a product decision — it is a category
  // error, the same one `classifyCluster` made, and the spec already resolved
  // it. Colour carries the bucket; glyph density carries the percentile; the
  // two are orthogonal ([FUSION-CONTENT-GATE], #344).
  test("a demoted shape-only family is not painted with act-now severity", () => {
    // The engine ranks the demoted family first and bands it `worst`
    // accordingly — rank 1 of 10 sits at the top of the percentile.
    const demoted = cluster("shape-giant", 0.31, "structural_only", "worst", 1);
    const proven = cluster("proven", 0.95, "identical", "top10", 2);
    const clusters = [
      demoted,
      proven,
      ...Array.from({ length: 8 }, (_, i) =>
        cluster(`filler-${i}`, 0.9, IDENTICAL_BUCKET, FAINT_BAND, i + 3),
      ),
    ];

    assert.equal(
      clusters[0]?.bucket,
      "structural_only",
      "fixture: the demoted family is the top-ranked cluster",
    );
    assert.ok(
      (clusters[0]?.signals.fused ?? 1) < (clusters[1]?.signals.fused ?? 0),
      "fixture: it also carries strictly lower confidence than the proven clone",
    );

    // The paint. This is the assertion the defect was about.
    assert.equal(
      clusterSeverity(demoted),
      HINT_SEVERITY,
      "a cluster the content gate demoted must never be painted the loudest",
    );
    assert.equal(
      clusterSeverity(proven),
      ERROR_SEVERITY,
      "a byte-proven clone keeps the loudest paint",
    );
    assert.notEqual(
      clusterSeverity(demoted),
      clusterSeverity(proven),
      "a demoted family and a proven clone must not share a severity",
    );
    assert.equal(
      DESLOP_SEVERITY_COLOR[clusterSeverity(demoted)],
      DESLOP_SEVERITY_COLOR.hint,
      "and the demoted family resolves to the muted colour token, not crimson",
    );
    assert.notEqual(
      DESLOP_SEVERITY_COLOR[clusterSeverity(demoted)],
      DESLOP_SEVERITY_COLOR[clusterSeverity(proven)],
      "the two must not resolve to the same colour token either",
    );

    // The orthogonality that makes both facts survivable at once. The
    // demoted family really is the biggest offender by weight, so it keeps
    // the densest glyph — it is loud about *size* and quiet about *kind*.
    // This is [SEVERITY-COLOR]'s own worked example, inverted.
    assert.equal(
      clusterBand(demoted),
      "worst",
      "rank is still rank: the top-ranked cluster keeps the densest glyph",
    );
    assert.equal(
      SEVERITY_DOT[clusterBand(demoted)],
      "●●",
      "so the demoted family renders as a muted ●● — high impact, low confidence",
    );
  });

  test("the colour channel is a pure function of the bucket, at every band", () => {
    // The guarantee that makes D satisfiable and the monotonicity contract
    // true at the same time: moving a cluster through the ranking cannot
    // change one byte of its paint.
    for (const bucket of BUCKETS) {
      const first = clusterSeverity(cluster("target", 0.5, bucket));
      for (const band of SEVERITIES) {
        const ranked = cluster("target", 0.5, bucket, band);
        assert.equal(
          resolveSeverity(ranked).level,
          first,
          `${bucket} changed colour at band ${band}`,
        );
        assert.equal(
          resolveSeverity(ranked).band,
          band,
          `${bucket} at band ${band} must keep the engine's band`,
        );
      }
    }
    assert.equal(clusterSeverity(cluster("i", 0, IDENTICAL_BUCKET)), ERROR_SEVERITY);
    assert.equal(clusterSeverity(cluster("n", 0, "nearly_identical")), "warning");
    assert.equal(clusterSeverity(cluster("l", 0, "loosely_similar")), "information");
    assert.equal(clusterSeverity(cluster("s", 0, "structural_only")), HINT_SEVERITY);
    assert.equal(clusterSeverity(cluster("b", 0, "same_behavior")), HINT_SEVERITY);
  });

  test("only act-now buckets may wear an act-now colour", () => {
    // The one-line statement of the defect, so a future remap cannot quietly
    // hand crimson back to a bucket the engine refused to vouch for.
    for (const bucket of BUCKETS) {
      const level = deslopSeverityOf(bucket);
      if (level === ERROR_SEVERITY) {
        assert.ok(
          isActNow(bucket),
          `${bucket} resolves to the loudest paint but the engine does not call it actionable`,
        );
      }
      assert.ok(
        DESLOP_SEVERITIES.includes(level),
        `${bucket} must resolve to a known level`,
      );
    }
    assert.equal(
      BUCKETS.filter((bucket) => deslopSeverityOf(bucket) === ERROR_SEVERITY).length,
      1,
      "exactly one bucket — the byte-proven one — earns crimson",
    );
  });
});
