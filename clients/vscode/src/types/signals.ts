// Signal value formatting shared by the VS Code surfaces that quote the
// wire's measured pair signals ([FUSED-CLUSTER-SIGNALS], [FUSED-CONTENT-GATE]).
//
// Rendering only. Every reading of these numbers — which pair the signals
// describe, the plain-English verdict — is computed once by
// `deslop-core::render::signals` and carried on the wire; this module decides
// labels and decimal places and derives nothing.
//
// The measured axes (structural, token_jaccard, embedding_cos, agreement,
// rename_consistency) describe **one pair of occurrences**, never the
// cluster: admission is decided pair by pair, and a cluster card that
// rendered them would let a reader attribute a pair measurement to the whole
// cluster. The cluster surfaces therefore show cluster facts (bucket, weight,
// size, occurrences) and no signal bars; the pair-named reports and LSP
// surfaces quote the numbers through the engine's own renderers.

import { formatScore } from "./format";

/** Two-decimal rendering, matching `deslop-core::render::signals`. The
 * one implementation lives in `./format` so a signal cell and a weight
 * cell can never print to different precision. */
export function formatSignal(value: number): string {
  return formatScore(value);
}

/**
 * Hover text for any helped value in the cluster panel: the explanation
 * followed by the current reading. One sentence template, so the header
 * metrics never phrase the same tooltip two ways.
 */
export function helpValueTitle(copy: string, value: string): string {
  return `${copy} Current value: ${value}.`;
}
