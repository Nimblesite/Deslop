// Unit: pure rendering helpers from bubble/live. Keep them tight so edit
// latency stays inside the 250ms budget.

import * as assert from "node:assert/strict";
import {
  inlineText,
  ghostText,
  signalStrip,
  shortPath,
  bubbleHover,
} from "../../bubble/live";
import { ReportCluster } from "../../types/report";

function cluster(signals = {
  structural: 1,
  token_jaccard: 0.9,
  embedding_cos: 0.5,
  fused: 0.95,
}): ReportCluster {
  return {
    id: "c-1",
    weight: 3,
    size: 4,
    canonical_node_count: 5,
    signals,
    occurrences: [
      { path: "/tmp/a/b/Alpha.cs", start_byte: 0, end_byte: 10, hidden: false },
      { path: "/tmp/a/b/Beta.cs", start_byte: 0, end_byte: 10, hidden: false },
    ],
    summary: "",
    interpretation: "interp",
  };
}

suite("bubble rendering helpers", () => {
  test("inlineText includes the severity dot, verdict, count, and filename", () => {
    const text = inlineText(cluster(), "worst");
    assert.match(text, /×\s*2/);
    assert.match(text, /Alpha\.cs/);
  });

  test("inlineText without occurrences omits the location tail", () => {
    const c = cluster();
    c.occurrences = [];
    const text = inlineText(c, "faint");
    assert.doesNotMatch(text, /Alpha/);
  });

  test("ghostText encodes the signal strip", () => {
    const text = ghostText(cluster(), "top10");
    assert.match(text, /└─/);
    assert.match(text, /×\s*2/);
  });

  test("signalStrip clamps inputs to the bar range", () => {
    const strip = signalStrip(
      cluster({ structural: 2, token_jaccard: -1, embedding_cos: 0.5, fused: 1 }),
    );
    assert.equal(strip.length, 3);
  });

  test("shortPath returns the basename for posix and windows separators", () => {
    assert.equal(shortPath("/a/b/File.cs"), "File.cs");
    assert.equal(shortPath("C:\\a\\b\\File.cs"), "File.cs");
    assert.equal(shortPath("no-separator"), "no-separator");
  });

  test("bubbleHover renders three action links", () => {
    const md = bubbleHover(cluster());
    const text = md.value;
    assert.match(text, /command:deslop.openCluster/);
    assert.match(text, /command:deslop.compareWithCanonical/);
    assert.match(text, /command:deslop.bubble.dismissCluster/);
  });
});
