// [CORPUS-PIN] Drives `scripts/corpus/fetch-corpus.mjs` against a throwaway
// corpus tree and asserts what it clones, what it refuses, and where it learns
// the difference.
//
// Black box on purpose: the fetch is run as a process against a corpus folder
// built for the test, and every assertion reads a clone it made or an error it
// printed. `corpus/` holds two kinds of file — repositories to clone, and
// settings that describe no repository at all — and the fetch has to tell them
// apart. It got that wrong (gh #530): the rule was written down three times in
// three languages, one copy went stale when a settings file was added, and the
// fetch died on it before a single accuracy check ran.
//
// So the third test pins the shape of the fix as hard as the first two pin the
// behaviour: one list, read by everything that has to know. Run with
// `node --test`.

import { strict as assert } from "node:assert";
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import test from "node:test";

import { recipeBlocks } from "../lib/makefile.mjs";
import { repoRoot } from "../lib/repo-root.mjs";
import { writeFileAt } from "../lib/write-file.mjs";

/// The script under test, the harness that has to agree with it, and the make
/// target that runs this gate.
const FETCH = "scripts/corpus/fetch-corpus.mjs";
const HARNESS = "crates/deslop-test-support/src/corpus.rs";
const GATE = "scripts/repository/corpus-fetch.test.mjs";
const GATE_TARGET = "_ci-contract-tests";

/// Where manifests are read from, where clones land, and where the registers
/// the same rule applies to live.
const CORPUS_DIR = "corpus";
const REGISTER_DIR = "corpus/register";
const CACHE_DIR = ".corpus";
const JSON_SUFFIX = ".json";

/// [CORPUS-PIN] The one list naming the corpus files that describe no
/// repository. It names itself, because it is one of them.
const NOT_A_REPOSITORY = `not-a-repository${JSON_SUFFIX}`;
const QUEUE_FILE = `judging-queue${JSON_SUFFIX}`;
const KNOWN_FAILURES_FILE = `known-failures${JSON_SUFFIX}`;

/// Everything a manifest that names a repository must carry. A settings file
/// carries none of them; a manifest missing one is an error, never a skip.
const MANIFEST_FIELDS = ["name", "url", "sha"];
const missingField = (field) => `corpus manifest is missing "${field}"`;

/// How many of the listed files one reader may name. Naming a settings file is
/// reading it — the harness loads the known-failure baseline. Naming several is
/// deciding which corpus files are settings, and that decision lives in one
/// place or it goes stale in the copies.
const MAXIMUM_LISTED_NAMES = 1;

/// How much of a commit id names a clone directory.
const SHORT_SHA_LENGTH = 12;
/// A pin nothing in the fixture ever fetches, only reads back.
const UNFETCHED_SHA = "0123456789abcdef0123456789abcdef01234567";

/// The fixture repository the fetch is asked to clone, and the commit it is
/// asked to land on.
const REPO = "widgets";
const SOURCE_FILE = "src/widgets.txt";
const SOURCE_TEXT = "the same twenty lines, twice over\n";
const COMMIT_MESSAGE = "widgets at the commit the manifest pins";
const GIT_IDENTITY = ["-c", "user.name=corpus fixture", "-c", "user.email=corpus@example.invalid"];

/// The settings files that sit beside the manifests. Neither names a
/// repository, and the fetch must clone neither.
const QUEUE = {
  why: "Repositories waiting on a first judging pass.",
  repositories: [
    {
      name: "serilog",
      url: "https://example.invalid/serilog.git",
      sha: UNFETCHED_SHA,
      language: "csharp",
      why: "One register is the thinnest ground truth a language can have.",
    },
  ],
};
const KNOWN_FAILURES = { why: "Checks already known to fail.", repos: {} };
const NOT_A_REPOSITORY_LIST = {
  why: "Files under corpus/ that hold settings rather than a repository to clone.",
  files: [KNOWN_FAILURES_FILE, NOT_A_REPOSITORY, QUEUE_FILE],
};

const read = (path) => readFileSync(resolve(repoRoot, path), "utf8");
const jsonFiles = (directory) =>
  readdirSync(resolve(repoRoot, directory)).filter((name) => name.endsWith(JSON_SUFFIX));
const git = (cwd, args) => execFileSync("git", args, { cwd, encoding: "utf8" }).trim();

/// A temp directory for one test, removed win or lose.
function workdir(t) {
  const work = mkdtempSync(join(tmpdir(), "deslop-corpus-fetch-"));
  t.after(() => rmSync(work, { recursive: true, force: true }));
  return work;
}

/// A real git repository with one commit, so the fetch has something to clone
/// without reaching the network.
function originRepo(work) {
  const origin = join(work, "origin");
  writeFileAt(join(origin, SOURCE_FILE), SOURCE_TEXT);
  git(origin, ["-c", "init.defaultBranch=main", "init", "--quiet"]);
  git(origin, ["add", "."]);
  git(origin, [...GIT_IDENTITY, "commit", "--quiet", "-m", COMMIT_MESSAGE]);
  return { url: origin, sha: git(origin, ["rev-parse", "HEAD"]) };
}

/// A corpus folder holding one manifest and every kind of file that is not one.
function corpusTree(work, manifest) {
  const write = (name, value) =>
    writeFileAt(join(work, CORPUS_DIR, name), `${JSON.stringify(value, null, 2)}\n`);
  write(`${REPO}${JSON_SUFFIX}`, manifest);
  write(QUEUE_FILE, QUEUE);
  write(KNOWN_FAILURES_FILE, KNOWN_FAILURES);
  write(NOT_A_REPOSITORY, NOT_A_REPOSITORY_LIST);
  return work;
}

/// Runs the fetch over a prepared folder, capturing the exit status rather than
/// throwing on it: a run that meets a broken manifest is meant to fail.
const runFetch = (work, args = []) =>
  spawnSync("node", [resolve(repoRoot, FETCH), ...args], { cwd: work, encoding: "utf8" });

/// What landed in the clone cache, whether or not the fetch got as far as
/// creating it.
const cloned = (work) =>
  existsSync(join(work, CACHE_DIR)) ? readdirSync(join(work, CACHE_DIR)).sort() : [];

test("[CORPUS-PIN] the fetch skips every corpus file that is not a repository", (t) => {
  const work = workdir(t);
  const { url, sha } = originRepo(work);
  corpusTree(work, { name: REPO, url, sha });

  const result = runFetch(work);
  assert.equal(
    result.status,
    0,
    `the fetch refused a corpus folder holding ${QUEUE_FILE}, so no accuracy check ran:\n` +
      `${result.stdout}${result.stderr}`,
  );
  const clone = `${REPO}-${sha.slice(0, SHORT_SHA_LENGTH)}`;
  assert.deepEqual(
    cloned(work),
    [clone],
    `the fetch must clone the one manifest that names a repository and nothing else; ` +
      `${QUEUE_FILE}, ${KNOWN_FAILURES_FILE} and ${NOT_A_REPOSITORY} are settings`,
  );
  assert.equal(
    git(join(work, CACHE_DIR, clone), ["rev-parse", "HEAD"]),
    sha,
    `${REPO} landed on a commit the manifest does not pin`,
  );
  assert.equal(
    readFileSync(join(work, CACHE_DIR, clone, SOURCE_FILE), "utf8"),
    SOURCE_TEXT,
    `${REPO} was cloned without the source the corpus suite scans`,
  );
  assert.ok(
    result.stdout.includes(REPO),
    `the fetch reported nothing about ${REPO}; it must say what it cloned`,
  );
});

test("[CORPUS-PIN] a manifest that names a repository is held to every field", (t) => {
  const work = workdir(t);
  const { url, sha } = originRepo(work);
  for (const omitted of MANIFEST_FIELDS) {
    const manifest = { name: REPO, url, sha };
    delete manifest[omitted];
    corpusTree(work, manifest);

    const result = runFetch(work);
    assert.notEqual(
      result.status,
      0,
      `a manifest with no "${omitted}" was accepted; skipping what will not parse is not the ` +
        `fix, or a mistyped pin becomes a silently unscanned repository`,
    );
    assert.ok(
      result.stderr.includes(missingField(omitted)),
      `the failure must name the missing "${omitted}" field, got:\n${result.stderr}`,
    );
    assert.deepEqual(cloned(work), [], `a broken manifest still produced a clone`);
  }
});

test("[CORPUS-PIN] one list names every corpus file that is not a repository", () => {
  const listPath = resolve(repoRoot, CORPUS_DIR, NOT_A_REPOSITORY);
  assert.ok(
    existsSync(listPath),
    `${CORPUS_DIR}/${NOT_A_REPOSITORY} is missing: the rule saying which corpus files hold ` +
      `settings has no single home, so every reader keeps its own copy and one of them goes stale`,
  );
  const listed = JSON.parse(readFileSync(listPath, "utf8")).files;
  assert.ok(
    listed.includes(NOT_A_REPOSITORY),
    `${NOT_A_REPOSITORY} must name itself; it is a settings file, and a reader that treats it ` +
      `as a manifest dies on the very file that exists to stop that`,
  );
  for (const directory of [CORPUS_DIR, REGISTER_DIR]) {
    for (const name of jsonFiles(directory).filter((name) => !listed.includes(name))) {
      const manifest = JSON.parse(read(join(directory, name)));
      for (const field of MANIFEST_FIELDS) {
        assert.ok(
          manifest[field],
          `${directory}/${name} carries no "${field}", so it names no repository — add it to ` +
            `${CORPUS_DIR}/${NOT_A_REPOSITORY} or give it a pin`,
        );
      }
    }
  }
  for (const [path, source] of [[FETCH, read(FETCH)], [HARNESS, read(HARNESS)]]) {
    assert.ok(
      source.includes(NOT_A_REPOSITORY),
      `${path} must resolve ${CORPUS_DIR}/${NOT_A_REPOSITORY}; a reader with its own copy of the ` +
        `list breaks the moment a settings file is added to ${CORPUS_DIR}/`,
    );
    const named = listed
      .filter((file) => file !== NOT_A_REPOSITORY)
      .filter((file) => source.includes(basename(file, JSON_SUFFIX)));
    assert.ok(
      named.length <= MAXIMUM_LISTED_NAMES,
      `${path} names ${named.join(", ")} — that is a second copy of the list in ` +
        `${CORPUS_DIR}/${NOT_A_REPOSITORY}, and a second copy is what went stale. Naming one ` +
        `such file is reading it; naming several is deciding which files are settings`,
    );
  }
  const blocks = recipeBlocks(GATE_TARGET);
  assert.equal(blocks.length, 1, `the Makefile must declare exactly one \`${GATE_TARGET}\` recipe`);
  assert.ok(
    blocks[0].body.includes(GATE),
    `${GATE_TARGET} must run ${GATE}; an unrun gate defends nothing`,
  );
});
