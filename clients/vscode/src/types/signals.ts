// One rendering of the six-axis elected-pair evidence breakdown, shared by
// every VS Code surface ([FUSED-CLUSTER-SIGNALS], [FUSED-CONTENT-GATE]).
//
// Rendering only. Every reading of these numbers — the elected pair, the
// plain-English verdict — is computed once by `deslop-core::render::signals`
// and carried on the wire; this module decides labels, ordering and decimal
// places and derives nothing. There is no combined score to render: the
// engine admits pairs and routes buckets, and this panel shows the measured
// evidence behind the bucket it was given.
//
// `structural`, `token_jaccard` and `embedding_cos` say *how much* the members
// matched; they never say *why* a cluster landed where it did. A corroborated
// Type-2 rename and an anchor-poor scaffolding family render the identical
// triple — only the measured content evidence separates them. A panel that
// draws the shape match without the evidence behind it shows
// `structural 1.00 / jaccard 1.00` for both and leaves the reader guessing why
// one is worth extracting and the other is sibling boilerplate.
//
// Every VS Code surface formats signals here rather than restating the field
// list, so the cluster panel, its help bubbles and the docs anchors can never
// drift into describing the same numbers differently.

import { formatScore } from "./format";
import type { ReportSignals } from "./report";

/** One measured axis of the breakdown — a help topic and a docs anchor. */
export type SignalAxisTopic =
  | "structural"
  | "jaccard"
  | "embedding"
  | "agreement"
  | "rename-consistency"
  | "literal-fraction";

/** The two headings the axes sit under, which carry help of their own. */
export type SignalSectionTopic = "signals" | "content-evidence";

/** Every signal-related help topic: the six axes plus the two headings. */
export type SignalTopic = SignalAxisTopic | SignalSectionTopic;

/** One rendered signal: its docs topic, its column label, its measured value. */
export interface SignalRow {
  topic: SignalAxisTopic;
  label: string;
  value: number;
}

/**
 * Help copy for every signal topic. The single definition: the webview help
 * bubbles fold this into their topic table instead of restating it, so a
 * wording change lands on the tooltip, the aria-label and the docs link at
 * once.
 */
export const SIGNAL_HELP: Record<SignalTopic, string> = {
  signals:
    "The three shape and semantic axes of one elected pair, plus the measured content evidence behind them.",
  structural: "AST-shape similarity after identifiers and literals are normalized.",
  jaccard: "Token-overlap similarity after formatting and trivia are ignored.",
  embedding: "Semantic similarity from the selected local embedding model.",
  "content-evidence":
    "What Deslop actually measured inside the matched code. Shape alone cannot tell a renamed copy from unrelated code that happens to share a skeleton — two locations can both score structural 1.00 and jaccard 1.00 while one is worth extracting and the other is sibling boilerplate. These three numbers are the difference.",
  agreement:
    "How much of the matched content the locations genuinely share, byte for byte. Low agreement under a high shape score means the skeleton lined up but the code inside it did not.",
  "rename-consistency":
    "Whether one consistent identifier renaming explains every difference between the locations. This is what tells a real renamed copy apart from unrelated code that merely shares a shape.",
  "literal-fraction":
    "How much of the match is literal data rather than logic. A match that is mostly literals is a data table, not a function worth extracting.",
};

/** Two-decimal rendering, matching `deslop-core::render::signals`. The
 * one implementation lives in `./format` so a signal cell and a weight
 * cell can never print to different precision. */
export function formatSignal(value: number): string {
  return formatScore(value);
}

/**
 * Hover text for any helped value in the cluster panel: the explanation
 * followed by the current reading. One sentence template, so the signal cells
 * and the header metrics never phrase the same tooltip two ways.
 */
export function helpValueTitle(copy: string, value: string): string {
  return `${copy} Current value: ${value}.`;
}

/** Hover text for one signal cell: the explanation plus the measured value. */
export function signalTitle(row: SignalRow): string {
  return helpValueTitle(SIGNAL_HELP[row.topic], formatSignal(row.value));
}

/** The three shape/semantic axes of the elected pair, in the order every
 * Deslop surface prints them. */
export function confidenceRows(signals: ReportSignals): SignalRow[] {
  return [
    { topic: "structural", label: "structural", value: signals.structural },
    { topic: "jaccard", label: "jaccard", value: signals.token_jaccard },
    { topic: "embedding", label: "embedding", value: signals.embedding_cos },
  ];
}

/**
 * The three measured content-evidence axes ([FUSED-CONTENT-GATE]). Labels
 * match the `agreement / rename / literal` columns of the CLI signal table so
 * a reader moving between the panel and the report reads one vocabulary.
 */
export function evidenceRows(signals: ReportSignals): SignalRow[] {
  return [
    { topic: "agreement", label: "agreement", value: signals.agreement },
    { topic: "rename-consistency", label: "rename", value: signals.rename_consistency },
    { topic: "literal-fraction", label: "literal", value: signals.literal_fraction },
  ];
}
