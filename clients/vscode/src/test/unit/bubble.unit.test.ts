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
  test("inlineText includes the severity dot, verdict, authoritative count, and filename", () => {
    const text = inlineText(cluster(), "worst");
    assert.match(text, /×\s*4/);
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
    assert.match(text, /×\s*4/);
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

  // Audience: HUMAN. Issue #30. The live bubble is the editor-visible
  // tooltip humans read while coding. The bold bucket label at the
  // start of the title must be the plain human name
  // ("Identical code", "Nearly identical code", ...), never the
  // `hybridTitle` variant that appends academic taxonomy tags
  // (`[Type-1/2]`, `[Type-3]`, `[Type-4, AI match]`, `[weak LSH]`).
  //
  // Assertion is phrased as a prefix so it is compatible with #32: if
  // the bubble later drops the em-dash + interpretation follow-on, the
  // first line becomes `**Identical code**` alone and this still
  // passes.
  test("bubbleHover bucket label in the title is the plain human name (#30)", () => {
    const c = cluster();
    c.signals = { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 };
    const text = bubbleHover(c).value;
    const firstLine = text.split("\n")[0] ?? "";
    assert.match(
      firstLine,
      /^\*\*Identical code\*\*/,
      `human title must open with the plain bucket label; got first line: ${firstLine}`,
    );
  });

  // Audience: HUMAN. Issue #31. The hover body humans read must be
  // short and prose-only: one interpretation sentence plus the action
  // links. No raw numeric signal strip in the body — the inline
  // decoration already carries the compact bar strip for at-a-glance
  // confidence.
  test("bubbleHover body is one prose sentence plus the action links (#31)", () => {
    const c = cluster();
    c.interpretation = "Safe to extract — every copy is the same.";
    const text = bubbleHover(c).value;
    assert.match(
      text,
      /Safe to extract — every copy is the same\./,
      `human body must carry the interpretation sentence: ${text}`,
    );
    const paragraphs = text
      .split(/\n\s*\n/)
      .map((p) => p.trim())
      .filter((p) => p.length > 0);
    assert.equal(
      paragraphs.length,
      2,
      `human body must be exactly two paragraphs (title+interpretation, then action links); got ${paragraphs.length} in: ${text}`,
    );
  });

  // Audience: HUMAN. Issue #32. The LSP hover and the VSCode bubble
  // hover both fire for the same cursor position and VS Code stacks
  // their markdown — so whichever sentence both render ends up
  // duplicated on-screen. The LSP hover already carries the full
  // interpretation ("<bucket> — <action> N occurrences."); the bubble
  // must stop at the bucket label so the stacked hovers complement
  // rather than repeat each other.
  test("bubbleHover title is just the bucket label so the LSP hover owns the interpretation (#32)", () => {
    const c = cluster();
    c.interpretation = "Safe to extract — every copy is the same.";
    const text = bubbleHover(c).value;
    const firstLine = text.split("\n")[0] ?? "";
    assert.match(
      firstLine,
      /^\*\*[A-Z][A-Za-z ]+\*\*\s*$/,
      `bubble title must be only the bold bucket label; got: ${firstLine}`,
    );
  });
});
