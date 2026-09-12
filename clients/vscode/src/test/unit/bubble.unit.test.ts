// Unit: pure rendering helpers from bubble/live. Keep them tight so edit
// latency stays inside the 250ms budget.

import * as assert from "node:assert/strict";
import {
  inlineText,
  ghostText,
  shortPath,
  bubbleHover,
} from "../../bubble/live";
import * as liveBubble from "../../bubble/live";
import { clusterHoverMarkdown } from "../../clusterHover";
import { kindTitle, ReportCluster } from "../../types/report";
import { FIXTURE_KIND, occurrence, wireCluster } from "../cluster.helpers";

const IDENTICAL_KIND = "identical";

function cluster(): ReportCluster {
  return wireCluster({
    id: "abcdef0123456789",
    mass: 3,
    canonical_node_count: 4,
    occurrences: [
      occurrence("/tmp/a/b/Alpha.cs", 0, 10),
      occurrence("/tmp/a/b/Beta.cs", 0, 10),
    ],
    occurrence_count: 4,
  });
}

suite("bubble rendering helpers", () => {
  test("inlineText includes the count and filename", () => {
    const text = inlineText(cluster(), "error");
    assert.match(text, /×\s*4/);
    assert.match(text, /Alpha\.cs/);
  });

  test("inlineText without occurrences omits the location tail", () => {
    const c = cluster();
    c.occurrences = [];
    const text = inlineText(c, "hint");
    assert.doesNotMatch(text, /Alpha/);
  });

  test("ghostText encodes the tree-branch prefix and count", () => {
    const text = ghostText(cluster(), "warning");
    assert.match(text, /└─/);
    assert.match(text, /×\s*4/);
  });

  test("the ghost line renders no pair evidence for any cluster", () => {
    // [FUSED-PAIR-SIGNALS] Admission signals (structural, token, embedding,
    // content similarity) are pair-only and never touch a cluster surface.
    // The ghost line carries the verdict, slug and count — never the former
    // `pair 1↔2` bar strip.
    const c = cluster();
    const ghost = ghostText(c, "warning");
    assert.equal(ghost.includes("pair"), false, "no pair label may render");
    assert.equal(ghost.includes("↔"), false, "no pair separator may render");
    assert.match(ghost, /└─/);
    assert.match(ghost, /×\s*4/);
    for (const glyph of ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"]) {
      assert.equal(
        ghost.includes(glyph),
        false,
        `signal bar glyph ${glyph} must not render on a cluster surface`,
      );
    }
  });

  test("no cluster renders pair scores", () => {
    // The former strip showed the elected pair's bars when a source was
    // named and nothing otherwise. Pair evidence now renders on no cluster
    // surface at all, so both arms assert the same absence.
    const sourced = cluster();
    const unsourced = cluster();
    for (const [context, c] of [
      ["sourced cluster", sourced],
      ["unsourced cluster", unsourced],
    ] as const) {
      assert.equal(ghostText(c, "warning").includes("pair"), false, context);
      assert.equal(
        ghostText(c, "warning").includes("█"),
        false,
        `${context}: no bar glyph`,
      );
      assert.equal(
        inlineText(c, "warning").includes("pair"),
        false,
        `${context}: inline line carries no pair label`,
      );
    }
  });

  test("shortPath returns the basename for posix and windows separators", () => {
    assert.equal(shortPath("/a/b/File.cs"), "File.cs");
    assert.equal(shortPath("C:\\a\\b\\File.cs"), "File.cs");
    assert.equal(shortPath("no-separator"), "no-separator");
  });

  test("bubbleHover links open and dismiss, never an implicit compare", () => {
    const md = bubbleHover(cluster());
    const text = md.value;
    assert.match(text, /command:deslop.openCluster/);
    assert.match(text, /command:deslop.bubble.dismissCluster/);
    // [VSIX-PAIR-COMPARE] A bubble hover names one cluster — it can never
    // supply both pair endpoints, so no compare link may render here.
    assert.doesNotMatch(text, /command:deslop\.compareWithCanonical/);
    assert.doesNotMatch(text, /command:deslop\.comparePair/);
  });

  // Audience: HUMAN. Issue #30. The plain human clone-kind title
  // ("Identical code", "Nearly identical code", …) must be bold in
  // the first line — never the taxonomy jargon ([CLONE-KIND-LABELS]).
  test("bubbleHover title is the plain human clone kind (#30)", () => {
    const c = cluster();
    const text = bubbleHover(c).value;
    const firstLine = text.split("\n")[0] ?? "";
    assert.match(
      firstLine,
      new RegExp(`\\*\\*[0-9a-f]+ ${kindTitle(FIXTURE_KIND)}\\*\\*`),
      `human title must carry the cluster's kind title; got first line: ${firstLine}`,
    );
    assert.doesNotMatch(firstLine, /Type-/, `no taxonomy jargon in the title: ${firstLine}`);
    const identical = { ...c, kind: IDENTICAL_KIND } as const;
    const identicalFirstLine = bubbleHover(identical).value.split("\n")[0] ?? "";
    assert.match(
      identicalFirstLine,
      new RegExp(`\\*\\*[0-9a-f]+ ${kindTitle(IDENTICAL_KIND)}\\*\\*`),
      `a byte-identical cluster is titled as such: ${identicalFirstLine}`,
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
    assert.match(bubble.value, /command:deslop\.openCluster/);
    assert.match(bubble.value, /command:deslop\.bubble\.dismissCluster/);
    assert.doesNotMatch(bubble.value, /command:deslop\.compareWithCanonical/);
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
    const parts = renderBubbleParts(c, "warning");
    assert.equal(inlineText(c, "warning"), parts.inline);
    assert.equal(ghostText(c, "warning"), parts.ghost);
    assert.equal(bubbleHover(c).value, parts.hover.value);
    assert.equal(
      "signalStrip" in parts,
      false,
      "no bubble render part may carry a pair signal strip",
    );
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
    const md = clusterHoverMarkdown(c, { showVerdict: false });
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
    const md = clusterHoverMarkdown(c, { showVerdict: true });
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

// [CLONE-BUCKETS-STRUCTURAL-ONLY] Informational matches never claim clone status.
const SHAPE_ONLY_KIND = "structural_only";
const INFORMATIONAL_NOTICE = "Informational — not a clone.";
const COMPACT_HOVER_COUNT = 2;
const HOVER_STABLE_SLUG = "abcdef0";
const HOVER_OPEN_COMMAND = "command:deslop.openCluster";
const HOVER_CANONICAL_FILE = "Alpha.cs";
const DUPLICATED_MASS_LABEL = "mass";
const DUPLICATION_RANK_LABEL = "rank";
const COPY_COUNT_MARKER = "×";
const SHAPE_HOVER_OPTIONS = [
  { showVerdict: true, showDismiss: true },
  { showVerdict: false, count: COMPACT_HOVER_COUNT },
] as const;
suite("shape-only hover presentation", () => {
  for (const options of SHAPE_HOVER_OPTIONS) {
    test(options.showVerdict ? "full card" : "compact card", () => {
      const shape = { ...cluster(), kind: SHAPE_ONLY_KIND } as const;
      const text = clusterHoverMarkdown(shape, options).value;
      assert.ok(text.includes(INFORMATIONAL_NOTICE), text);
      assert.ok(text.includes(HOVER_STABLE_SLUG), text);
      assert.ok(text.includes(HOVER_OPEN_COMMAND), text);
      assert.ok(text.includes(HOVER_CANONICAL_FILE), text);
      assert.equal(text.includes(DUPLICATED_MASS_LABEL), false, text);
      assert.equal(text.includes(DUPLICATION_RANK_LABEL), false, text);
      assert.equal(text.includes(COPY_COUNT_MARKER), false, text);
      const clone = clusterHoverMarkdown(cluster(), options).value;
      assert.equal(clone.includes(INFORMATIONAL_NOTICE), false, clone);
      assert.ok(clone.includes(COPY_COUNT_MARKER), clone);
    });
  }
});
