// The one reader of action and workflow YAML the contract suites share.
// [ACTION-TESTS]
//
// The static contract checks, the branch-executed gate proof, and the release
// publish contract all need a step's shell body out of an action.yml or a
// workflow file — the step grammar is the same in both. A second copy of the block scanner
// would let the two drift on exactly the steps that decide whether a guard is
// under test at all, so the scanner lives here and is imported.
//
// Hand-scanned rather than regex-matched: this repo prohibits regex over
// source and structured data, and no YAML parser sits on the repo-root
// dependency path this suite runs from.

import assert from "node:assert/strict";

/** The sequence entry that opens a composite step. */
const STEP_MARKER = "- name: ";

/** The mapping key whose value is a step's shell script. */
const RUN_KEY = "run:";

/** Block-scalar indicators that introduce a multi-line `run:` body. */
const BLOCK_SCALARS = ["|", "|-", "|+", ">", ">-", ">+"];

/** Extra indentation a block scalar's content carries over its own key. */
const BLOCK_INDENT = 2;

/**
 * Column the first non-space character of `line` sits at.
 *
 * @param {string} line
 * @returns {number}
 */
export function leadingSpaces(line) {
  return line.length - line.trimStart().length;
}

/**
 * Index one past the last line of the step opening at `start`: the next
 * sequence entry at the same or shallower indentation, or end of file.
 *
 * @param {string[]} lines
 * @param {number} start
 * @param {number} indent
 * @returns {number}
 */
function stepEnd(lines, start, indent) {
  const next = lines.findIndex(
    (line, index) => index > start && line.trimStart().startsWith("- ") && leadingSpaces(line) <= indent,
  );
  return next === -1 ? lines.length : next;
}

/**
 * The `run:` value declared between `start` and `end`, dedented to column
 * zero so bash reads it as a script, or `null` when the step runs a `uses:`
 * action instead.
 *
 * @param {string[]} lines
 * @param {number} start
 * @param {number} end
 * @returns {{line: number, body: string} | null}
 */
function runBodyWithin(lines, start, end) {
  const at = lines.findIndex(
    (line, index) => index > start && index < end && line.trimStart().startsWith(RUN_KEY),
  );
  if (at === -1) return null;
  const value = lines[at].trimStart().slice(RUN_KEY.length).trim();
  if (!BLOCK_SCALARS.includes(value)) return { line: at, body: value };
  const indent = leadingSpaces(lines[at]) + BLOCK_INDENT;
  const body = [];
  for (const line of lines.slice(at + 1, end)) {
    if (line.trim() !== "" && leadingSpaces(line) < indent) break;
    body.push(line.slice(indent));
  }
  return { line: at, body: body.join("\n") };
}

/**
 * Every composite step that carries a shell body, in file order.
 *
 * @param {string} action the action.yml source
 * @returns {{name: string, line: number, body: string}[]} one entry per
 *   `run:` step, `line` being the 1-based line its `run:` key sits on
 */
export function runBodies(action) {
  const lines = action.split("\n");
  const steps = [];
  lines.forEach((line, index) => {
    if (!line.trimStart().startsWith(STEP_MARKER)) return;
    const found = runBodyWithin(lines, index, stepEnd(lines, index, leadingSpaces(line)));
    if (!found) return;
    steps.push({
      name: line.trimStart().slice(STEP_MARKER.length).trim(),
      line: found.line + 1,
      body: found.body,
    });
  });
  return steps;
}

/**
 * One named step's shell body, dedented to column zero.
 *
 * @param {string} action the action.yml source
 * @param {string} stepName the `- name:` value of the step to extract
 * @returns {string} the step's shell body
 */
export function stepBody(action, stepName) {
  const step = runBodies(action).find((candidate) => candidate.name === stepName);
  assert.ok(step, `action.yml lost its "${stepName}" run step`);
  return step.body;
}

/**
 * Every value declared for a repeated mapping key, in file order. Hand-scanned
 * like the rest of this module: the build matrix that names one
 * `vsix_target:` per platform is read this way rather than pattern-matched.
 *
 * @param {string} source the workflow or action YAML
 * @param {string} key the mapping key whose values to collect
 * @returns {string[]} each declared value, trimmed
 */
export function mappingValues(source, key) {
  const marker = key + ":";
  return source
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.startsWith(marker))
    .map((line) => line.slice(marker.length).trim())
    .filter((value) => value !== "");
}
