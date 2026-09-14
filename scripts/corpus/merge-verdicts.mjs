#!/usr/bin/env node
// [CORPUS-REGISTER-MERGE] Folds judging passes into the clone registers.
//
// This is the only way a verdict reaches a register. It is mechanical on
// purpose: a register is the independent evidence that this detector is
// accurate, and a human retyping verdicts into it is a place where a judgement
// can quietly change on the way in.
//
// Several judges are sent the same folder and rule independently. Their
// answers only become ground truth if they AGREE — two judges who file one
// pair under two verdicts have between them said something false, and taking
// either answer would write that falsehood into the register and score every
// future engine against it.
//
// So a pair is imported only when EVERY source agrees on it — this
// repository's existing registers and all the judged folders together. Three
// sources agreeing with the register leave the register as it is; two judged
// folders agreeing on a pair the register does not hold add it. Anything with
// a disagreement, or with one judge firm where another would not commit, is
// left out and written to the report instead.
//
// The rules live in `merge-verdicts/pass.mjs`; the report in
// `merge-verdicts/report.mjs`. Spec: `docs/specs/corpus.md`
// [CORPUS-REGISTER-MERGE].
//
// Usage: merge-verdicts.mjs [--dry-run] [--report <path>] <judged-folder>...
//   e.g. merge-verdicts.mjs ~/clone-judging-codex ~/clone-judging-glm5.3

import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { mergePass, MINIMUM_JUDGES, NOT_CLEAR, VERDICTS } from "./merge-verdicts/pass.mjs";
import { disagreementCount, render } from "./merge-verdicts/report.mjs";

const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const REGISTER_DIR = "corpus/register";
const JUDGING_QUEUE = "corpus/judging-queue.json";
const DEFAULT_REPORT = "docs/reports/verdict-merge.md";
/// What a judging workspace holds, and this script reads.
const VERDICTS_FILE = "verdicts.json";
const PINNED_FILE = "PINNED.txt";
const PAIRS_FILE = join("candidates", "pairs.json");
/// The protocol a register cites so anyone can re-judge it the same way.
/// `corpus_register_contract` resolves every one of these on disk.
const PROTOCOL = {
  spec: "docs/specs/corpus.md",
  spec_section: "[CORPUS-REGISTER]",
  judging_skill: ".agents/skills/judge-clone-pairs/SKILL.md",
  preparer_skill: ".agents/skills/clone-register-prepare/SKILL.md",
};
const USAGE =
  "usage: merge-verdicts.mjs [--dry-run] [--root <dir>] [--report <path>] <judged-folder>...";

const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
const writeJson = (path, value) => writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
const isDirectory = (path) => existsSync(path) && statSync(path).isDirectory();

/// The command line, as flags and judged folders.
const parseArguments = (argv) => {
  const folders = [];
  const options = { dryRun: false, report: DEFAULT_REPORT, root: REPO_ROOT };
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === "--dry-run") options.dryRun = true;
    else if (argv[index] === "--report") options.report = argv[(index += 1)];
    else if (argv[index] === "--root") options.root = argv[(index += 1)];
    else if (argv[index].startsWith("--")) throw new Error(`unknown flag ${argv[index]}\n${USAGE}`);
    else folders.push(argv[index]);
  }
  if (folders.length < MINIMUM_JUDGES) {
    throw new Error(
      `${folders.length} judging folder(s) given; ${MINIMUM_JUDGES} independent passes are ` +
        `required before any verdict is recorded\n${USAGE}`,
    );
  }
  return { folders, options };
};

/// The repository directories one judged folder holds, by workspace name.
const workspacesIn = (folder) =>
  readdirSync(folder).filter((name) => existsSync(join(folder, name, VERDICTS_FILE)));

/// The url and commit a workspace was built at.
const pinned = (workspace) => {
  const [url, sha] = readFileSync(join(workspace, PINNED_FILE), "utf8").trim().split("\n");
  return { url: url.trim(), sha: sha.trim() };
};

/// The pair list a workspace showed its judge, as `number -> occurrences`.
const pairsIn = (workspace) =>
  new Map(readJson(join(workspace, PAIRS_FILE)).pairs.map((pair) => [pair.number, pair.occurrences]));

/// Every judged folder that holds this repository, as `[judge, workspace]`.
const workspacesFor = (folders, slug) =>
  folders
    .map((folder) => [folder.split("/").filter(Boolean).pop(), join(folder, slug)])
    .filter(([, workspace]) => existsSync(join(workspace, VERDICTS_FILE)));

/// Refuses to compare judges who were not shown the same thing.
///
/// Candidate numbers are the only handle a verdict has on a pair, so two
/// workspaces whose pair lists differ would have their verdicts silently
/// cross-matched: judge A's ruling on its candidate 41 filed against judge B's
/// entirely different candidate 41. That is not a disagreement, it is a
/// fabricated one, and it would poison both directions of the register.
const assertOneCandidateSet = (slug, workspaces) => {
  const shown = workspaces.map(([judge, workspace]) => [
    judge,
    readFileSync(join(workspace, PAIRS_FILE), "utf8"),
  ]);
  const [[first, expected]] = shown;
  for (const [judge, seen] of shown) {
    if (seen !== expected) {
      throw new Error(
        `${slug}: ${judge} and ${first} were shown different candidate lists, so their ` +
          `candidate numbers name different pairs and cannot be compared`,
      );
    }
  }
};

/// Refuses to merge verdicts read at a commit the register was not judged at.
const assertOneCommit = (slug, register, workspaces) => {
  const shas = new Set(workspaces.map(([, workspace]) => pinned(workspace).sha));
  if (register.sha) shas.add(register.sha);
  if (shas.size > 1) {
    throw new Error(
      `${slug}: verdicts span commits ${[...shas].join(", ")} — a line number means nothing ` +
        `without the tree it was read in`,
    );
  }
};

/// The language a repository is judged in, from its register or the queue.
const languageOf = (root, slug, register) => {
  if (register.language) return register.language;
  const queued = readJson(join(root, JUDGING_QUEUE)).repositories.find(
    (repository) => repository.name.toLowerCase() === slug.toLowerCase(),
  );
  if (!queued) {
    throw new Error(
      `${slug} has neither a register nor a queue entry, so nothing says what language it is`,
    );
  }
  return queued.language;
};

/// The register a repository starts this merge from: the one on disk, or a
/// fresh one pinned to the commit its judges actually read.
const startingRegister = (root, slug, registerPath, workspaces) => {
  if (existsSync(registerPath)) return readJson(registerPath);
  const [, workspace] = workspaces[0];
  const { url, sha } = pinned(workspace);
  return { name: slug.toLowerCase(), language: languageOf(root, slug, {}), url, sha, protocol: PROTOCOL };
};

/// States in words why a register records no false positives, so nobody reads
/// an empty list as evidence that precision is good.
const clearlyOutStatus = (result, candidates) => {
  const split = result.disagreements.filter((entry) =>
    entry.rulings.some((ruling) => ruling.verdict === "clearly_out"),
  ).length;
  return (
    `NONE FOUND. ${result.judges} judges ruled independently on ${candidates} candidates at ` +
    `this commit and no pair was called CLEARLY OUT by all of them. ${split} pair(s) drew a ` +
    `CLEARLY OUT from one judge and a different verdict from another; those are listed in ` +
    `${DEFAULT_REPORT} and assert nothing.`
  );
};

/// Merges one repository, returning everything the report needs.
const mergeRepository = (root, folders, slug) => {
  const workspaces = workspacesFor(folders, slug);
  const registerPath = join(root, REGISTER_DIR, `${slug.toLowerCase()}.json`);
  const hadRegister = existsSync(registerPath);
  assertOneCandidateSet(slug, workspaces);
  const register = startingRegister(root, slug, registerPath, workspaces);
  assertOneCommit(slug, register, workspaces);
  const pairs = pairsIn(workspaces[0][1]);
  const passes = workspaces.map(([judge, workspace]) => [
    judge,
    readJson(join(workspace, VERDICTS_FILE)),
  ]);
  const result = mergePass({ register, pairs, passes });
  if (result.merged.clearly_out.length === 0 && !result.merged.clearly_out_status) {
    result.merged.clearly_out_status = clearlyOutStatus(result, pairs.size);
  }
  return { name: slug.toLowerCase(), slug, language: register.language, sha: register.sha, hadRegister, registerPath, result };
};

/// Takes every repository that now has a register out of the judging queue. A
/// queue that never drains scans the same repository forever and buys nothing.
const drainQueue = (root, merged, dryRun) => {
  const path = join(root, JUDGING_QUEUE);
  const queue = readJson(path);
  const names = new Set(merged.map((repo) => repo.name));
  const kept = queue.repositories.filter((repo) => !names.has(repo.name.toLowerCase()));
  const drained = queue.repositories
    .filter((repo) => names.has(repo.name.toLowerCase()))
    .map((repo) => repo.name);
  if (drained.length > 0 && !dryRun) writeJson(path, { ...queue, repositories: kept });
  return { drained, remaining: kept.length };
};

/// Prints what happened, per repository, so a run is legible without opening
/// the report.
const announce = (repos, reportPath, dryRun, disputed) => {
  for (const repo of repos) {
    const added = VERDICTS.map((verdict) => `+${repo.result.added[verdict].length} ${verdict}`);
    console.log(`${repo.name}: ${added.join(", ")}`);
  }
  console.log(`${disputed} pair(s) left out: not every source agreed. See ${reportPath}`);
  if (dryRun) console.log("dry run; nothing written.");
};

/// Writes the merge: every register, then the queue.
const apply = (root, repos, dryRun) => {
  if (dryRun) return;
  for (const repo of repos) writeJson(repo.registerPath, repo.result.merged);
  drainQueue(root, repos, dryRun);
};

const main = () => {
  const { folders, options } = parseArguments(process.argv.slice(2));
  for (const folder of folders) {
    if (!isDirectory(folder)) throw new Error(`not a judging folder: ${folder}`);
  }
  const slugs = [...new Set(folders.flatMap(workspacesIn))].sort();
  if (slugs.length === 0) throw new Error(`no ${VERDICTS_FILE} under any of ${folders.join(", ")}`);
  const repos = slugs.map((slug) => mergeRepository(options.root, folders, slug));

  const reportPath = join(options.root, options.report);
  mkdirSync(dirname(reportPath), { recursive: true });
  writeFileSync(reportPath, render({ sources: folders, repos }));

  apply(options.root, repos, options.dryRun);
  announce(repos, options.report, options.dryRun, disagreementCount(repos));
};

main();
