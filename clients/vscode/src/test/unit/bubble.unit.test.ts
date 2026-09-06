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
import * as liveBubble from "../../bubble/live";
import { clusterHoverMarkdown } from "../../clusterHover";
import { ReportCluster } from "../../types/report";

function cluster(
  signals = {
    structural: 1,
    token_jaccard: 0.9,
    embedding_cos: 0.5,
    fused: 0.95,
  },
): ReportCluster {
  return {
    id: "abcdef0123456789",
    weight: 3,
    size: 4,
    canonical_node_count: 5,
    bucket: "identical",
    signals,
    occurrences: [
      { path: "/tmp/a/b/Alpha.cs", start_byte: 0, end_byte: 10, hidden: false },
      { path: "/tmp/a/b/Beta.cs", start_byte: 0, end_byte: 10, hidden: false },
    ],
    occurrences_total: 0,
    occurrences_truncated: false,
    summary: "",
    interpretation: "interp",
  };
}

suite("bubble rendering helpers", () => {
  test("inlineText includes the severity dot, bucket label, authoritative count, and filename", () => {
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
      cluster({
        structural: 2,
        token_jaccard: -1,
        embedding_cos: 0.5,
        fused: 1,
      }),
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

  // Audience: HUMAN. Issue #30. The plain human bucket label
  // ("Identical code", "Nearly identical code", …) must be bold in
  // the first line — never the hybridTitle taxonomy variant.
  test("bubbleHover bucket label in the title is the plain human name (#30)", () => {
    const c = cluster();
    c.signals = { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 };
    const text = bubbleHover(c).value;
    const firstLine = text.split("\n")[0] ?? "";
    assert.match(
      firstLine,
      /\*\*[0-9a-f]+ Identical code\*\*/,
      `human title must contain the plain bucket label; got first line: ${firstLine}`,
    );
    assert.doesNotMatch(
      firstLine,
      /Type-\d/,
      `human title must not expose taxonomy Type-N label: ${firstLine}`,
    );
  });

  // Audience: HUMAN. Issues #31/#32. Card layout matches the design:
  // (1) slug + category + count, (2) canonical path, (3) action links.
  test("bubbleHover body shows slug, canonical, count, and three action links (#31/#32)", () => {
    const c = cluster();
    const text = bubbleHover(c).value;
    assert.match(text, /\babcdef0\b/, `slug must appear in the body: ${text}`);
    assert.match(
      text,
      /×\s*4/,
      `instance count must appear in the body: ${text}`,
    );
    assert.match(
      text,
      /Canonical/,
      `canonical section must be present: ${text}`,
    );
    assert.match(text, /Alpha\.cs/, `canonical file must be visible: ${text}`);
    assert.doesNotMatch(
      text,
      /Safe to extract|interpretation/i,
      `bubble must not carry interpretation prose: ${text}`,
    );
    const paragraphs = text
      .split(/\n\s*\n/)
      .map((p) => p.trim())
      .filter((p) => p.length > 0);
    assert.equal(
      paragraphs.length,
      3,
      `body must be three paragraphs (header, canonical, links); got ${paragraphs.length} in: ${text}`,
    );
  });

  // Audience: HUMAN. Issue #32. First line: bold label with the stable
  // slug prefix and instance count. No raw signal scores, no taxonomy
  // tags, no interpretation prose on line 1.
  test("bubbleHover first line carries slug, bold label, and count (#32)", () => {
    const c = cluster();
    const text = bubbleHover(c).value;
    const firstLine = text.split("\n")[0] ?? "";
    assert.match(
      firstLine,
      /\*\*[0-9a-f]+ [A-Z][A-Za-z ]+\*\* ×/,
      `first line must be slug + bold label + count; got: ${firstLine}`,
    );
  });

  test("bubbleHover uses the shared renderer with dismiss option (#46)", () => {
    const c = cluster();
    const bubble = bubbleHover(c);
    const shared = clusterHoverMarkdown(c, { showDismiss: true });
    assert.equal(
      bubble.value,
      shared.value,
      "bubble must not rebuild cluster markdown separately",
    );
    assert.match(bubble.value, /^\*\*abcdef0 [A-Z][A-Za-z, ]+\*\* × 4/);
    assert.match(bubble.value, /Canonical: `.*Alpha\.cs`/);
    assert.match(bubble.value, /command:deslop\.compareWithCanonical/);
    assert.match(bubble.value, /command:deslop\.openCluster/);
    assert.match(bubble.value, /command:deslop\.bubble\.dismissCluster/);
  });

  test("renderBubbleParts is the single rebuild path for live bubble text (#46)", () => {
    const c = cluster();
    type RenderBubbleParts = (
      cluster: ReportCluster,
      severity: Parameters<typeof inlineText>[1],
    ) => {
      inline: string;
      ghost: string;
      signalStrip: string;
      hover: { value: string };
    };
    const renderBubbleParts = (
      liveBubble as typeof liveBubble & {
        renderBubbleParts?: RenderBubbleParts;
      }
    ).renderBubbleParts;
    if (typeof renderBubbleParts !== "function") {
      assert.fail(
        "live bubble surfaces must be rebuilt through one shared render function",
      );
    }
    const parts = renderBubbleParts(c, "top10");
    assert.equal(inlineText(c, "top10"), parts.inline);
    assert.equal(ghostText(c, "top10"), parts.ghost);
    assert.equal(signalStrip(c), parts.signalStrip);
    assert.equal(bubbleHover(c).value, parts.hover.value);
    assert.match(parts.inline, /\babcdef0\b/);
    assert.match(parts.ghost, /\babcdef0\b/);
    assert.match(parts.hover.value, /^\*\*abcdef0 /);
  });

  // Issue #46 follow-up; same defect class as Deslop#149 / Deslop#349.
  // The compact hover (squiggle, alongside diagnostic) was rendering
  // `**#103 **× 3` — rank used as stable id, and a trailing space inside
  // the bold delimiters made markdown leak literal asterisks. Headlines
  // use the cluster's stable slug (first 7 hex chars of cluster.id) and
  // close the bold delimiters cleanly.
  test("compact hover uses stable slug, not rank, and closes bold cleanly", () => {
    const c = cluster();
    c.id = "ab3f9c2def012345";
    const md = clusterHoverMarkdown(c, { showCategory: false });
    const firstLine = md.value.split("\n")[0] ?? "";

    assert.doesNotMatch(
      firstLine,
      /#\d+\b/,
      `rank-as-id is forbidden in compact hover; got: ${firstLine}`,
    );
    assert.doesNotMatch(
      firstLine,
      /\s\*\*/,
      `bold delimiter must not be preceded by whitespace; got: ${firstLine}`,
    );
    assert.match(
      firstLine,
      /\bab3f9c2\b/,
      `stable slug must appear in compact hover; got: ${firstLine}`,
    );
  });

  test("full hover uses stable slug, not rank, in the bold headline", () => {
    const c = cluster();
    c.id = "ab3f9c2def012345";
    const md = clusterHoverMarkdown(c, { showCategory: true });
    const firstLine = md.value.split("\n")[0] ?? "";

    assert.doesNotMatch(
      firstLine,
      /#\d+\b/,
      `rank-as-id is forbidden in full hover; got: ${firstLine}`,
    );
    assert.match(
      firstLine,
      /\bab3f9c2\b/,
      `stable slug must appear in full hover; got: ${firstLine}`,
    );
  });
});
