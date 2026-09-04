// [CORPUS-REGISTER-MERGE] Drives `scripts/corpus/merge-verdicts.mjs` against a
// throwaway pair of judging folders and asserts what reaches the register and
// what is refused entry.
//
// Black box on purpose: the script is run as a process, and every assertion
// reads a file it wrote. A register is the independent evidence that this
// detector is accurate, so the interesting cases are all the ones where a
// verdict must NOT get in — judges who split, a judge who cites ranges the
// candidate never showed, a pass that contradicts a standing entry.

import { strict as assert } from "node:assert";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const SCRIPT = join(REPO_ROOT, "scripts", "corpus", "merge-verdicts.mjs");
const REPORT = "docs/reports/verdict-merge.md";
const REGISTER_DIR = "corpus/register";
const QUEUE = "corpus/judging-queue.json";

/// The fixture repository, and the two judges sent to read it.
const REPO = "widgets";
const REGISTER = "widgets.json";
const LANGUAGE = "rust";
const URL = "https://example.invalid/widgets.git";
const SHA = "0123456789abcdef0123456789abcdef01234567";
const FIRST_JUDGE = "judge-one";
const SECOND_JUDGE = "judge-two";

/// A repository that already has a register, so a standing verdict can be
/// contradicted by a later pass.
const JUDGED = "gadgets";
const JUDGED_REGISTER = "gadgets.json";

/// Prose long enough to clear the assertion floor a scored entry is held to.
const WHY = "The same twenty lines appear twice with only the type name changed.";
const VERIFIED = "diff of the two ranges: identical but for the type name on line 3.";
/// A NOT CLEAR note, deliberately shorter than that floor. NOT CLEAR asserts
/// nothing, so it must still be recorded — otherwise the next pass re-reads a
/// pair somebody has already ruled on.
const TERSE_NOTE = "2-line idiom.";
/// The note both judges write about the pair the standing register already
/// calls CLEARLY IN, so the merge has to choose between them.
const STANDING_NOTE = "Copied text is real but the boundaries are ragged.";

/// The candidates every judge is shown. Same list for both, or their candidate
/// numbers would name different pairs.
const PAIRS = {
  pairs: [
    { number: 1, occurrences: ["src/a.rs:10-30", "src/b.rs:10-30"] },
    { number: 2, occurrences: ["src/c.rs:1-4", "src/d.rs:1-4"] },
    { number: 3, occurrences: ["src/e.rs:5-9", "src/f.rs:5-9"] },
    { number: 4, occurrences: ["src/g.rs:1-2", "src/h.rs:1-2"] },
    { number: 5, occurrences: ["src/i.rs:1-9", "src/j.rs:1-9"] },
    { number: 6, occurrences: ["src/k.rs:1-9", "src/l.rs:1-9"] },
  ],
};
/// The pair the standing register already calls CLEARLY IN. Both judges call
/// it NOT CLEAR, so the merge meets a verdict it must report and not apply.
const STANDING = ["src/k.rs:1-9", "src/l.rs:1-9"];

const at = (pair) => PAIRS.pairs.find((entry) => entry.number === pair).occurrences;
const scored = (candidate) => ({ candidate, why: WHY, verified: VERIFIED, occurrences: at(candidate) });
const noted = (candidate, why = TERSE_NOTE) => ({ candidate, why, occurrences: at(candidate) });

/// What each judge filed. Between them they cover every outcome the script has
/// to tell apart.
//   1 both agree CLEARLY IN          -> merged
//   2 both agree NOT CLEAR, tersely  -> merged, note and all
//   3 CLEARLY OUT against CLEARLY IN -> flat contradiction, merged nowhere
//   4 NOT CLEAR against CLEARLY IN   -> confidence split, merged nowhere
//   5 second judge cites ranges the candidate never showed -> refused
//   6 both agree NOT CLEAR, but the register already calls it CLEARLY IN
const VERDICTS_BY_JUDGE = {
  [FIRST_JUDGE]: {
    clearly_in: [scored(1)],
    clearly_out: [scored(3)],
    not_clear: [noted(2), noted(4), noted(5), noted(6, STANDING_NOTE)],
  },
  [SECOND_JUDGE]: {
    clearly_in: [scored(1), scored(3), scored(4)],
    clearly_out: [],
    not_clear: [
      noted(2),
      { candidate: 5, why: TERSE_NOTE, occurrences: ["src/zz.rs:1-1", "src/yy.rs:1-1"] },
      noted(6, STANDING_NOTE),
    ],
  },
};

const write = (path, value) => {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, typeof value === "string" ? value : `${JSON.stringify(value, null, 2)}\n`);
};

/// A judging folder holding one workspace per repository, as the preparer
/// builds it.
const judgingFolder = (root, judge, repos) => {
  const folder = join(root, judge);
  for (const [slug, verdicts] of Object.entries(repos)) {
    const workspace = join(folder, slug);
    write(join(workspace, "PINNED.txt"), `${URL}\n${SHA}\n`);
    write(join(workspace, "candidates", "pairs.json"), PAIRS);
    write(join(workspace, "verdicts.json"), verdicts);
  }
  return folder;
};

/// A repository root with a queue, one standing register, and nothing else.
const fixtureRoot = (root) => {
  write(join(root, QUEUE), {
    why: "Repositories waiting on a first judging pass.",
    repositories: [{ name: REPO, url: URL, sha: SHA, language: LANGUAGE, why: "First pass." }],
  });
  write(join(root, REGISTER_DIR, JUDGED_REGISTER), {
    name: JUDGED,
    language: LANGUAGE,
    url: URL,
    sha: SHA,
    protocol: { spec: "docs/specs/corpus.md" },
    clearly_in: [{ why: WHY, verified: VERIFIED, occurrences: STANDING }],
    clearly_out: [],
    clearly_out_status: "NONE FOUND. Nothing in this repository reached CLEARLY OUT in any pass.",
    not_clear: [],
  });
};

/// Runs the merge over a prepared root, capturing the exit status rather than
/// throwing on it: a run that finds a disagreement is meant to fail.
const run = (root, folders, extra = []) => {
  const result = spawnSync("node", [SCRIPT, "--root", root, ...extra, ...folders], {
    encoding: "utf8",
  });
  const read = (path) => readFileSync(join(root, path), "utf8");
  return {
    root,
    status: result.status,
    stdout: result.stdout,
    stderr: result.stderr,
    read,
    json: (path) => JSON.parse(read(path)),
  };
};

/// Builds a fixture whose judges disagree, and runs the merge over it.
const runDisputed = (extra = []) => {
  const root = mkdtempSync(join(tmpdir(), "verdict-merge-"));
  fixtureRoot(root);
  const folders = [FIRST_JUDGE, SECOND_JUDGE].map((judge) =>
    judgingFolder(root, judge, {
      [REPO]: VERDICTS_BY_JUDGE[judge],
      [JUDGED]: VERDICTS_BY_JUDGE[judge],
    }),
  );
  return run(root, folders, extra);
};

/// Builds a fixture the judges agree on completely: candidate 1 CLEARLY IN,
/// candidate 2 NOT CLEAR, and nothing else ruled on.
const AGREED = {
  clearly_in: [scored(1)],
  clearly_out: [],
  not_clear: [noted(2)],
};

const runAgreed = (extra = []) => {
  const root = mkdtempSync(join(tmpdir(), "verdict-merge-agreed-"));
  fixtureRoot(root);
  const folders = [FIRST_JUDGE, SECOND_JUDGE].map((judge) =>
    judgingFolder(root, judge, { [REPO]: AGREED }),
  );
  return run(root, folders, extra);
};

test("only the pairs every source agrees on are imported", () => {
  const { status, stdout, json } = runDisputed();
  assert.equal(status, 0, "disputed pairs are left out, they do not stop the run");
  assert.match(stdout, /pair\(s\) left out: not every source agreed/);

  const register = json(join(REGISTER_DIR, REGISTER));
  const filed = [...register.clearly_in, ...register.clearly_out, ...register.not_clear];
  const has = (candidate) =>
    filed.some((entry) => JSON.stringify(entry.occurrences) === JSON.stringify(at(candidate)));
  assert.ok(has(1), "candidate 1: both judges said CLEARLY IN, so it is imported");
  assert.ok(has(2), "candidate 2: both judges said NOT CLEAR, so it is imported");
  assert.ok(!has(3), "candidate 3: CLEARLY IN against CLEARLY OUT is never imported");
  assert.ok(!has(4), "candidate 4: one judge firm, one NOT CLEAR, is never imported");
  assert.ok(!has(5), "candidate 5: one judge's ranges were not the candidate's");
  assert.equal(register.clearly_in.length, 1, "nothing else reached a scored verdict");
  assert.equal(register.clearly_out.length, 0);
});

test("a pair the register already holds is left exactly as it was", () => {
  const { json } = runDisputed();
  const standing = json(join(REGISTER_DIR, JUDGED_REGISTER));
  assert.deepEqual(
    standing.clearly_in[0].occurrences,
    STANDING,
    "the standing entry keeps its verdict though both judges now say NOT CLEAR",
  );
  assert.ok(
    !standing.not_clear.some(
      (entry) => JSON.stringify(entry.occurrences) === JSON.stringify(STANDING),
    ),
    "and it is not filed a second time under the verdict the register disagrees with",
  );
  assert.equal(standing.clearly_in.length, 2, "the pair every source agreed on is added beside it");
});

test("the report is data: derived labels, counts, and the judges' own words", () => {
  const { read } = runDisputed();
  const report = read(REPORT);
  const kinds = ["clearly_in/clearly_out", "clearly_in/not_clear", "occurrences_mismatch"];
  for (const kind of kinds) assert.ok(report.includes(kind), `${kind} is a label read off the verdicts`);
  const rows = report.slice(report.indexOf("## disagreements"));
  const first = rows.indexOf("clearly_in/clearly_out");
  const softer = rows.indexOf("clearly_in/not_clear");
  assert.ok(first > 0 && softer > first, "rows sort by kind, so the irreconcilable split comes first");
  for (const range of at(3)) assert.ok(report.includes(range), "the opposed pair's ranges are printed");
  assert.ok(
    report.includes(`${FIRST_JUDGE}=clearly_out`) && report.includes(`${SECOND_JUDGE}=clearly_in`),
    "each source is named beside the verdict it gave",
  );
  assert.ok(report.includes(WHY), "the judge's own reasoning is quoted, not summarised");
});

test("the report counts every kind, and totals what went in and what stayed out", () => {
  const { read } = runDisputed();
  const report = read(REPORT);
  for (const [kind, count] of [
    ["clearly_in/clearly_out", 2],
    ["clearly_in/not_clear", 2],
    ["occurrences_mismatch", 2],
    ["register_conflict", 1],
  ]) {
    assert.ok(report.includes(`| ${kind} | ${count} |`), `by_kind must report ${kind} = ${count}`);
  }
  assert.ok(report.includes("| pairs_left_out | 7 |"), "seven pairs were not agreed on by every source");
  // widgets takes candidates 1, 2 and 6; gadgets takes 1 and 2, because the
  // register already holds candidate 6's pair under a verdict the judges deny.
  assert.ok(report.includes("| pairs_imported | 5 |"), "five were, across the two repositories");
  assert.ok(!report.includes("states too little"), "thin prose is not a disagreement between sources");
});

test("a run nothing disagreed on reports an empty disagreement table", () => {
  const { read } = runAgreed();
  const report = read(REPORT);
  assert.ok(report.includes("| pairs_left_out | 0 |"), "every pair was agreed on by every source");
  assert.ok(report.includes("| pairs_imported | 2 |"));
  assert.ok(report.slice(report.indexOf("## disagreements")).includes("(none)"));
});

test("judges who agree completely get merged, in their own words", () => {
  const { status, json } = runAgreed();
  assert.equal(status, 0, "nothing was disputed, so the merge runs");
  const register = json(join(REGISTER_DIR, REGISTER));
  assert.equal(register.name, REPO, "the register is named for the repository it judges");
  assert.equal(register.language, LANGUAGE, "the language comes from the judging queue");
  assert.equal(register.sha, SHA, "the register pins the commit the judges actually read");
  assert.equal(register.url, URL);
  assert.equal(
    register.protocol.judging_skill,
    ".agents/skills/judge-clone-pairs/SKILL.md",
    "a register must cite the protocol that produced it, or nobody can re-judge it the same way",
  );
  assert.equal(register.clearly_in.length, 1);
  assert.deepEqual(register.clearly_in[0].occurrences, at(1));
  assert.equal(register.clearly_in[0].why, WHY, "the judge's words are copied, never rewritten");
  assert.equal(register.clearly_in[0].verified, VERIFIED);
});

test("an agreed NOT CLEAR note shorter than the assertion floor is still recorded", () => {
  const { json } = runAgreed();
  const register = json(join(REGISTER_DIR, REGISTER));
  assert.deepEqual(
    register.not_clear.map((entry) => entry.why),
    [TERSE_NOTE],
    "NOT CLEAR asserts nothing, so it is held to a note rather than to the assertion floor",
  );
  assert.ok(
    TERSE_NOTE.length < WHY.length,
    "the note under test really is shorter than what a scored verdict must state",
  );
  assert.equal(
    register.not_clear[0].verified,
    undefined,
    "NOT CLEAR carries no `verified`: there is no assertion to have verified",
  );
});

test("a repository that gains a register leaves the judging queue", () => {
  const { json } = runAgreed();
  assert.deepEqual(json(QUEUE).repositories, [], "a queue that never drains buys nothing");
});

test("--dry-run merges nothing even when every judge agrees", () => {
  const { root, json, stdout } = runAgreed(["--dry-run"]);
  assert.match(stdout, /dry run; nothing written/);
  assert.equal(json(QUEUE).repositories.length, 1, "the queue is untouched by a dry run");
  assert.throws(
    () => readFileSync(join(root, REGISTER_DIR, REGISTER), "utf8"),
    "no register is created by a dry run",
  );
});

test("judges shown different candidate lists are refused outright", () => {
  const root = mkdtempSync(join(tmpdir(), "verdict-merge-split-"));
  fixtureRoot(root);
  const folders = [FIRST_JUDGE, SECOND_JUDGE].map((judge) =>
    judgingFolder(root, judge, { [REPO]: VERDICTS_BY_JUDGE[judge] }),
  );
  write(join(folders[1], REPO, "candidates", "pairs.json"), {
    pairs: [{ number: 1, occurrences: ["src/other.rs:1-9", "src/another.rs:1-9"] }],
  });
  assert.throws(
    () => execFileSync("node", [SCRIPT, "--root", root, ...folders], { encoding: "utf8" }),
    /were shown different candidate lists/,
    "cross-matching two candidate numbers that name different pairs fabricates disagreements",
  );
  rmSync(root, { recursive: true, force: true });
});

test("one judging pass is never enough to record a verdict", () => {
  const root = mkdtempSync(join(tmpdir(), "verdict-merge-lonely-"));
  fixtureRoot(root);
  const folder = judgingFolder(root, FIRST_JUDGE, { [REPO]: VERDICTS_BY_JUDGE[FIRST_JUDGE] });
  assert.throws(
    () => execFileSync("node", [SCRIPT, "--root", root, folder], { encoding: "utf8" }),
    /independent passes are/,
    "one reader having a firm opinion is an opinion; two arriving at it separately is evidence",
  );
  rmSync(root, { recursive: true, force: true });
});
