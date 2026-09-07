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

/** Two decimals — the precision every human-facing row shows a measured
 * pair signal at. */
export function formatScore(value: number): string {
  return value.toFixed(2);
}

/** No decimals — mass is a whole-number count, canonical nodes × additional
 * visible occurrences ([RANK-MASS-SUM]), and prints exactly as the CLI text
 * report prints it: `527`, never `527.00`. A decimal point would claim a
 * precision a count cannot have. */
export function formatMass(mass: number): string {
  return String(mass);
}
