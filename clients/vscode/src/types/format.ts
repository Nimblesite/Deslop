// The one rendering of every engine-computed figure a surface prints
// ([METRICS-REPO], [PRINCIPLES-ONE-CALCULATION]).
//
// The values themselves are always the engine's, carried on the wire.
// This module only decides how many digits a human sees, and it decides
// it once, so the status bar, the Duplication panel, the threshold row,
// the tree rows and both webviews can never print the same figure to
// different precision.

/** One decimal place, percent-suffixed — `12.8%`. */
export function formatPercent(percent: number): string {
  return `${percent.toFixed(1)}%`;
}

/** Two decimals — the precision every human-facing row shows a signal or
 * a ranking weight at. */
export function formatScore(value: number): string {
  return value.toFixed(2);
}

/** Four decimals — the copy-for-AI payload, which quotes the same figures
 * at more precision than a human row needs. */
export function formatScorePrecise(value: number): string {
  return value.toFixed(4);
}
