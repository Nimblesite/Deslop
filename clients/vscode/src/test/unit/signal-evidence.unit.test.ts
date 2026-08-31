// Unit: the VS Code cluster surfaces render no pair-only signal evidence
// ([FUSED-PAIR-SIGNALS]).
//
// The admission signals (structural, token Jaccard, embedding cosine, content
// agreement, rename consistency, literal fraction) are measurements of one
// exact pair of occurrences. They never belong to the cluster: the cluster
// carries its occurrence membership and duplicated mass only, and no cluster
// JSON, text, HTML, LSP, MCP, editor, or AI-context surface renders any of
// them. An explicit pair comparison is the only surface that may quote a
// pair's values.
//
// The *reading* of those numbers — the bucket and the verdict — is the
// engine's and arrives on the wire. Its wording is pinned where it is
// written, in `deslop-core::render::signals`; what is asserted here is that
// no VS Code cluster surface manufactures a second one or renders a pair
// measurement as a cluster fact.

import * as assert from "node:assert/strict";
import * as signalsModule from "../../types/signals";
import { helpValueTitle } from "../../types/signals";
import { parseWebviewSource } from "./webview-source.helpers";

const CLUSTER_RENDERER_PATH = "cluster/main.tsx";

suite("cluster surfaces render no pair evidence", () => {
  test("types/signals carries no signal-row formatter for a cluster panel", () => {
    for (const dead of ["confidenceRows", "evidenceRows", "signalTitle", "SIGNAL_HELP"]) {
      assert.ok(
        !(dead in signalsModule),
        `the cluster surfaces must not carry ${dead}: no cluster surface renders a signal row`,
      );
    }
    assert.equal(
      typeof helpValueTitle,
      "function",
      "the one-sentence hover template for header metrics survives",
    );
  });

  test("no VS Code surface owns a second reading of the signal numbers", () => {
    // The bucket and the verdict are engine calculations carried on the
    // wire. A client copy of either could disagree with the report it sits
    // beside, which is the whole defect this module was cleansed of.
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
  });

  test("the cluster panel renders no signal strip and no pair-evidence panel", () => {
    const renderer = parseWebviewSource(CLUSTER_RENDERER_PATH);
    const source = renderer.getFullText();
    for (const gone of [
      "SignalStrip",
      "PairEvidence",
      "signal_source",
      "pair_agreement",
      "pair_rename_consistency",
      "literal_fraction",
      "structural",
      "evidence_verdict",
      "CONTENT EVIDENCE",
      "ELECTED PAIR",
    ]) {
      assert.ok(
        !source.includes(gone),
        `the cluster panel must not render pair evidence: ${gone}`,
      );
    }
  });

  test("the help bubbles carry no signal help copy", () => {
    const bubble = parseWebviewSource("components/HelpBubble.tsx");
    const source = bubble.getFullText();
    for (const restated of [
      "SIGNAL_HELP",
      "AST-shape similarity",
      "Combined clone score",
      "content-evidence",
    ]) {
      assert.ok(
        !source.includes(restated),
        `HelpBubble must not restate signal copy: ${restated}`,
      );
    }
  });

  test("the hover template still appends the current value", () => {
    assert.equal(helpValueTitle("Copy.", "0.42"), "Copy. Current value: 0.42.");
  });
});
