// Static lint: nothing caller-influenced may reach `$GITHUB_PATH` or
// `$GITHUB_ENV`. [ACTION-ENVPATH]
//
// The runner reads both files back as the *next* step's PATH and environment.
// A value built from an action input, a step output, or a `${{ }}` expression
// therefore lets whoever controls that input decide where later steps resolve
// their executables, or what their environment says — the injection class
// CodeQL reports as `actions/envpath-injection`. The fix is always the same:
// export a constant path built from runner-owned variables and move the
// variable part of the layout to that constant, never the other way round.
//
// Runs in `make lint` so it fires on every CI run, not only when a workflow
// path filter happens to match. Proven by `scripts/test-env-path-writes.mjs`.

import { readdirSync, readFileSync } from "node:fs";
import { relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

/** Environment files the runner replays into the next step. */
const SINKS = ["GITHUB_PATH", "GITHUB_ENV"];

/**
 * Expansions the runner itself owns, and which no caller can influence.
 * Everything else — action inputs, step outputs, `${{ }}` expressions, job
 * env — is caller-influenced until proven otherwise, so it is rejected.
 */
const TRUSTED = new Set([
  "GITHUB_ACTION_PATH",
  "GITHUB_WORKSPACE",
  "HOME",
  "RUNNER_ARCH",
  "RUNNER_OS",
  "RUNNER_TEMP",
  "RUNNER_TOOL_CACHE",
  "RUNNER_WORKSPACE",
]);

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const WORKFLOW_DIR = resolve(repoRoot, ".github/workflows");

/**
 * Names the sink a line redirects into, or `""` when it writes neither.
 *
 * @param {string} line
 * @returns {string}
 */
export function redirectSink(line) {
  if (!line.includes(">>")) return "";
  return SINKS.find((sink) => line.includes(sink)) ?? "";
}

/**
 * The fragments a redirect writes: the body of a `{ … } >> "$SINK"` group, or
 * the part of the line preceding its own `>>`.
 *
 * @param {string[]} lines
 * @param {number} index Zero-based index of the redirecting line.
 * @returns {string[]}
 */
export function writtenFragments(lines, index) {
  const head = lines[index].slice(0, lines[index].indexOf(">>"));
  return head.trim() === "}" ? groupBody(lines, index) : [head];
}

function groupBody(lines, index) {
  const body = [];
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    if (lines[cursor].trim() === "{") return body;
    body.push(lines[cursor]);
  }
  throw new Error(`line ${index + 1} closes a redirect group that was never opened`);
}

/**
 * Every name a shell fragment expands — `$NAME`, `${NAME}` and whole `${{ }}`
 * expressions. Hand-scanned rather than regex-matched: this repo prohibits
 * regex over source and structured data.
 *
 * @param {string} fragment
 * @returns {string[]}
 */
export function expansions(fragment) {
  const names = [];
  for (let index = fragment.indexOf("$"); index >= 0; index = fragment.indexOf("$", index)) {
    const { name, length } = expansionAt(fragment, index);
    if (name) names.push(name);
    index += length;
  }
  return names;
}

function expansionAt(fragment, start) {
  if (fragment.startsWith("${{", start)) {
    const end = fragment.indexOf("}}", start);
    if (end < 0) return { name: "", length: 3 };
    return { name: fragment.slice(start, end + 2).trim(), length: end + 2 - start };
  }
  const from = fragment.startsWith("${", start) ? start + 2 : start + 1;
  const name = identifierAt(fragment, from);
  return { name, length: Math.max(1, from - start + name.length) };
}

function identifierAt(fragment, from) {
  let end = from;
  while (end < fragment.length && isIdentifierCharacter(fragment[end])) end += 1;
  return fragment.slice(from, end);
}

function isIdentifierCharacter(character) {
  const alphabetic = (character >= "a" && character <= "z") || (character >= "A" && character <= "Z");
  return alphabetic || (character >= "0" && character <= "9") || character === "_";
}

/**
 * Every caller-influenced expansion this source writes into a sink.
 *
 * @param {string} source Workflow or action YAML.
 * @param {string} label Path reported in the violation.
 * @returns {{file: string, line: number, sink: string, name: string}[]}
 */
export function envWriteViolations(source, label) {
  const lines = source.split("\n");
  return lines.flatMap((line, index) => {
    const sink = redirectSink(line);
    if (!sink) return [];
    return writtenFragments(lines, index)
      .flatMap(expansions)
      .filter((name) => !TRUSTED.has(name))
      .map((name) => ({ file: label, line: index + 1, sink, name }));
  });
}

/**
 * Every workflow and composite action this lint covers.
 *
 * @returns {string[]} Absolute paths.
 */
export function lintTargets() {
  const workflows = readdirSync(WORKFLOW_DIR)
    .filter((entry) => entry.endsWith(".yml") || entry.endsWith(".yaml"))
    .sort()
    .map((entry) => resolve(WORKFLOW_DIR, entry));
  return [resolve(repoRoot, "action.yml"), ...workflows];
}

function main() {
  const targets = lintTargets();
  const violations = targets.flatMap((target) =>
    envWriteViolations(readFileSync(target, "utf8"), relative(repoRoot, target)),
  );
  for (const { file, line, sink, name } of violations) {
    console.error(`${file}:${line}: ${sink} is written from ${name}, which a caller can influence`);
  }
  if (violations.length > 0) {
    console.error(
      `\n${violations.length} PATH/env injection(s). ${SINKS.join(" and ")} are replayed as the next ` +
        "step's PATH and environment: export a constant built from runner-owned variables and move the " +
        "variable part of the layout to it. See docs/specs/release.md [ACTION-ENVPATH].",
    );
    process.exit(1);
  }
  console.log(`env/PATH write lint: ${targets.length} files clean`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
