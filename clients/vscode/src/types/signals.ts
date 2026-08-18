// One rendering of the seven-field signal breakdown, shared by every VS Code
// surface — the client-side twin of `deslop-core::render::signals`, which does
// the same job for the CLI, Markdown and HTML reports ([FUSION-CONTENT-GATE],
// #344).
//
// `structural`, `token_jaccard` and `embedding_cos` say *how much* the members
// matched; they never say *why* a cluster landed where it did. A corroborated
// Type-2 rename and an anchor-poor scaffolding family render the identical
// triple — only the measured content evidence separates them. A panel that
// draws the confidence without the evidence behind it shows
// `structural 1.00 / jaccard 1.00` for both and leaves the reader guessing why
// one is worth extracting and the other is sibling boilerplate.
//
// Every VS Code surface formats signals here rather than restating the field
// list, so the cluster panel, its help bubbles and the docs anchors can never
// drift into describing the same numbers differently.

import { FUSED_THRESHOLD, type ReportSignals } from "./report";

/** One measured axis of the breakdown — a help topic and a docs anchor. */
export type SignalAxisTopic =
  | "structural"
  | "jaccard"
  | "embedding"
  | "fused"
  | "agreement"
  | "rename-consistency"
  | "literal-fraction";

/** The two headings the axes sit under, which carry help of their own. */
export type SignalSectionTopic = "signals" | "content-evidence";

/** Every signal-related help topic: the seven axes plus the two headings. */
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
    "Four confidence scores and the three pieces of measured content evidence behind them.",
  structural: "AST-shape similarity after identifiers and literals are normalized.",
  jaccard: "Token-overlap similarity after formatting and trivia are ignored.",
  embedding: "Semantic similarity from the selected local embedding model.",
  fused:
    "Combined clone score used to decide whether a pair is reportable. A shape match is discounted by the content evidence, so fused can sit far below a perfect structural score.",
  "content-evidence":
    "What Deslop actually measured inside the matched code. Shape alone cannot tell a renamed copy from unrelated code that happens to share a skeleton — two locations can both score structural 1.00 and jaccard 1.00 while one is worth extracting and the other is sibling boilerplate. These three numbers are the difference.",
  agreement:
    "How much of the matched content the locations genuinely share, byte for byte. Low agreement under a high shape score means the skeleton lined up but the code inside it did not.",
  "rename-consistency":
    "Whether one consistent identifier renaming explains every difference between the locations. This is what tells a real renamed copy apart from unrelated code that merely shares a shape.",
  "literal-fraction":
    "How much of the match is literal data rather than logic. A match that is mostly literals is a data table, not a function worth extracting.",
};

/** Two-decimal rendering, matching `deslop-core::render::signals`. */
export function formatSignal(value: number): string {
  return value.toFixed(2);
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

/** The four confidence axes, in the order every Deslop surface prints them. */
export function confidenceRows(signals: ReportSignals): SignalRow[] {
  return [
    { topic: "structural", label: "structural", value: signals.structural },
    { topic: "jaccard", label: "jaccard", value: signals.token_jaccard },
    { topic: "embedding", label: "embedding", value: signals.embedding_cos },
    { topic: "fused", label: "fused", value: signals.fused },
  ];
}

/**
 * The three measured content-evidence axes ([FUSION-CONTENT-GATE]). Labels
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

/**
 * The shape reading. `structural` and `token_jaccard` are two views of one
 * normalised representation — summing them says nothing beyond "the shapes
 * matched" — so the stronger of the two is what "the shape matched" means,
 * the same reduction `deslop-core::buckets` applies.
 */
export function shapeScore(signals: ReportSignals): number {
  return Math.max(signals.structural, signals.token_jaccard);
}

// Half of the last digit the panel prints. Two values that render as the same
// string must never be described as different.
const RENDERED_EPSILON = 0.005;

/**
 * Plain-English reading of the shape score against the measured content
 * evidence, for readers who have four decimals in front of them and no way to
 * tell a renamed copy from boilerplate.
 *
 * Grounded only in the numbers the panel already renders: it explains the gap
 * between the shape match and the fused confidence, and never re-derives the
 * engine's bucket ([CLONE-BUCKETS-ROUTING] — the UI reads the engine's label
 * and never manufactures one).
 */
export function contentEvidenceVerdict(signals: ReportSignals): string {
  const shape = shapeScore(signals);
  if (signals.embedding_cos > shape && signals.embedding_cos + RENDERED_EPSILON >= signals.fused) {
    return semanticVerdict(signals, shape);
  }
  if (signals.fused + RENDERED_EPSILON >= shape) return corroboratedVerdict(signals, shape);
  return signals.fused >= FUSED_THRESHOLD
    ? discountedVerdict(signals, shape)
    : boilerplateVerdict(signals, shape);
}

// The embedding pass, not the shape, is what produced this confidence.
function semanticVerdict(signals: ReportSignals, shape: number): string {
  return (
    `The shapes barely match (${formatSignal(shape)}) — the ` +
    `${formatSignal(signals.fused)} confidence comes from the embedding model, ` +
    `which read these as the same behavior written two ways. The content evidence ` +
    `measures the code itself, not the behavior: shared content ` +
    `${formatSignal(signals.agreement)}, renaming ${formatSignal(signals.rename_consistency)}.`
  );
}

// The evidence did not pull the confidence below the shape match. Stated as
// the measurement, never as a recommendation: the engine owns the verdict.
function corroboratedVerdict(signals: ReportSignals, shape: number): string {
  return (
    `The shapes match at ${formatSignal(shape)} and the content evidence did not ` +
    `discount that: the locations share ${formatSignal(signals.agreement)} of their ` +
    `content and consistent renaming explains ` +
    `${formatSignal(signals.rename_consistency)} of what differs, so confidence stayed ` +
    `at ${formatSignal(signals.fused)}.`
  );
}

// Discounted, but still above the reportable bar: the evidence carried it.
function discountedVerdict(signals: ReportSignals, shape: number): string {
  return (
    `The shapes match at ${formatSignal(shape)} but the locations are not byte for ` +
    `byte the same: they share ${formatSignal(signals.agreement)} of their content and ` +
    `one consistent identifier renaming explains ` +
    `${formatSignal(signals.rename_consistency)} of what differs. That measured ` +
    `evidence is what holds confidence at ${formatSignal(signals.fused)} instead of the ` +
    `full shape match.`
  );
}

// Discounted below the reportable bar — the anchor-poor scaffolding family.
function boilerplateVerdict(signals: ReportSignals, shape: number): string {
  return (
    `The shapes match at ${formatSignal(shape)} but the content behind them does not ` +
    `agree: the locations share only ${formatSignal(signals.agreement)} of their content ` +
    `and consistent renaming explains ${formatSignal(signals.rename_consistency)} of what ` +
    `differs, so confidence fell to ${formatSignal(signals.fused)}. A matching shape over ` +
    `content that does not agree is what sibling boilerplate looks like — read both ` +
    `locations before extracting anything.`
  );
}
