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

test("one disagreement stops the whole merge, and nothing is written", () => {
  const { status, stderr, root, json } = runDisputed();
  assert.equal(status, 1, "a run that found a disagreement must fail, not succeed quietly");
  assert.match(stderr, /pair\(s\) disagree\. NOTHING was merged\./);
  assert.throws(
    () => readFileSync(join(root, REGISTER_DIR, REGISTER), "utf8"),
    "no register may be created while a judge is demonstrably unreliable",
  );
  const standing = json(join(REGISTER_DIR, JUDGED_REGISTER));
  assert.equal(standing.clearly_in.length, 1, "the existing register is left exactly as it was");
  assert.equal(standing.not_clear.length, 0);
  assert.equal(json(QUEUE).repositories.length, 1, "the judging queue is not drained either");
});

test("the report leads with opposite conclusions, in each judge's own words", () => {
  const { read } = runDisputed();
  const report = read(REPORT);
  const opposite = report.indexOf("## Opposite conclusions");
  const softer = report.indexOf("## One judge committed, the other would not");
  assert.ok(opposite > 0 && softer > opposite, "the irreconcilable split is reported first");
  assert.match(report, /Nothing was merged/);
  const contradicted = at(3).map((range) => report.includes(range));
  assert.deepEqual(contradicted, [true, true], "both ranges of the opposed pair are printed");
  assert.ok(
    report.includes(`${FIRST_JUDGE} — clearly_out`) && report.includes(`${SECOND_JUDGE} — clearly_in`),
    "each judge is named beside the verdict they reached",
  );
  assert.ok(report.includes(WHY), "the judge's own reasoning is quoted, not summarised");
});

test("the report counts every kind of disagreement, and carries nothing else", () => {
  const { read } = runDisputed();
  const report = read(REPORT);
  for (const [heading, count] of [
    ["Opposite conclusions", 2],
    ["A judge's ranges are not the candidate's", 2],
    ["Contradicts a verdict the register already holds", 1],
    ["One judge committed, the other would not", 2],
  ]) {
    const section = report.slice(report.indexOf(heading));
    assert.match(
      section.slice(0, 400),
      new RegExp(`\\*\\*${count}\\.?\\*\\*`),
      `${heading} must report ${count}`,
    );
  }
  assert.ok(!report.includes("What was merged"), "a merge summary would bury the disagreements");
  assert.ok(!report.includes("states too little"), "thin prose is not a disagreement between judges");
});

test("judges who agree completely get merged, in their own words", () => {
  const { status, json, read } = runAgreed();
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
  assert.match(read(REPORT), /\*\*0 pairs/, "the report says plainly that nothing disagreed");
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
