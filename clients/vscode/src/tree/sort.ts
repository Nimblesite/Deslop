// Sort axis for the Top Offenders tree ([VSIX-TOP-OFFENDERS-SORT]).
// Orthogonal to the grouping mode: it reorders the DISPLAY order in every
// mode — cluster, file, and folder roots, plus the occurrences inside a
// cluster. impact is worst-first; path is alphabetical. Sorting is
// presentation-only ([VSIX-VIEW-STATE-UI-ONLY]): it never re-fetches or
// re-analyses, and it never changes a cluster's global rank #N
// ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]).

export type SortBy = "impact" | "path";

/** A row keyed by a representative name plus its worst (max) and
 * aggregate (sum) cluster weights — the comparable shape shared by file
 * rows and folder rows. `path` is the local sort key (a folder label or
 * a file/folder name), so path-order reads alphabetically within each
 * parent. */
export interface WeightedPath {
  path: string;
  maxWeight: number;
  sumWeight: number;
}

/** Reads the persisted sort axis, falling back to `"impact"` for
 * unknown / missing values — never throws. */
export function normalizeSortBy(raw: string | undefined): SortBy {
  return raw === "path" ? "path" : "impact";
}

/** Comparator for {@link WeightedPath} rows under the active sort axis.
 * `impact` is worst-first (max desc, sum desc, name); `path` is
 * alphabetical. Both end on `localeCompare` so the order is total and
 * stable. */
export function compareWeightedPath(sortBy: SortBy): (left: WeightedPath, right: WeightedPath) => number {
  if (sortBy === "path") {
    return (left, right) => left.path.localeCompare(right.path);
  }
  return (left, right) =>
    right.maxWeight - left.maxWeight ||
    right.sumWeight - left.sumWeight ||
    left.path.localeCompare(right.path);
}
