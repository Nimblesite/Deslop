// Unit: the compare diff title ([VSIX-PAIR-COMPARE]). The title names both
// endpoints and repeats the engine's verdict on exactly that pair; the
// extension never derives a verdict of its own ([FUSED-PAIR-SIGNALS]).

import * as assert from "node:assert/strict";

import {
  compareTitle,
  ENDPOINT_SEPARATOR,
  IDENTICAL_BYTES_VERDICT,
  INDENTATION_ONLY_VERDICT,
  measurePair,
  pairVerdict,
  VERDICT_SEPARATOR,
} from "../../compare/title";
import { kindTitle, type PairEndpoint, type PairEvidence } from "../../types/report";

const LEFT: PairEndpoint = { path: "src/alpha/Alpha.cs", start_byte: 0, end_byte: 40 };
const RIGHT: PairEndpoint = { path: "src/beta/Beta.cs", start_byte: 8, end_byte: 48 };
const LEFT_NAME = "Alpha.cs";
const RIGHT_NAME = "Beta.cs";
const NAMES = `${LEFT_NAME}${ENDPOINT_SEPARATOR}${RIGHT_NAME}`;
const REJECTION = "rejected: pair fails content corroboration";

function evidence(overrides: Partial<PairEvidence>): PairEvidence {
  return {
    structural: 1,
    token_jaccard: 1,
    embedding_cos: 0,
    agreement: 1,
    rename_consistency: 1,
    literal_fraction: 0,
    text_identity: "different",
    fused_score: 1,
    content_required: true,
    content_ok: true,
    admitted: true,
    classification: "nearly_identical",
    explanation: "admitted: explicit pair clears every admission guard",
    ...overrides,
  };
}

suite("compare diff title", () => {
  test("byte-identical ranges are called identical, whatever else the evidence says", () => {
    const verdict = pairVerdict(evidence({ classification: "identical", text_identity: "byte_identical" }));
    assert.equal(verdict, IDENTICAL_BYTES_VERDICT);
  });

  test("a copy that differs only by indentation says so instead of its classification", () => {
    const verdict = pairVerdict(evidence({ classification: "nearly_identical", text_identity: "indentation_only" }));
    assert.equal(verdict, INDENTATION_ONLY_VERDICT);
    assert.notEqual(verdict, kindTitle("nearly_identical"));
  });

  test("a real difference shows the engine's classification and never claims indentation", () => {
    for (const classification of ["nearly_identical", "same_behavior", "structural_only", "loosely_similar"] as const) {
      const verdict = pairVerdict(evidence({ classification }));
      assert.equal(verdict, kindTitle(classification));
      assert.notEqual(verdict, INDENTATION_ONLY_VERDICT);
      assert.notEqual(verdict, IDENTICAL_BYTES_VERDICT);
    }
  });

  test("an unclassified pair repeats the engine's explanation verbatim", () => {
    const verdict = pairVerdict(evidence({ admitted: false, classification: undefined, explanation: REJECTION }));
    assert.equal(verdict, REJECTION);
  });

  test("the title names both endpoints by file and ends with the verdict", () => {
    const title = compareTitle(LEFT, RIGHT, evidence({ classification: "identical", text_identity: "byte_identical" }));
    assert.equal(title, `${NAMES}${VERDICT_SEPARATOR}${IDENTICAL_BYTES_VERDICT}`);
    assert.ok(title.startsWith(LEFT_NAME), "the left endpoint is named first");
    assert.ok(title.includes(RIGHT_NAME), "the right endpoint is named");
  });

  test("without evidence the title carries the two names and no verdict", () => {
    assert.equal(compareTitle(LEFT, RIGHT, undefined), NAMES);
  });

  // The request path itself runs against the real language server in
  // clusters.e2e.test.ts; no stand-in engine is exercised here.
  test("measurePair yields no verdict without a language client", async () => {
    assert.equal(await measurePair(undefined, LEFT, RIGHT), undefined);
  });
});
