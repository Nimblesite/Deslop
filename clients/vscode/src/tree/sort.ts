// [VSIX-TOP-OFFENDERS-SORT] Fixed highest-mass-first ordering.
// Display ordering never changes a cluster's engine-provided rank.

/** Ordering keys shared by file and folder rows. */
export interface MassPath {
  path: string;
  /** The row's worst cluster mass, read from the engine. */
  mass: number;
  /** A tie break over engine values; never displayed as a duplication figure. */
  massTotal: number;
}

/** Descending worst mass, then total mass, with a deterministic path tie break. */
export function compareMassPath(): (left: MassPath, right: MassPath) => number {
  return (left, right) =>
    right.mass - left.mass ||
    right.massTotal - left.massTotal ||
    left.path.localeCompare(right.path);
}
