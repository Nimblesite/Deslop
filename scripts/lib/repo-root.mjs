// The repository root, resolved once. [ACTION-TESTS]
//
// Every gate script needs an absolute path to the tree it is asserting
// against, and each one used to derive it from its own module URL. Nine
// copies of the same derivation is nine chances for one of them to drift a
// directory level after a script moves, and a gate pointed at the wrong root
// reads as "nothing to check" rather than as a failure.

import { fileURLToPath } from "node:url";

/** Absolute path to the repository root, with a trailing separator. */
export const repoRoot = fileURLToPath(new URL("../..", import.meta.url));
