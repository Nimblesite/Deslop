// Unit tests for severity bucketing. Pure function — no VS Code needed.

import * as assert from "node:assert/strict";
import {
  DESLOP_SEVERITY_COLOR,
  SEVERITY_DOT,
  clusterSeverity,
  deslopSeverityOf,
  indexedSeverity,
  rankPercentile,
  resolveSeverity,
  severityForRank,
} from "../../severity";
import {
  BUCKETS,
  Bucket,
  DESLOP_SEVERITIES,
  ReportCluster,
  isActNow,
  severityOf,
} from "../../types/report";
import { signalsWith } from "../signals.helpers";

function cluster(id: string, fused = 0, bucket: Bucket = "identical"): ReportCluster {
  return {
    id,
    weight: 0,
    size: 0,
    canonical_node_count: 0,
    bucket,
    signals: signalsWith("identical", { fused }),
    occurrences: [],
    occurrences_total: 0,
    occurrences_truncated: false,
    summary: "",
    interpretation: "",
  };
}

suite("severity", () => {
  test("rankPercentile handles single-element list", () => {
    assert.equal(rankPercentile(1, 1), 0);
  });

  test("rankPercentile handles empty", () => {
    assert.equal(rankPercentile(1, 0), 0);
  });

  test("rankPercentile rank 1 of N is top", () => {
    assert.equal(rankPercentile(1, 100), 1);
  });

  test("rankPercentile rank N of N is zero", () => {
    assert.equal(rankPercentile(100, 100), 0);
  });

  test("severityForRank worst bucket", () => {
    assert.equal(severityForRank(1, 100), "worst");
  });

  test("severityForRank top10 bucket", () => {
    assert.equal(severityForRank(5, 100), "top10");
  });

  test("severityForRank mid bucket", () => {
    assert.equal(severityForRank(40, 100), "mid");
  });

  test("severityForRank faint bucket", () => {
    assert.equal(severityForRank(80, 100), "faint");
  });

  test("indexedSeverity produces correct buckets", () => {
    const clusters = Array.from({ length: 10 }, (_, i) => cluster(`c${i}`));
    const map = indexedSeverity(clusters);
    assert.equal(map.size, 10);
    assert.equal(map.get("c0"), "worst");
    assert.equal(map.get("c9"), "faint");
  });

  test("indexedSeverity on empty returns empty map", () => {
    assert.equal(indexedSeverity([]).size, 0);
  });

  test("severity never brightens as rank worsens, at any confidence", () => {
    // Whatever severity reads, it must be monotonic down the report: a
    // cluster further from the top can never be painted louder than one
    // above it, or the decoration ordering contradicts the ranking.
    const order = ["worst", "top10", "mid", "faint"] as const;
    const clusters = [
      cluster("a", 1.0),
      cluster("b", 0.3, "structural_only"),
      cluster("c", 0.95),
      cluster("d", 0.31, "structural_only"),
      ...Array.from({ length: 16 }, (_, i) => cluster(`e${i}`, 0.9)),
    ];
    const map = indexedSeverity(clusters);

    assert.equal(map.size, clusters.length, "every cluster must be assigned a severity");
    let previous = 0;
    for (const [index, entry] of clusters.entries()) {
      const severity = map.get(entry.id);
      assert.ok(severity !== undefined, `cluster ${entry.id} must have a severity`);
      const band = severity ?? "faint";
      const position = order.indexOf(band);
      assert.ok(position >= 0, `${entry.id}: severity must be a known band`);
      assert.ok(
        position >= previous,
        `${entry.id} at rank ${index + 1} brightened to ${band} after ${order[previous] ?? "faint"}`,
      );
      previous = position;
    }
    assert.equal(map.get("a"), "worst", "the top-ranked cluster is the loudest");
    assert.equal(
      map.get(`e15`),
      "faint",
      "the last-ranked cluster is the quietest",
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
    const demoted = cluster("shape-giant", 0.31, "structural_only");
    const proven = cluster("proven", 0.95, "identical");
    const clusters = [
      demoted,
      proven,
      ...Array.from({ length: 8 }, (_, i) => cluster(`filler-${i}`, 0.9)),
    ];
    const bands = indexedSeverity(clusters);

    assert.equal(bands.size, 10, "every cluster must be assigned a severity band");
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
      "hint",
      "a cluster the content gate demoted must never be painted the loudest",
    );
    assert.equal(
      clusterSeverity(proven),
      "error",
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
      bands.get("shape-giant"),
      "worst",
      "rank is still rank: the top-ranked cluster keeps the densest glyph",
    );
    assert.equal(
      SEVERITY_DOT[bands.get("shape-giant") ?? "faint"],
      "●●",
      "so the demoted family renders as a muted ●● — high impact, low confidence",
    );
  });

  test("the colour channel is a pure function of the bucket, at every rank", () => {
    // The guarantee that makes D satisfiable and the monotonicity test true
    // at the same time: moving a cluster through the ranking cannot change
    // one byte of its paint.
    for (const bucket of BUCKETS) {
      const target = cluster("target", 0.5, bucket);
      const first = clusterSeverity(target);
      for (const percentile of [0, 0.49, 0.5, 0.9, 0.99, 1]) {
        assert.equal(
          resolveSeverity(bucket, percentile).level,
          first,
          `${bucket} changed colour at percentile ${percentile}`,
        );
        assert.equal(
          resolveSeverity(bucket, percentile).band,
          severityOf(percentile),
          `${bucket} at percentile ${percentile} must keep the percentile band`,
        );
      }
    }
    assert.equal(clusterSeverity(cluster("i", 0, "identical")), "error");
    assert.equal(clusterSeverity(cluster("n", 0, "nearly_identical")), "warning");
    assert.equal(clusterSeverity(cluster("l", 0, "loosely_similar")), "information");
    assert.equal(clusterSeverity(cluster("s", 0, "structural_only")), "hint");
    assert.equal(clusterSeverity(cluster("b", 0, "same_behavior")), "hint");
  });

  test("only act-now buckets may wear an act-now colour", () => {
    // The one-line statement of the defect, so a future remap cannot quietly
    // hand crimson back to a bucket the engine refused to vouch for.
    for (const bucket of BUCKETS) {
      const level = deslopSeverityOf(bucket);
      if (level === "error") {
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
      BUCKETS.filter((bucket) => deslopSeverityOf(bucket) === "error").length,
      1,
      "exactly one bucket — the byte-proven one — earns crimson",
    );
  });
});
