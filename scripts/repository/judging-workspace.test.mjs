// The blinded judging folder. [CORPUS-REGISTER-WORKSPACE]
//
// The register is the only independent evidence that this detector is accurate,
// and it is only independent while the judge cannot work out what produced the
// reports. That property is not a matter of discipline — it is whatever this
// script actually writes to disk. So these tests build a real folder from a
// throwaway source tree and two throwaway reports, then assert what a judge
// would find in it: the repositories, the reports, and the judging skill
// installed at the root so an agent can run the protocol by name.
//
// They also assert the two things that would silently void a pass — the A/B key
// landing inside the folder, and this project's name appearing anywhere in it.
// Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  realpathSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { repoRoot } from "../lib/repo-root.mjs";

const read = (path) => readFileSync(resolve(repoRoot, path), "utf8");

const BUILDER = "scripts/corpus/register-workspace.mjs";
const PREPARER = "scripts/corpus/prepare-judging.sh";
const JUDGE_SKILL = ".agents/skills/judge-clone-pairs/SKILL.md";
/// The protocol lives under `.agents/` and `.claude/` links to it, the way this
/// repository lays out its own skills — one file, two names, no drift.
const SKILL_NAME = "judge-clone-pairs";
const PROTOCOL_IN_ROOT = `.agents/skills/${SKILL_NAME}/SKILL.md`;
const PROTOCOL_LINKED = `.claude/skills/${SKILL_NAME}/SKILL.md`;
/// Every word a judge reads that is not source, a report or a candidate is a
/// file in this repository, copied across unchanged. Prose composed at build
/// time is prose nobody reviewed, and two runs would hand two different folders
/// to two judges.
const HANDOVER_DIR = ".agents/skills/judge-clone-pairs/handover";
const ROOT_GUIDE = "AGENTS.md";
const ROOT_IMPORT = "CLAUDE.md";
const WORKSPACE_GUIDE = "README.md";
const LINKED_PROTOCOL = "JUDGING.md";
const KEY_SUFFIX = ".key.json";
/// Naming this project anywhere in the folder tells the judge what produced the
/// reports, and every verdict from that pass is void.
const FORBIDDEN_WORD = "deslop";
const REPO = "widget";
const SOURCE_FILE = "widget.py";
const PINNED_URL = "https://example.invalid/widget.git";
const PINNED_SHA = "0123456789abcdef0123456789abcdef01234567";
const SEED = "7";
/// Two regions per candidate, and enough distinct clusters that a draw stratified
/// across provenance and size has something to draw from.
const REGIONS_PER_CANDIDATE = 2;
const CLUSTER_COUNT = 6;
const SOURCE_LINES = 200;

const preparerScript = read(PREPARER);
const makefile = read("Makefile");

/// A source tree, two pair lists over it, and somewhere to put the result — the
/// smallest input that still exercises stratification, the coin flip and the
/// leak scan.
const fixture = () => {
  const base = mkdtempSync(join(realpathSync(tmpdir()), "judging-"));
  const source = join(base, "source");
  mkdirSync(source, { recursive: true });
  writeFileSync(
    join(source, SOURCE_FILE),
    Array.from({ length: SOURCE_LINES }, (_, line) => `value_${line} = ${line}`).join("\n"),
  );
  const report = (offset, span) => ({
    clusters: Array.from({ length: CLUSTER_COUNT }, (_, index) => ({
      id: `cluster-${offset}-${index}`,
      rank: index,
      occurrences: Array.from({ length: REGIONS_PER_CANDIDATE }, (_, side) => ({
        path: SOURCE_FILE,
        start_line: offset + index * span * REGIONS_PER_CANDIDATE + side * span + 1,
        end_line: offset + index * span * REGIONS_PER_CANDIDATE + side * span + span,
      })),
    })),
  });
  const one = join(base, "one.json");
  const two = join(base, "two.json");
  writeFileSync(one, JSON.stringify(report(0, 4)));
  writeFileSync(two, JSON.stringify(report(0, 8)));
  return { base, source, one, two };
};

const build = ({ base, source, one, two }, keys) =>
  execFileSync(
    "node",
    [
      resolve(repoRoot, BUILDER),
      "--workspace", join(base, "folder", REPO),
      "--source", source,
      "--report-one", one,
      "--report-two", two,
      "--url", PINNED_URL,
      "--sha", PINNED_SHA,
      "--seed", SEED,
      "--keys", keys ?? join(base, "keys"),
    ],
    { encoding: "utf8" },
  );

const everyFile = (directory) =>
  readdirSync(directory, { withFileTypes: true, recursive: true })
    .filter((entry) => entry.isFile())
    .map((entry) => join(entry.parentPath ?? entry.path, entry.name));

test("[CORPUS-REGISTER-WORKSPACE] the folder holds the repository, the reports and the skill", () => {
  const files = fixture();
  build(files);
  const root = join(files.base, "folder");
  const workspace = join(root, REPO);

  assert.equal(
    readFileSync(join(root, PROTOCOL_IN_ROOT), "utf8"),
    read(JUDGE_SKILL),
    `${PROTOCOL_IN_ROOT} must be the judging protocol itself, or the judge has no protocol to run`,
  );
  assert.equal(
    readFileSync(join(root, PROTOCOL_LINKED), "utf8"),
    read(JUDGE_SKILL),
    `${PROTOCOL_LINKED} must reach the same file — that is the path a skill is loaded from`,
  );
  assert.ok(
    lstatSync(join(root, ".claude/skills", SKILL_NAME)).isSymbolicLink(),
    "the second name must be a link, not a second copy; two copies drift and one goes stale",
  );
  assert.equal(
    readFileSync(join(workspace, LINKED_PROTOCOL), "utf8"),
    read(JUDGE_SKILL),
    `${LINKED_PROTOCOL} must resolve to the installed protocol from inside a repository directory`,
  );
  assert.ok(
    read(JUDGE_SKILL).includes("name: judge-clone-pairs"),
    "the installed file must be a skill an agent can run by name, not loose prose",
  );
  assert.ok(
    readFileSync(join(root, ROOT_IMPORT), "utf8").includes(`@${ROOT_GUIDE}`),
    `${ROOT_IMPORT} must pull in ${ROOT_GUIDE}, so the blind is stated before the first candidate`,
  );

  assert.equal(
    readFileSync(join(workspace, "source", SOURCE_FILE), "utf8").split("\n").length,
    SOURCE_LINES,
    "the repository source travels into the workspace whole",
  );
  assert.equal(
    readFileSync(join(workspace, "PINNED.txt"), "utf8"),
    `${PINNED_URL}\n${PINNED_SHA}\n`,
    "the workspace cites the exact commit the register will be pinned to",
  );
  assert.deepEqual(
    JSON.parse(readFileSync(join(workspace, "verdicts.json"), "utf8")),
    { clearly_in: [], clearly_out: [], not_clear: [] },
    "the judge is handed an empty verdict file, never a partly filled one",
  );
});

test("[CORPUS-REGISTER-WORKSPACE] the reports carry regions and no engine reasoning", () => {
  const files = fixture();
  build(files);
  const workspace = join(files.base, "folder", REPO);
  for (const letter of ["a", "b"]) {
    const { groups } = JSON.parse(readFileSync(join(workspace, `report-${letter}.json`), "utf8"));
    assert.equal(groups.length, CLUSTER_COUNT, `report-${letter} must list every pair`);
    for (const group of groups) {
      assert.equal(group.regions.length, REGIONS_PER_CANDIDATE);
      assert.deepEqual(
        Object.keys(group.regions[0]).sort(),
        ["end_line", "path", "start_line"],
        "a region names where it is and nothing about how confident the engine was",
      );
      assert.ok(!("rank" in group), "rank is the engine's opinion of itself and must be stripped");
    }
  }
  const candidates = readdirSync(join(workspace, "candidates"));
  assert.ok(candidates.includes("index.md"), "the judge needs a checklist to work through");
  assert.ok(candidates.includes("pairs.json"), "the merge step reads the ranges back from pairs.json");
  const { pairs } = JSON.parse(readFileSync(join(workspace, "candidates", "pairs.json"), "utf8"));
  assert.ok(pairs.length > 0, "a folder with no candidates asks the judge nothing");
  for (const pair of pairs) {
    assert.equal(pair.occurrences.length, REGIONS_PER_CANDIDATE, "a candidate is two regions");
  }
});

test("[CORPUS-REGISTER-WORKSPACE] the A/B key stays outside the folder the judge is handed", () => {
  const files = fixture();
  build(files);
  const key = JSON.parse(readFileSync(join(files.base, "keys", `${REPO}${KEY_SUFFIX}`), "utf8"));
  assert.equal(Number(key.seed), Number(SEED), "the key records the seed the draw used");
  assert.deepEqual(
    [key["report-a"], key["report-b"]].sort(),
    [files.one, files.two].sort(),
    "the key is what lets scoring undo the coin flip; without it the pass cannot be read back",
  );
  assert.ok(
    !everyFile(join(files.base, "folder")).some((path) => path.endsWith(KEY_SUFFIX)),
    "a key inside the folder is the answer sheet — the judge would be reading the comparison",
  );
});

test("[CORPUS-REGISTER-WORKSPACE] a key aimed inside the folder is refused", () => {
  const files = fixture();
  assert.throws(
    () => build(files, join(files.base, "folder", "keys")),
    /inside the folder handed to the judge/,
    "writing the key into the folder must fail loudly, not produce a contaminated pass",
  );
});

test("[CORPUS-REGISTER-WORKSPACE] nothing in the folder names this project", () => {
  const files = fixture();
  build(files);
  const named = everyFile(join(files.base, "folder")).filter((path) =>
    readFileSync(path, "utf8").toLowerCase().includes(FORBIDDEN_WORD),
  );
  assert.deepEqual(named, [], `these files tell the judge what produced the reports`);
});

test("[CORPUS-REGISTER-WORKSPACE] the folder is built by one command, outside this repository", () => {
  assert.ok(
    makefile.includes("judging-folder:") && makefile.includes(PREPARER),
    `\`make judging-folder\` must run ${PREPARER}; a folder built by hand is built differently each time`,
  );
  assert.ok(
    preparerScript.includes('WORK_DIR="$(dirname "$COMPARE_REPORTS")"') &&
      preparerScript.includes('--keys "$root.keys"'),
    "the keys belong beside the folder, and the checkouts must be the comparison's own — a " +
      "second clone of every repository is both waste and a tree nothing was ever scanned at",
  );
  assert.ok(
    preparerScript.includes("is inside this repository"),
    "a folder inside this checkout lets a judge walk up into the source that produced the reports",
  );
  assert.ok(
    preparerScript.includes("meta.json"),
    "the two reports must come from one comparison manifest, or a workspace can mix two runs",
  );
});

test("[CORPUS-REGISTER-WORKSPACE] every word the judge reads is copied, never composed", () => {
  const files = fixture();
  build(files);
  const root = join(files.base, "folder");
  const copied = [
    [join(root, ROOT_GUIDE), `${HANDOVER_DIR}/root/${ROOT_GUIDE}`],
    [join(root, ROOT_IMPORT), `${HANDOVER_DIR}/root/${ROOT_IMPORT}`],
    [join(root, REPO, WORKSPACE_GUIDE), `${HANDOVER_DIR}/workspace/${WORKSPACE_GUIDE}`],
    [join(root, PROTOCOL_IN_ROOT), JUDGE_SKILL],
  ];
  for (const [written, source] of copied) {
    assert.equal(
      readFileSync(written, "utf8"),
      read(source),
      `${written} must be ${source} byte for byte. Prose built at run time is prose nobody ` +
        "reviewed, and two passes would hand two different folders to two judges",
    );
  }
});
