// Accuracy-gate wiring contract. [CORPUS-SCORE]
//
// The clone registers are the only independent evidence that this detector is
// accurate, and they are worth nothing if nothing enforces them. These tests
// assert the wiring that keeps the gate live: that CI runs it, that a failing
// gate blocks the merge, that the scorecard is published whether the gate
// passed or failed, and that the local target and the CI job call one script so
// a green local run cannot mean a red one in CI.
//
// They also pin the two properties that make the gate honest: every judged
// pair is held to its own outcome in one recorded JSON file, which CI refuses to
// let a run change uncommitted, and the CI slice takes its commits from the
// registers rather than repeating a pin that could drift away from the source a
// judge actually read. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { execFileSync } from "node:child_process";

import { repoRoot } from "../lib/repo-root.mjs";
import { recipeBlocks } from "../lib/makefile.mjs";

const read = (path) => readFileSync(resolve(repoRoot, path), "utf8");
const ciWorkflow = read(".github/workflows/ci.yml");
const gateScript = read("scripts/corpus/score-gate.sh");
const targetsScript = read("scripts/corpus/target-repos.sh");
const compareScript = read("scripts/compare-versions.sh");
const makefile = read("Makefile");

const SCRIPT = "scripts/corpus/score-gate.sh";
const TARGETS = "scripts/corpus/target-repos.sh";
const COMPARE = "scripts/compare-versions.sh";
const JOB = "corpus-score";
/// [CORPUS-SCORE-OUTCOMES] The judged pair record, and the per-repository
/// count gate it replaced, which must never come back beside it.
const OUTCOMES = "corpus/register/outcomes.json";
const RETIRED_THRESHOLDS = "corpus/register/score-thresholds.json";
const COUNT_ALLOWANCES = ["score-thresholds", "maximum_false", "minimum_score_percent"];
/// The CI step that refuses a record the run changed but nobody committed.
const RECORD_CHECK = `git status --porcelain -- ${OUTCOMES}`;
const REGISTER_DIR = "corpus/register";
const POLLY_REGISTER = `${REGISTER_DIR}/polly.json`;
const FSHARP_REGISTER = `${REGISTER_DIR}/fsharp.data.json`;
/// Repositories queued for a first judging pass: scanned by the wide
/// comparison so they can be judged, never by the gate, which can only score
/// what a judge has already ruled on.
const JUDGING_QUEUE = "corpus/judging-queue.json";
/// [CORPUS-PIN] A pin is a full git object name, and nothing weaker.
const COMMIT_ID_LENGTH = 40;
const COMMIT_ID_ALPHABET = "0123456789abcdef";
const isCommitId = (value) =>
  typeof value === "string" &&
  value.length === COMMIT_ID_LENGTH &&
  [...value].every((character) => COMMIT_ID_ALPHABET.includes(character));
/// The aggregate job the branch ruleset requires; a gate missing from its
/// `needs` reports red and merges anyway.
const AGGREGATE_NEEDS = /^\s*needs: \[(.+)\]\s*$/m;

test("[CORPUS-SCORE] CI runs the accuracy gate", () => {
  assert.ok(
    ciWorkflow.includes(`${JOB}:`),
    `ci.yml no longer declares the \`${JOB}\` job; nothing would enforce the clone registers`,
  );
  assert.ok(
    ciWorkflow.includes(`run: ${SCRIPT}`),
    `ci.yml must run ${SCRIPT} — the same script \`make score-gate\` runs`,
  );
});

test("[CORPUS-SCORE] a failing accuracy gate blocks the merge", () => {
  const aggregate = ciWorkflow.slice(ciWorkflow.lastIndexOf("\n  ci:\n"));
  const needs = AGGREGATE_NEEDS.exec(aggregate);
  assert.ok(needs, "the aggregate `ci` job declares no `needs` list");
  const required = needs[1].split(",").map((name) => name.trim());
  assert.ok(
    required.includes(JOB),
    `the aggregate \`ci\` job must require \`${JOB}\`; without it a false positive or false ` +
      `negative reports red and merges anyway. Required: ${required.join(", ")}`,
  );
});

test("[CORPUS-SCORE] the scorecard is published even when the gate fails", () => {
  const job = ciWorkflow.slice(ciWorkflow.indexOf(`  ${JOB}:`));
  const upload = job.indexOf("name: corpus-scorecard");
  assert.ok(upload > 0, "the scorecard is never uploaded, so a failing gate leaves nothing to read");
  const step = job.slice(job.lastIndexOf("- name:", upload), upload);
  assert.ok(
    step.includes("if: always()"),
    "the scorecard upload must be `if: always()` — a failing run is exactly when someone needs it",
  );
});

test("[CORPUS-SCORE] the local target and the CI job run one script", () => {
  const blocks = recipeBlocks("score-gate");
  assert.equal(blocks.length, 1, `Makefile must declare exactly one \`score-gate\` recipe; found ${blocks.length}`);
  assert.ok(
    blocks[0].body.includes(SCRIPT),
    `make score-gate must invoke ${SCRIPT}, so local and CI can never drift apart`,
  );
});

test("[CORPUS-SCORE-OUTCOMES] every pair is held to one record, which CI refuses to see changed", () => {
  assert.ok(
    !existsSync(resolve(repoRoot, RETIRED_THRESHOLDS)),
    `${RETIRED_THRESHOLDS} is back; a count allowance tolerates any defect up to its number, so one pair fixed and another broken leave it unmoved`,
  );
  for (const [name, source] of [["ci.yml", ciWorkflow], ["Makefile", makefile], [SCRIPT, gateScript], [COMPARE, compareScript]]) {
    for (const allowance of COUNT_ALLOWANCES) {
      assert.ok(!source.includes(allowance), `${name} mentions \`${allowance}\`; ${OUTCOMES} is the only gate`);
    }
  }
  const job = ciWorkflow.slice(ciWorkflow.indexOf(`  ${JOB}:`));
  const gate = job.indexOf(`run: ${SCRIPT}`);
  const check = job.indexOf(RECORD_CHECK);
  assert.ok(
    check > gate && gate > 0,
    `the \`${JOB}\` job must run \`${RECORD_CHECK}\` after ${SCRIPT}: a fix left off the record keeps the pair on record as failing, and its next regression passes as the old tolerated defect`,
  );
  assert.ok(
    job.slice(check, job.indexOf("- name:", check)).includes("exit 1"),
    "a record the run changed must fail the job, not just print a diff",
  );
});

/// The repositories the CI slice scans, as named in score-gate.sh.
const ciSlice = () => {
  const declared = /^DEFAULT_SLICE=\((.+)\)$/m.exec(gateScript);
  assert.ok(declared, "score-gate.sh declares no DEFAULT_SLICE");
  return declared[1].split(/\s+/).filter(Boolean);
};

const assertDefaultTargetMatchesRegister = (registerPath, repository) => {
  const targets = execFileSync("bash", [SCRIPT, "--list-default-targets"], {
    cwd: repoRoot,
    encoding: "utf8",
  }).trim().split("\n").map((line) => line.split("#"));
  const judged = JSON.parse(read(registerPath));
  assert.ok(
    targets.some(([url, sha, language]) =>
      url === judged.url && sha === judged.sha && language === judged.language),
    `the default gate must scan ${repository} at the exact commit and language its register judged`,
  );
};

test("[CORPUS-SCORE-GATE-SLICE] the default gate scans pinned C# pairs in Polly", () => {
  assertDefaultTargetMatchesRegister(POLLY_REGISTER, "Polly");
});

test("[CORPUS-SCORE-GATE-SLICE] the default gate scans pinned F# pairs in FSharp.Data", () => {
  assertDefaultTargetMatchesRegister(FSHARP_REGISTER, "FSharp.Data");
});

test("[CORPUS-SCORE] the CI slice is pinned by its registers, not by a repeated commit", () => {
  assert.ok(
    gateScript.includes(`source "$REPO_ROOT/${TARGETS}"`) &&
      gateScript.includes('register_targets "$REPO_ROOT" "${DEFAULT_SLICE[@]}"'),
    "the slice must read each target's url and commit from its register — a pin repeated in " +
      "the script can drift away from the commit the judge actually read",
  );
  assert.ok(
    targetsScript.includes(`REGISTER_DIR="${REGISTER_DIR}"`) &&
      targetsScript.includes("$root/$REGISTER_DIR/$name.json"),
    `${TARGETS} must resolve a slice name to its register under ${REGISTER_DIR}`,
  );
  const slice = ciSlice();
  assert.ok(slice.length >= 2, `the CI slice must cover more than one repository; got ${slice.join(", ")}`);
  for (const name of slice) {
    const register = resolve(repoRoot, REGISTER_DIR, `${name}.json`);
    assert.ok(existsSync(register), `the CI slice names \`${name}\`, which has no register at ${register}`);
    const judged = JSON.parse(readFileSync(register, "utf8"));
    assert.ok(
      judged.clearly_in.length + (judged.clearly_out?.length ?? 0) > 0,
      `\`${name}\` is in the CI slice but judges no pairs, so gating on it asserts nothing`,
    );
  }
});

test("[CORPUS-PIN] every scanned repository is pinned to a commit id, never a version", () => {
  assert.ok(
    targetsScript.includes('["url", "sha", "language"]') && !targetsScript.includes('"tag"'),
    `${TARGETS} must read the COMMIT from each register; a tag is a name upstream can re-point`,
  );
  for (const name of ciSlice()) {
    const judged = JSON.parse(read(`${REGISTER_DIR}/${name}.json`));
    assert.ok(
      isCommitId(judged.sha),
      `${name} is pinned by \`${judged.sha}\`, not a ${COMMIT_ID_LENGTH}-character commit id`,
    );
    assert.equal(
      judged.tag,
      undefined,
      `${name} still carries a \`tag\`; two pins mean the weaker one eventually gets used`,
    );
  }
});

test("[CORPUS-REGISTER-COVERAGE] every register in the CI slice judges a real body of pairs", () => {
  /// A register that judges a handful of pairs cannot distinguish an engine
  /// that improved from one that got lucky. See docs/specs/corpus.md.
  const MINIMUM_JUDGED = 15;
  for (const name of ciSlice()) {
    const judged = JSON.parse(read(`${REGISTER_DIR}/${name}.json`));
    const total = judged.clearly_in.length + (judged.clearly_out?.length ?? 0);
    assert.ok(
      total >= MINIMUM_JUDGED,
      `${name} judges ${total} pair(s); the floor is ${MINIMUM_JUDGED}. A register this thin ` +
        `scores an engine on luck, and one wrong verdict moves the percentage double digits`,
    );
  }
});

test("[CORPUS-SCORE] one command runs the CI corpus, and one runs the comparison", () => {
  for (const [target, script] of [
    ["score-gate", SCRIPT],
    ["compare", COMPARE],
  ]) {
    const blocks = recipeBlocks(target);
    assert.equal(blocks.length, 1, `Makefile must declare exactly one \`${target}\` recipe`);
    assert.ok(
      blocks[0].body.includes(script),
      `make ${target} must invoke ${script}, so local and CI can never drift apart`,
    );
  }
});

test("[CORPUS-SCORE] the comparison defaults to the last release against HEAD, rebuilding both", () => {
  assert.ok(
    compareScript.includes("last_release_commit") && compareScript.includes('resolve_commit HEAD'),
    `${COMPARE} must default to the last release against HEAD; naming commits by hand is the ` +
      "exception, not the price of running it",
  );
  const cycle = compareScript.slice(compareScript.indexOf("run_cycle() {"));
  assert.ok(
    cycle.includes('rm -rf "$TARGET_DIR"') && cycle.includes("extract_source") && cycle.includes("compile"),
    "every cycle must wipe the build artifacts and rebuild from a fresh extract; a reused " +
      "target directory can leave one side scanning the other side's engine",
  );
});

test("[CORPUS-REGISTER-QUEUE] the comparison scans the queue and the gate does not", () => {
  assert.ok(
    targetsScript.includes(JUDGING_QUEUE),
    `${TARGETS} must read ${JUDGING_QUEUE}; without it a repository with no register is never ` +
      "scanned, has no reports, can never be judged, and the corpus cannot grow a new language",
  );
  assert.ok(
    compareScript.includes("default_targets"),
    `${COMPARE} must scan the registers and the queue together — that is what produces the two ` +
      "reports a first judging pass needs",
  );
  assert.ok(
    !gateScript.includes("default_targets") && !gateScript.includes(JUDGING_QUEUE),
    `${SCRIPT} must not scan ${JUDGING_QUEUE}: an unjudged repository answers no questions, so ` +
      "every push would pay for a scan that can never move the score",
  );
  const queue = JSON.parse(read(JUDGING_QUEUE));
  assert.ok(queue.repositories.length > 0, "an empty queue means no new language is coming");
  for (const repository of queue.repositories) {
    assert.ok(
      isCommitId(repository.sha),
      `${JUDGING_QUEUE} pins ${repository.name} by '${repository.sha}', which is not a commit id`,
    );
    assert.ok(
      !ciSlice().includes(repository.name),
      `${repository.name} is queued for judging and cannot be in the CI slice`,
    );
  }
});

/// A repository judged once, with no agreed CLEARLY IN: it holds a register and
/// is still queued for another pass. And one that is only queued.
const REJUDGED = { name: "Rejudged", url: "https://example.invalid/rejudged.git", sha: "a".repeat(COMMIT_ID_LENGTH), language: "fsharp" };
const UNJUDGED = { name: "unjudged", url: "https://example.invalid/unjudged.git", sha: "b".repeat(COMMIT_ID_LENGTH), language: "go" };
const spec = (repository) => `${repository.url}#${repository.sha}#${repository.language}`;

/// [CORPUS-PIN] The settings files beside the registers, which a target list
/// must never read as a repository to scan.
const NOT_A_REPOSITORY = "corpus/not-a-repository.json";
const RECORD_FILE = "outcomes.json";

test("[CORPUS-REGISTER-QUEUE] a queued repository that already has a register is scanned once", () => {
  const root = mkdtempSync(join(tmpdir(), "default-targets-"));
  mkdirSync(join(root, REGISTER_DIR), { recursive: true });
  writeFileSync(join(root, REGISTER_DIR, `${REJUDGED.name.toLowerCase()}.json`), JSON.stringify(REJUDGED));
  writeFileSync(join(root, REGISTER_DIR, RECORD_FILE), JSON.stringify({ spec: "", pairs: [] }));
  writeFileSync(join(root, NOT_A_REPOSITORY), JSON.stringify({ files: [RECORD_FILE] }));
  writeFileSync(join(root, JUDGING_QUEUE), JSON.stringify({ repositories: [REJUDGED, UNJUDGED] }));
  const listed = execFileSync(
    "bash",
    ["-c", `WORK_DIR="$1"; source "${TARGETS}"; default_targets "$1"`, "default-targets", root],
    { cwd: repoRoot, encoding: "utf8" },
  ).split("\n").filter(Boolean);
  assert.deepEqual(
    listed,
    [spec(REJUDGED), spec(UNJUDGED)],
    "a register still waiting on another pass is one target, not two scans of one tree, and the " +
      "judged pair record beside the registers is no target at all",
  );
});
