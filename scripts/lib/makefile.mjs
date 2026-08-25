// The repository Makefile, read line-exactly for contract tests.
//
// Three gates ([CI-DESLOP], [TEST-SELECTION], [CI-COVERAGE-ISOLATION]) assert
// what a make target actually runs, and each one used to carry its own copy of
// the same block-finding code. Deslop detects duplication; its own tooling gets
// one implementation. Everything here is line-exact string work — never a
// pattern match over recipe text.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { repoRoot } from "./repo-root.mjs";

/// Every line of the Makefile, in order.
export const makefileLines = readFileSync(resolve(repoRoot, "Makefile"), "utf8").split("\n");

/// True when `line` continues a recipe rather than starting a new declaration.
/// Make ends a block at the first non-blank line in column 0.
function continuesRecipe(line) {
  return line.length === 0 || line.startsWith("\t") || line.startsWith(" ");
}

/**
 * Every block declaring `target`, in file order. A target may be declared more
 * than once — once to export an environment variable, once for the recipe — so
 * a caller that reads only the first block can miss half of what runs.
 *
 * @param {string} target
 * @returns {Array<{header: string, body: string}>}
 */
export function recipeBlocks(target) {
  return makefileLines.flatMap((line, index) => {
    if (!line.startsWith(`${target}:`)) return [];
    const rest = makefileLines.slice(index + 1);
    const end = rest.findIndex((next) => !continuesRecipe(next));
    return [{ header: line, body: (end < 0 ? rest : rest.slice(0, end)).join("\n") }];
  });
}
