// Unit: the shared VS Code signal renderer ([FUSED-CONTENT-GATE], #344).
//
// A corroborated Type-2 rename and an anchor-poor scaffolding family render
// the identical confidence triple — structural 1.00, jaccard 1.00 — so a panel
// that draws only the triple tells a reader nothing about which one is worth
// extracting. These tests drive the real formatter and assert the real
// rendered strings for both families, then parse the two webview components to
// prove they render through that formatter instead of a second copy.
//
// The *reading* of those numbers — the shape score and the plain-English
// verdict — is the engine's and arrives on the wire. Its wording is pinned
// where it is written, in `deslop-core::render::signals`
// (`verdict_reads_each_family`, `shape_score_is_the_stronger_axis`); what is
// asserted here is that no VS Code surface manufactures a second one.

import * as assert from "node:assert/strict";
import * as ts from "typescript";
import type { ReportSignals } from "../../types/report";
import * as signalsModule from "../../types/signals";
import {
  SIGNAL_HELP,
  confidenceRows,
  evidenceRows,
  formatSignal,
  helpValueTitle,
  signalTitle,
  type SignalRow,
} from "../../types/signals";
import { parseWebviewSource } from "./webview-source.helpers";

const FULL_CONFIDENCE_TEXT = "1.00";
const STRUCTURAL_TOPIC = "structural";
const JACCARD_TOPIC = "jaccard";
const EMBEDDING_TOPIC = "embedding";
const ZERO_CONFIDENCE_TEXT = "0.00";
const AGREEMENT_TOPIC = "agreement";
const CONTENT_EVIDENCE_TOPIC = "content-evidence";

// Shape saturates, content does not back it up: the scaffolding family the
// engine demotes. The elected pair shares almost none of its content, so
// support falls under the content floor the engine routes by.
const SCAFFOLDING: ReportSignals = {
  structural: 1,
  token_jaccard: 1,
  shape: 1,
  embedding_cos: 0,
  pair_agreement: 0.08,
  pair_rename_consistency: 0,
  literal_fraction: 0.91,
};

// The same shape reading, opposite verdict: one consistent renaming explains
// every difference, so the elected pair's content evidence corroborates it.
const PROVEN_RENAME: ReportSignals = {
  structural: 1,
  token_jaccard: 1,
  shape: 1,
  embedding_cos: 0,
  pair_agreement: 0.1,
  pair_rename_consistency: 1,
  literal_fraction: 0,
};

const VERBATIM: ReportSignals = {
  structural: 1,
  token_jaccard: 1,
  shape: 1,
  embedding_cos: 0,
  pair_agreement: 1,
  pair_rename_consistency: 1,
  literal_fraction: 0,
};

const SEMANTIC: ReportSignals = {
  structural: 0.2,
  token_jaccard: 0.3,
  shape: 0.3,
  embedding_cos: 0.9,
  pair_agreement: 0.05,
  pair_rename_consistency: 0,
  literal_fraction: 0,
};

// The sentences the engine stamps on these two fixtures and ships as
// `cluster.evidence_verdict`. Quoted, not computed: their wording is
// pinned in `deslop-core::render::signals`, and the point here is that
// the panel is handed two different readings of one identical triple.
const SCAFFOLDING_VERDICT =
  "The shapes match at 1.00 but the content behind them does not agree: " +
  "the locations share only 0.08 of their content and consistent renaming " +
  "explains 0.00 of what differs, so support falls below the 0.70 content " +
  "floor. A matching shape over content that does not agree is what " +
  "sibling boilerplate looks like — read both locations before extracting " +
  "anything.";

const PROVEN_RENAME_VERDICT =
  "The shapes match at 1.00 and the content evidence vouches for it: the " +
  "locations share 0.10 of their content and consistent renaming explains " +
  "1.00 of what differs, so the match clears the 0.70 content floor.";

function rendered(rows: SignalRow[]): [string, string][] {
  return rows.map((row) => [row.label, formatSignal(row.value)]);
}

function topics(rows: SignalRow[]): string[] {
  return rows.map((row) => row.topic);
}

suite("signal evidence rendering", () => {
  test("every one of the seven wire fields reaches a labelled, valued row", () => {
    assert.deepEqual(rendered(confidenceRows(SCAFFOLDING)), [
      [STRUCTURAL_TOPIC, FULL_CONFIDENCE_TEXT],
      [JACCARD_TOPIC, FULL_CONFIDENCE_TEXT],
      [EMBEDDING_TOPIC, ZERO_CONFIDENCE_TEXT],
    ]);
    assert.deepEqual(rendered(evidenceRows(SCAFFOLDING)), [
      [AGREEMENT_TOPIC, "0.08"],
      ["rename", ZERO_CONFIDENCE_TEXT],
      ["literal", "0.91"],
    ]);
    assert.deepEqual(topics(confidenceRows(SCAFFOLDING)), [
      STRUCTURAL_TOPIC,
      JACCARD_TOPIC,
      EMBEDDING_TOPIC,
    ]);
    assert.deepEqual(topics(evidenceRows(SCAFFOLDING)), [
      AGREEMENT_TOPIC,
      "rename-consistency",
      "literal-fraction",
    ]);
    assert.deepEqual(rendered(evidenceRows(PROVEN_RENAME)), [
      [AGREEMENT_TOPIC, "0.10"],
      ["rename", FULL_CONFIDENCE_TEXT],
      ["literal", ZERO_CONFIDENCE_TEXT],
    ]);
    assert.deepEqual(rendered(confidenceRows(VERBATIM)), [
      [STRUCTURAL_TOPIC, FULL_CONFIDENCE_TEXT],
      [JACCARD_TOPIC, FULL_CONFIDENCE_TEXT],
      [EMBEDDING_TOPIC, ZERO_CONFIDENCE_TEXT],
    ]);
    assert.equal(VERBATIM.shape, 1, "a byte-proven cluster carries a saturated shape reading");
  });

  test("the two families that render one triple get two different readings", () => {
    // The whole point of #344: without the evidence rows and the verdict,
    // these two clusters are indistinguishable in the panel.
    assert.deepEqual(rendered(confidenceRows(SCAFFOLDING)).slice(0, 2), [
      [STRUCTURAL_TOPIC, FULL_CONFIDENCE_TEXT],
      [JACCARD_TOPIC, FULL_CONFIDENCE_TEXT],
    ]);
    assert.deepEqual(rendered(confidenceRows(PROVEN_RENAME)).slice(0, 2), [
      [STRUCTURAL_TOPIC, FULL_CONFIDENCE_TEXT],
      [JACCARD_TOPIC, FULL_CONFIDENCE_TEXT],
    ]);
    assert.notDeepEqual(
      rendered(evidenceRows(SCAFFOLDING)),
      rendered(evidenceRows(PROVEN_RENAME)),
      "the content evidence is the only thing separating the two families",
    );
    assert.notEqual(
      SCAFFOLDING_VERDICT,
      PROVEN_RENAME_VERDICT,
      "one shape reading must not produce one explanation",
    );
    assert.ok(
      SCAFFOLDING_VERDICT.includes("sibling boilerplate"),
      "the demoted family is named as boilerplate",
    );
    assert.ok(
      !PROVEN_RENAME_VERDICT.includes("boilerplate"),
      "a corroborated rename must never be described as boilerplate",
    );
  });

  test("no VS Code surface owns a second reading of the signal numbers", () => {
    // The shape score and the verdict are engine calculations carried on
    // the wire as `signals.shape` and `cluster.evidence_verdict`. A client
    // copy of either could disagree with the report it sits beside, which
    // is the whole defect this module was cleansed of.
    assert.ok(
      !("contentEvidenceVerdict" in signalsModule),
      "the client must not carry a verdict engine",
    );
    assert.ok(
      !("shapeScore" in signalsModule),
      "the client must not re-derive the shape score",
    );
    assert.ok(
      !("FUSED_THRESHOLD" in signalsModule),
      "the reportable-confidence cutoff is the engine's constant, not a client copy",
    );
    assert.equal(
      SCAFFOLDING.shape,
      1,
      "the engine's shape reading rides on the signals the panel renders",
    );
    assert.equal(SEMANTIC.shape, 0.3);
  });

  test("the strip renders the engine's verdict verbatim", () => {
    const strip = parseWebviewSource("components/SignalStrip.tsx");
    const source = strip.getFullText();
    assert.ok(
      source.includes("{verdict}"),
      "the panel must print the engine's sentence, not compose one",
    );
    assert.ok(
      !source.includes("contentEvidenceVerdict"),
      "the panel must not call a client-side verdict builder",
    );
    for (const word of ["boilerplate", "The shapes match at"]) {
      assert.ok(
        !source.includes(word),
        `the panel must not restate the verdict wording: ${word}`,
      );
    }
  });

  test("every evidence tooltip carries its explanation and its measured value", () => {
    const [agreement, rename, literal] = evidenceRows(SCAFFOLDING);
    assert.ok(agreement && rename && literal, "three evidence rows must render");
    assert.equal(
      signalTitle(agreement),
      "How much of the matched content the locations genuinely share, byte for " +
        "byte. Low agreement under a high shape score means the skeleton lined " +
        "up but the code inside it did not. Current value: 0.08.",
    );
    assert.match(signalTitle(rename), /one consistent identifier renaming/);
    assert.match(signalTitle(rename), /Current value: 0\.00\.$/);
    assert.match(signalTitle(literal), /literal data rather than logic/);
    assert.match(signalTitle(literal), /Current value: 0\.91\.$/);
    assert.equal(helpValueTitle("Copy.", "0.42"), "Copy. Current value: 0.42.");
  });

  test("the signal help vocabulary covers every axis and both headings", () => {
    assert.deepEqual(Object.keys(SIGNAL_HELP).sort(), [
      AGREEMENT_TOPIC,
      CONTENT_EVIDENCE_TOPIC,
      EMBEDDING_TOPIC,
      JACCARD_TOPIC,
      "literal-fraction",
      "rename-consistency",
      "signals",
      STRUCTURAL_TOPIC,
    ]);
    for (const [topic, copy] of Object.entries(SIGNAL_HELP)) {
      assert.ok(copy.length > 20, `help copy for ${topic} explains nothing`);
    }
    assert.match(SIGNAL_HELP[CONTENT_EVIDENCE_TOPIC], /structural 1\.00 and jaccard 1\.00/);
    assert.match(SIGNAL_HELP[CONTENT_EVIDENCE_TOPIC], /sibling boilerplate/);
    assert.ok(
      !("fused" in SIGNAL_HELP),
      "the help vocabulary must not carry a combined-score topic: there is no fused on the wire",
    );
  });

  test("the strip renders through the shared formatter, not a second copy", () => {
    const strip = parseWebviewSource("components/SignalStrip.tsx");
    const called = calledFunctions(strip);
    for (const fn of ["confidenceRows", "evidenceRows", "signalTitle"]) {
      assert.ok(called.has(fn), `SignalStrip must render through ${fn}()`);
    }
    assert.deepEqual(
      signalFieldReads(strip),
      [],
      "SignalStrip must not reach into signal fields itself — that is a second formatter",
    );
    assert.ok(
      strip.getFullText().includes('topic="content-evidence"'),
      "the evidence rows must sit under a helped CONTENT EVIDENCE heading",
    );
  });

  test("the help bubbles reuse the shared signal copy", () => {
    const bubble = parseWebviewSource("components/HelpBubble.tsx");
    const source = bubble.getFullText();
    assert.ok(source.includes("...SIGNAL_HELP"), "HelpBubble must fold in the shared copy");
    for (const restated of ["AST-shape similarity", "Combined clone score", "Four scores"]) {
      assert.ok(
        !source.includes(restated),
        `HelpBubble must not restate signal copy: ${restated}`,
      );
    }
  });
});

function calledFunctions(root: ts.SourceFile): Set<string> {
  const names = new Set<string>();
  function visit(node: ts.Node): void {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression)) {
      names.add(node.expression.text);
    }
    node.forEachChild(visit);
  }
  visit(root);
  return names;
}

const SIGNAL_FIELDS = new Set([
  "structural",
  "token_jaccard",
  "shape",
  "embedding_cos",
  "pair_agreement",
  "pair_rename_consistency",
  "literal_fraction",
]);

function signalFieldReads(root: ts.SourceFile): string[] {
  const reads: string[] = [];
  function visit(node: ts.Node): void {
    if (
      ts.isPropertyAccessExpression(node) &&
      ts.isIdentifier(node.expression) &&
      node.expression.text === "signals" &&
      SIGNAL_FIELDS.has(node.name.text)
    ) {
      reads.push(node.name.text);
    }
    node.forEachChild(visit);
  }
  visit(root);
  return reads;
}
