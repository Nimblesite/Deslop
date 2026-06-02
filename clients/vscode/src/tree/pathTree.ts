// Generic path trie shared by folder-mode grouping
// ([VSIX-TOP-OFFENDERS-FOLDER-MODE]) and the Duplication panel
// ([VSIX-METRICS-PANEL]). Splits display paths into nested folders and
// compresses single-child folder chains so a deep `a/b/c` run renders
// as one row. The leaf payload `T` is opaque — callers supply how to
// read a path from it and walk the returned tree into their own nodes.

/** A folder in the compressed path trie. The synthetic root has an
 * empty `label`/`path` and is never rendered itself; its `folders` and
 * `leaves` are the top-level rows. */
export interface PathTree<T> {
  /** Folder name relative to its parent, post-compression (e.g. `src/b`). */
  label: string;
  /** Full display path of this folder (`""` for the synthetic root). */
  path: string;
  folders: PathTree<T>[];
  leaves: T[];
}

interface MutableTree<T> {
  label: string;
  path: string;
  folders: Map<string, MutableTree<T>>;
  leaves: T[];
}

/** Builds a compressed path trie from `items`, keyed by `pathOf(item)`.
 * The synthetic root is returned uncompressed; every descendant folder
 * has single-child chains collapsed. */
export function buildPathTree<T>(items: T[], pathOf: (item: T) => string): PathTree<T> {
  const root: MutableTree<T> = { label: "", path: "", folders: new Map(), leaves: [] };
  for (const item of items) {
    const segments = pathOf(item).split(/[/\\]/).filter(Boolean);
    insertLeaf(root, segments, item);
  }
  return {
    label: "",
    path: "",
    folders: Array.from(root.folders.values()).map(freeze),
    leaves: root.leaves,
  };
}

/** Counts every leaf beneath `tree`, including nested folders. */
export function countLeaves<T>(tree: PathTree<T>): number {
  return tree.leaves.length + tree.folders.reduce((sum, child) => sum + countLeaves(child), 0);
}

function insertLeaf<T>(root: MutableTree<T>, segments: string[], item: T): void {
  const folders = segments.slice(0, -1);
  let node = root;
  let accumulated = "";
  for (const segment of folders) {
    accumulated = accumulated ? `${accumulated}/${segment}` : segment;
    let child = node.folders.get(segment);
    if (!child) {
      child = { label: segment, path: accumulated, folders: new Map(), leaves: [] };
      node.folders.set(segment, child);
    }
    node = child;
  }
  node.leaves.push(item);
}

/** Freezes a mutable node into the public shape, merging a no-leaf
 * single-subfolder chain into one compressed row. */
function freeze<T>(node: MutableTree<T>): PathTree<T> {
  let label = node.label;
  let current = node;
  while (current.leaves.length === 0 && current.folders.size === 1) {
    const child = current.folders.values().next().value as MutableTree<T>;
    label = `${label}/${child.label}`;
    current = child;
  }
  return {
    label,
    path: current.path,
    folders: Array.from(current.folders.values()).map(freeze),
    leaves: current.leaves,
  };
}
