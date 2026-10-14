// [CORPUS-REGISTER-TYPE] [CORPUS-REGISTER-MERGE] Drives the merge over judges
// who name clone types, and asserts which type, if any, each register entry
// carries afterwards. Black box, like `verdict-merge.test.mjs`: every assertion
// reads a file the script wrote.

import { strict as assert } from "node:assert";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  EMPTY_VERDICTS,
  FIRST_JUDGE,
  JUDGED,
  JUDGED_REGISTER,
  REGISTER,
  REGISTER_DIR,
  REPORT,
  SECOND_JUDGE,
  STANDING,
  WHY,
  at,
  fixtureRoot,
  judgingFolder,
  noted,
  run,
  runPasses,
  scored,
  write,
} from "./verdict-merge-fixture.mjs";

// [CORPUS-REGISTER-TYPE] A judge names a CLEARLY IN's clone type. The verdict
// alone decides whether a pair is imported; the type travels with it only when
// every source gave the same one, and is otherwise left unspecified.
const TYPE_ONE = "Type I";
const TYPE_TWO = "Type II";
const MALFORMED_TYPE = "Type 2";
const UNSPECIFIED_SECTION = "## types_left_unspecified";
const typed = (entry, type) => ({ ...entry, type });

/// The rows of the report's unspecified-type table, so assertions about it
/// cannot be satisfied by text elsewhere in the document.
const unspecifiedRows = (report) => report.slice(report.indexOf(UNSPECIFIED_SECTION));

test("judges who agree on a clone type file it with the pair", () => {
  const pass = { ...EMPTY_VERDICTS, clearly_in: [typed(scored(1), TYPE_TWO), scored(5)] };
  const { status, json, read } = runPasses({ [FIRST_JUDGE]: pass, [SECOND_JUDGE]: pass });
  assert.equal(status, 0);
  const register = json(join(REGISTER_DIR, REGISTER));
  assert.equal(register.clearly_in.length, 2, "both agreed pairs are imported");
  assert.deepEqual(register.clearly_in[0].occurrences, at(1));
  assert.equal(register.clearly_in[0].type, TYPE_TWO, "the agreed type travels with the pair");
  assert.deepEqual(
    Object.keys(register.clearly_in[0]),
    ["why", "verified", "type", "occurrences"],
    "the type sits beside the prose, before the ranges",
  );
  assert.equal(register.clearly_in[0].why, WHY, "the judge's words are still copied verbatim");
  assert.deepEqual(register.clearly_in[1].occurrences, at(5));
  assert.equal(register.clearly_in[1].type, undefined, "two blank types are written as no type");
  const report = read(REPORT);
  assert.ok(report.includes("| pairs_left_out | 0 |"), "nothing was disputed");
  assert.ok(report.includes("| types_unspecified | 0 |"), "every typed pair kept its type");
  assert.ok(unspecifiedRows(report).includes("(none)"));
});

test("judges who split on clone type still file the pair, without a type", () => {
  const { status, json, read } = runPasses({
    [FIRST_JUDGE]: { ...EMPTY_VERDICTS, clearly_in: [typed(scored(1), TYPE_ONE), typed(scored(5), TYPE_TWO)] },
    [SECOND_JUDGE]: { ...EMPTY_VERDICTS, clearly_in: [typed(scored(1), TYPE_TWO), scored(5)] },
  });
  assert.equal(status, 0);
  const register = json(join(REGISTER_DIR, REGISTER));
  assert.deepEqual(
    register.clearly_in.map((entry) => entry.occurrences),
    [at(1), at(5)],
    "the verdict was agreed, so both pairs are imported",
  );
  assert.deepEqual(
    register.clearly_in.map((entry) => entry.type),
    [undefined, undefined],
    "a type the judges did not agree on is not specified",
  );
  const report = read(REPORT);
  assert.ok(report.includes("| pairs_left_out | 0 |"), "a type split leaves no pair out");
  assert.ok(report.includes("| types_unspecified | 2 |"), report);
  const rows = unspecifiedRows(report);
  assert.ok(rows.includes(`${FIRST_JUDGE}=${TYPE_ONE}<br>${SECOND_JUDGE}=${TYPE_TWO}`), rows);
  assert.ok(rows.includes(`${FIRST_JUDGE}=${TYPE_TWO}<br>${SECOND_JUDGE}=blank`), "a blank is a vote too");
  for (const range of [...at(1), ...at(5)]) assert.ok(rows.includes(range), `${range} is named`);
});

test("a malformed type is never filed, and other verdicts never carry one", () => {
  const pass = {
    clearly_in: [typed(scored(1), MALFORMED_TYPE)],
    clearly_out: [typed(scored(3), TYPE_ONE)],
    not_clear: [typed(noted(2), TYPE_ONE)],
  };
  const { status, json, read } = runPasses({ [FIRST_JUDGE]: pass, [SECOND_JUDGE]: pass });
  assert.equal(status, 0);
  const register = json(join(REGISTER_DIR, REGISTER));
  assert.deepEqual(register.clearly_in.map((entry) => entry.occurrences), [at(1)]);
  assert.equal(register.clearly_in[0].type, undefined, "an unknown label is not a type");
  assert.deepEqual(register.clearly_out.map((entry) => entry.occurrences), [at(3)]);
  assert.equal(register.clearly_out[0].type, undefined, "a pair that is not a clone has no type");
  assert.deepEqual(register.not_clear.map((entry) => entry.occurrences), [at(2)]);
  assert.equal(register.not_clear[0].type, undefined);
  const rows = unspecifiedRows(read(REPORT));
  assert.ok(rows.includes(`${FIRST_JUDGE}=${MALFORMED_TYPE}<br>${SECOND_JUDGE}=${MALFORMED_TYPE}`), rows);
});

test("a typed pass over an untyped standing entry leaves it untyped", () => {
  const root = mkdtempSync(join(tmpdir(), "verdict-merge-typed-standing-"));
  fixtureRoot(root);
  const pass = { ...EMPTY_VERDICTS, clearly_in: [typed(scored(6), TYPE_ONE)] };
  const folders = [FIRST_JUDGE, SECOND_JUDGE].map((judge) =>
    judgingFolder(root, judge, { [JUDGED]: pass }),
  );
  const { status, json, read } = run(root, folders);
  assert.equal(status, 0);
  const standing = json(join(REGISTER_DIR, JUDGED_REGISTER));
  assert.equal(standing.clearly_in.length, 1, "nothing is added beside the standing entry");
  assert.deepEqual(standing.clearly_in[0].occurrences, STANDING);
  assert.equal(standing.clearly_in[0].type, undefined, "the register's blank disagrees, so no type");
  const report = read(REPORT);
  assert.ok(!report.includes("register_conflict"), "the verdicts agree; only the type differs");
  assert.ok(report.includes("| types_unspecified | 1 |"), report);
  assert.ok(unspecifiedRows(report).includes(`register=blank<br>${FIRST_JUDGE}=${TYPE_ONE}`));
});

test("a standing type the judges contradict is no longer specified", () => {
  const root = mkdtempSync(join(tmpdir(), "verdict-merge-retyped-standing-"));
  fixtureRoot(root);
  const path = join(root, REGISTER_DIR, JUDGED_REGISTER);
  const register = JSON.parse(readFileSync(path, "utf8"));
  register.clearly_in[0] = typed(register.clearly_in[0], TYPE_ONE);
  write(path, register);
  const pass = { ...EMPTY_VERDICTS, clearly_in: [typed(scored(6), TYPE_TWO), typed(scored(1), TYPE_ONE)] };
  const folders = [FIRST_JUDGE, SECOND_JUDGE].map((judge) =>
    judgingFolder(root, judge, { [JUDGED]: pass }),
  );
  const { status, json, read } = run(root, folders);
  assert.equal(status, 0);
  const merged = json(join(REGISTER_DIR, JUDGED_REGISTER));
  assert.deepEqual(merged.clearly_in.map((entry) => entry.occurrences), [STANDING, at(1)]);
  assert.equal(merged.clearly_in[0].type, undefined, "Type I against Type II: not specified");
  assert.equal(merged.clearly_in[0].why, WHY, "the standing entry is otherwise untouched");
  assert.equal(merged.clearly_in[1].type, TYPE_ONE, "the new pair the judges agree on keeps its type");
  const rows = unspecifiedRows(read(REPORT));
  assert.ok(rows.includes(`register=${TYPE_ONE}<br>${FIRST_JUDGE}=${TYPE_TWO}`), rows);
});
