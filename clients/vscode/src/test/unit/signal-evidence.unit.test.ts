// Unit: the shared VS Code signal renderer ([FUSION-CONTENT-GATE], #344).
//
// A corroborated Type-2 rename and an anchor-poor scaffolding family render
// the identical confidence triple — structural 1.00, jaccard 1.00 — so a panel
// that draws only the triple tells a reader nothing about which one is worth
// extracting. These tests drive the real formatter and assert the real
// rendered strings for both families, then parse the two webview components to
// prove they render through that formatter instead of a second copy.

import * as assert from "node:assert/strict";
import * as ts from "typescript";
import type { ReportSignals } from "../../types/report";
import {
  SIGNAL_HELP,
  confidenceRows,
  contentEvidenceVerdict,
  evidenceRows,
  formatSignal,
  helpValueTitle,
  shapeScore,
  signalTitle,
  type SignalRow,
} from "../../types/signals";
import { parseWebviewSource } from "./webview-source.helpers";

// Shape saturates, content does not back it up: the scaffolding family the
// engine demotes. Rendered by `deslop-core::buckets::content_gated_signals`
// as a perfect shape match discounted to a sixth of itself.
const SCAFFOLDING: ReportSignals = {
  structural: 1,
  token_jaccard: 1,
  embedding_cos: 0,
  fused: 0.16,
  agreement: 0.08,
  rename_consistency: 0,
  literal_fraction: 0.91,
};

// The same shape reading, opposite verdict: one consistent renaming explains
// every difference, so the confidence survives the gate.
const PROVEN_RENAME: ReportSignals = {
  structural: 1,
  token_jaccard: 1,
  embedding_cos: 0,
  fused: 0.9,
  agreement: 0.1,
  rename_consistency: 1,
  literal_fraction: 0,
};

const VERBATIM: ReportSignals = {
  structural: 1,
  token_jaccard: 1,
  embedding_cos: 0,
  fused: 1,
  agreement: 1,
  rename_consistency: 1,
  literal_fraction: 0,
};

const SEMANTIC: ReportSignals = {
  structural: 0.2,
  token_jaccard: 0.3,
  embedding_cos: 0.9,
  fused: 0.9,
  agreement: 0.05,
  rename_consistency: 0,
  literal_fraction: 0,
};

function rendered(rows: SignalRow[]): [string, string][] {
  return rows.map((row) => [row.label, formatSignal(row.value)]);
}

function topics(rows: SignalRow[]): string[] {
  return rows.map((row) => row.topic);
}

suite("signal evidence rendering", () => {
  test("every one of the seven wire fields reaches a labelled, valued row", () => {
    assert.deepEqual(rendered(confidenceRows(SCAFFOLDING)), [
      ["structural", "1.00"],
      ["jaccard", "1.00"],
      ["embedding", "0.00"],
      ["fused", "0.16"],
    ]);
    assert.deepEqual(rendered(evidenceRows(SCAFFOLDING)), [
      ["agreement", "0.08"],
      ["rename", "0.00"],
      ["literal", "0.91"],
    ]);
    assert.deepEqual(topics(confidenceRows(SCAFFOLDING)), [
      "structural",
      "jaccard",
      "embedding",
      "fused",
    ]);
    assert.deepEqual(topics(evidenceRows(SCAFFOLDING)), [
      "agreement",
      "rename-consistency",
      "literal-fraction",
    ]);
    assert.deepEqual(rendered(evidenceRows(PROVEN_RENAME)), [
      ["agreement", "0.10"],
      ["rename", "1.00"],
      ["literal", "0.00"],
    ]);
  });

  test("the two families that render one triple get two different readings", () => {
    // The whole point of #344: without the evidence rows and the verdict,
    // these two clusters are indistinguishable in the panel.
    assert.deepEqual(rendered(confidenceRows(SCAFFOLDING)).slice(0, 2), [
      ["structural", "1.00"],
      ["jaccard", "1.00"],
    ]);
    assert.deepEqual(rendered(confidenceRows(PROVEN_RENAME)).slice(0, 2), [
      ["structural", "1.00"],
      ["jaccard", "1.00"],
    ]);
    assert.notDeepEqual(
      rendered(evidenceRows(SCAFFOLDING)),
      rendered(evidenceRows(PROVEN_RENAME)),
      "the content evidence is the only thing separating the two families",
    );
    assert.notEqual(
      contentEvidenceVerdict(SCAFFOLDING),
      contentEvidenceVerdict(PROVEN_RENAME),
      "one shape reading must not produce one explanation",
    );
  });

  test("a shape match with no content behind it reads as sibling boilerplate", () => {
    assert.equal(
      contentEvidenceVerdict(SCAFFOLDING),
      "The shapes match at 1.00 but the content behind them does not agree: " +
        "the locations share only 0.08 of their content and consistent renaming " +
        "explains 0.00 of what differs, so confidence fell to 0.16. A matching " +
        "shape over content that does not agree is what sibling boilerplate " +
        "looks like — read both locations before extracting anything.",
    );
  });

  test("a corroborated rename explains the discount instead of warning about it", () => {
    const verdict = contentEvidenceVerdict(PROVEN_RENAME);
    assert.equal(
      verdict,
      "The shapes match at 1.00 but the locations are not byte for byte the " +
        "same: they share 0.10 of their content and one consistent identifier " +
        "renaming explains 1.00 of what differs. That measured evidence is what " +
        "holds confidence at 0.90 instead of the full shape match.",
    );
    assert.ok(
      !verdict.includes("boilerplate"),
      "a corroborated rename must never be described as boilerplate",
    );
  });

  test("an undiscounted match and an embedding-led match each say why", () => {
    assert.equal(
      contentEvidenceVerdict(VERBATIM),
      "The shapes match at 1.00 and the content evidence did not discount that: " +
        "the locations share 1.00 of their content and consistent renaming " +
        "explains 1.00 of what differs, so confidence stayed at 1.00.",
    );
    assert.equal(
      contentEvidenceVerdict(SEMANTIC),
      "The shapes barely match (0.30) — the 0.90 confidence comes from the " +
        "embedding model, which read these as the same behavior written two " +
        "ways. The content evidence measures the code itself, not the " +
        "behavior: shared content 0.05, renaming 0.00.",
    );
    assert.equal(shapeScore(SEMANTIC), 0.3);
    assert.equal(shapeScore(SCAFFOLDING), 1);
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
      "agreement",
      "content-evidence",
      "embedding",
      "fused",
      "jaccard",
      "literal-fraction",
      "rename-consistency",
      "signals",
      "structural",
    ]);
    for (const [topic, copy] of Object.entries(SIGNAL_HELP)) {
      assert.ok(copy.length > 20, `help copy for ${topic} explains nothing`);
    }
    assert.match(SIGNAL_HELP["content-evidence"], /structural 1\.00 and jaccard 1\.00/);
    assert.match(SIGNAL_HELP["content-evidence"], /sibling boilerplate/);
    assert.match(SIGNAL_HELP.fused, /discounted by the content evidence/);
  });

  test("the strip renders through the shared formatter, not a second copy", () => {
    const strip = parseWebviewSource("components/SignalStrip.tsx");
    const called = calledFunctions(strip);
    for (const fn of ["confidenceRows", "evidenceRows", "contentEvidenceVerdict", "signalTitle"]) {
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
  "embedding_cos",
  "fused",
  "agreement",
  "rename_consistency",
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
