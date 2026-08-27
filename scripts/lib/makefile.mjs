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

/// Make's line continuation. A declaration ending in one carries on to the
/// next line, and a reader that stops at the newline sees a fraction of the
/// value plus a stray backslash.
const CONTINUATION = "\\";

/// The three characters make and the shell treat as word separators, so
/// splitting never needs a pattern match.
const SEPARATORS = ["\n", "\t", " "];

/**
 * Split `text` into words on whitespace, without a pattern match.
 *
 * @param {string} text
 * @returns {string[]}
 */
export function words(text) {
  return SEPARATORS.reduce(
    (parts, separator) => parts.flatMap((part) => part.split(separator)),
    [text],
  ).filter((word) => word.length > 0);
}

/**
 * The whole right-hand side of a make variable, as words.
 *
 * `?=`, `=` and `:=` all count, and the name is matched at the start of its own
 * declaration so a variable is read where it is declared rather than wherever
 * its name happens to appear. Backslash continuations are joined first:
 * `CORPUS_TESTS_FULL` spans four lines, and a reader that stopped at the first
 * newline saw three of its eleven names and a literal `\` — the tail was never
 * checked against anything (gh #412).
 *
 * @param {string} name
 * @returns {string[]} every word assigned, `[]` when the variable is undeclared
 */
export function variableWords(name) {
  const start = makefileLines.findIndex(
    (line) => line.startsWith(`${name} `) || line.startsWith(`${name}=`),
  );
  if (start < 0) return [];
  const declaration = [];
  for (let index = start; index < makefileLines.length; index += 1) {
    const line = makefileLines[index];
    declaration.push(line);
    if (!line.trimEnd().endsWith(CONTINUATION)) break;
  }
  const [, value] = declaration.join(" ").split("=");
  return words(value ?? "").filter((word) => word !== CONTINUATION);
}
