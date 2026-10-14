// [CORPUS-REGISTER-MERGE] The throwaway judging folders and repository root the
// merge tests drive `scripts/corpus/merge-verdicts.mjs` against, shared by every
// suite that asserts what reaches a register.

import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
export const SCRIPT = join(REPO_ROOT, "scripts", "corpus", "merge-verdicts.mjs");
export const REPORT = "docs/reports/verdict-merge.md";
export const REGISTER_DIR = "corpus/register";
export const QUEUE = "corpus/judging-queue.json";

/// The fixture repository, and the two judges sent to read it.
export const REPO = "widgets";
export const REGISTER = "widgets.json";
export const LANGUAGE = "rust";
export const URL = "https://example.invalid/widgets.git";
export const SHA = "0123456789abcdef0123456789abcdef01234567";
export const FIRST_JUDGE = "judge-one";
export const SECOND_JUDGE = "judge-two";
export const THIRD_JUDGE = "judge-three";

/// A repository that already has a register, so a standing verdict can be
/// contradicted by a later pass.
export const JUDGED = "gadgets";
export const JUDGED_REGISTER = "gadgets.json";

/// Prose long enough to clear the assertion floor a scored entry is held to.
export const WHY = "The same twenty lines appear twice with only the type name changed.";
export const VERIFIED = "diff of the two ranges: identical but for the type name on line 3.";
/// A NOT CLEAR note, deliberately shorter than that floor. NOT CLEAR asserts
/// nothing, so it must still be recorded — otherwise the next pass re-reads a
/// pair somebody has already ruled on.
export const TERSE_NOTE = "2-line idiom.";
/// The note both judges write about the pair the standing register already
/// calls CLEARLY IN, so the merge has to choose between them.
export const STANDING_NOTE = "Copied text is real but the boundaries are ragged.";

/// The candidates every judge is shown. Same list for both, or their candidate
/// numbers would name different pairs.
export const PAIRS = {
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
export const STANDING = ["src/k.rs:1-9", "src/l.rs:1-9"];

export const at = (pair) => PAIRS.pairs.find((entry) => entry.number === pair).occurrences;
export const scored = (candidate) => ({ candidate, why: WHY, verified: VERIFIED, occurrences: at(candidate) });
export const noted = (candidate, why = TERSE_NOTE) => ({ candidate, why, occurrences: at(candidate) });

export const write = (path, value) => {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, typeof value === "string" ? value : `${JSON.stringify(value, null, 2)}\n`);
};

/// A judging folder holding one workspace per repository, as the preparer
/// builds it.
export const judgingFolder = (root, judge, repos) => {
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
export const fixtureRoot = (root) => {
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
export const run = (root, folders, extra = []) => {
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

export const EMPTY_VERDICTS = { clearly_in: [], clearly_out: [], not_clear: [] };

export const runPasses = (verdicts) => {
  const root = mkdtempSync(join(tmpdir(), "verdict-merge-independent-"));
  fixtureRoot(root);
  const folders = Object.entries(verdicts).map(([judge, pass]) =>
    judgingFolder(root, judge, { [REPO]: pass }),
  );
  return run(root, folders);
};
