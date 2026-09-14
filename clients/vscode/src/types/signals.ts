// Signal value formatting shared by the VS Code surfaces that quote the
// wire's measured pair signals ([FUSED-PAIR-SIGNALS], [FUSED-CONTENT-GATE]).
//
// Rendering only. The engine returns evidence for two exact endpoints; this
// module decides decimal places and derives nothing.
//
// The measured axes (structural, token_jaccard, embedding_cos, agreement,
// rename_consistency) describe **one pair of occurrences**, never the
// cluster: admission is decided pair by pair, and a cluster card that
// rendered them would misattribute a pair measurement to the whole closure.
// Cluster surfaces therefore show membership and mass only. Explicit pair
// comparison may render these values in a subtle secondary row.

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
