// What the accuracy gate scans. [CORPUS-SCORE-GATE-JUDGED]
//
// The gate can only score pairs a judge ruled on, so it scans repositories
// whose register judges at least one pair CLEARLY IN or CLEARLY OUT, and
// nothing else. A target list it cannot read in full fails the run: a register
// that broke the listing half way used to shorten the scan silently, and a
// shorter scan — down to none at all — passed. These tests drive the gate's own
// target resolution against registers written to a temporary root. Run with
// `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

import { repoRoot } from "../lib/repo-root.mjs";

const SCRIPT = "scripts/corpus/score-gate.sh";
const REGISTER_DIR = "corpus/register";
const NOT_A_REPOSITORY = "corpus/not-a-repository.json";
const COMMIT_ID_LENGTH = 40;
/// The flag that scans every judged register.
const ALL_JUDGED = "--all-judged";
/// The exit status of a run the gate refused.
const REFUSED = 1;
/// Prints the URL of every target the gate resolved, one per line.
const PRINT_TARGETS = `printf '%s\\n' "\${TARGET_URLS[@]}"`;

const LANGUAGE = "go";
const JUDGED = [{ why: "judged" }];
/// The register a broken fixture is written as, and the text of one that is
/// not JSON at all.
const BROKEN = "beta";
const UNREADABLE = "{";

/// A register for `name`, pinned to a commit of one repeated hex `digit`,
/// holding `verdicts`.
const repository = (name, digit, verdicts) => ({
  name,
  url: `https://example.invalid/${name}.git`,
  sha: digit.repeat(COMMIT_ID_LENGTH),
  language: LANGUAGE,
  ...verdicts,
});
const JUDGED_IN = repository("alpha", "a", { clearly_in: JUDGED });
const JUDGED_OUT = repository("gamma", "c", { clearly_out: JUDGED });
const UNDECIDED = repository("delta", "d", { not_clear: JUDGED });
const NO_URL = { ...repository(BROKEN, "b", { clearly_in: JUDGED }), url: "" };
const UNREGISTERED = repository("omega", "e", {});

const spec = (register) => `${register.url}#${register.sha}#${register.language}`;

/// A repository root whose register directory holds `registers`, each a
/// register object or the raw text of an unreadable one, keyed by stem.
const rootWith = (registers) => {
  const root = mkdtempSync(join(tmpdir(), "gate-targets-"));
  mkdirSync(join(root, REGISTER_DIR), { recursive: true });
  writeFileSync(join(root, NOT_A_REPOSITORY), JSON.stringify({ files: [] }));
  for (const [name, register] of Object.entries(registers)) {
    const text = typeof register === "string" ? register : JSON.stringify(register);
    writeFileSync(join(root, REGISTER_DIR, `${name}.json`), text);
  }
  return root;
};
const named = (...registers) => Object.fromEntries(registers.map((register) => [register.name, register]));

/// Resolves the gate's targets under `root` with `args`, after `setup`, and
/// prints them. The gate's own script is sourced, so this is its resolution.
const resolve = (root, args, setup = "") =>
  spawnSync(
    "bash",
    ["-c", `source "${SCRIPT}"; REPO_ROOT="$1"; shift; ${setup} resolve_targets "$@"; ${PRINT_TARGETS}`,
      "score-gate", root, ...args],
    { cwd: repoRoot, encoding: "utf8" },
  );
const slice = (...names) => `DEFAULT_SLICE=(${names.join(" ")});`;
const urls = (result) => result.stdout.split("\n").filter(Boolean);

test("[CORPUS-SCORE-GATE-JUDGED] the gate scans every judged register, and nothing else", () => {
  const root = rootWith(named(JUDGED_IN, JUDGED_OUT, UNDECIDED));
  const everyJudged = resolve(root, [ALL_JUDGED]);
  assert.equal(everyJudged.status, 0, everyJudged.stderr);
  assert.deepEqual(urls(everyJudged), [JUDGED_IN.url, JUDGED_OUT.url], "the undecided register is not scanned");
  const theSlice = resolve(root, [], slice(JUDGED_IN.name, JUDGED_OUT.name));
  assert.equal(theSlice.status, 0, theSlice.stderr);
  assert.deepEqual(urls(theSlice), [JUDGED_IN.url, JUDGED_OUT.url]);
  const chosen = resolve(root, [spec(JUDGED_OUT)]);
  assert.equal(chosen.status, 0, chosen.stderr);
  assert.deepEqual(urls(chosen), [JUDGED_OUT.url]);
});

test("[CORPUS-SCORE-GATE-JUDGED] a repository no register judges is refused, never scanned", () => {
  const root = rootWith(named(JUDGED_IN, UNDECIDED));
  for (const [what, result] of [
    ["an undecided target", resolve(root, [spec(JUDGED_IN), spec(UNDECIDED)])],
    ["an unregistered target", resolve(root, [spec(UNREGISTERED)])],
    ["an undecided slice", resolve(root, [], slice(JUDGED_IN.name, UNDECIDED.name))],
  ]) {
    assert.equal(result.status, REFUSED, `${what} must fail the gate:\n${result.stdout}`);
    assert.deepEqual(urls(result), [], `${what} resolves to no scan at all`);
  }
  const refusal = resolve(root, [spec(UNDECIDED)]).stderr;
  assert.ok(refusal.includes(spec(UNDECIDED)), `the refusal names the target:\n${refusal}`);
});

test("[CORPUS-SCORE-GATE-JUDGED] a register that cannot be read fails the gate rather than shortening it", () => {
  for (const [what, broken] of [
    ["an unreadable register", { [BROKEN]: UNREADABLE }],
    ["a register with no url", named(NO_URL)],
  ]) {
    const root = rootWith({ ...named(JUDGED_IN, JUDGED_OUT), ...broken });
    for (const [how, result] of [
      ["every judged register", resolve(root, [ALL_JUDGED])],
      ["the default slice", resolve(root, [], slice(JUDGED_IN.name, BROKEN, JUDGED_OUT.name))],
    ]) {
      assert.equal(result.status, REFUSED, `${what} must fail ${how}:\n${result.stdout}`);
      assert.deepEqual(urls(result), [], `${what} must not leave ${how} a shorter scan`);
    }
  }
});

test("[CORPUS-SCORE-GATE-JUDGED] a gate with nothing judged to scan fails", () => {
  const root = rootWith(named(UNDECIDED));
  for (const [how, result] of [
    ["every judged register", resolve(root, [ALL_JUDGED])],
    ["an empty slice", resolve(root, [], slice())],
  ]) {
    assert.equal(result.status, REFUSED, `${how} with no judgment must fail:\n${result.stdout}`);
    assert.deepEqual(urls(result), []);
  }
});
