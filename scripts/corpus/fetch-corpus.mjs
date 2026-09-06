// [CORPUS-PIN] Clones every corpus repository pinned by `corpus/*.json` into `.corpus/`
// (git-ignored) so the `corpus_*` accuracy and resource suite runs against
// real code at a fixed commit.
//
// Determinism is the whole point: a manifest pins one COMMIT ID and the fetch
// asks the remote for exactly that commit. Never a tag and never a branch — a
// version label is a name upstream can re-point at different source, and a
// curated duplicate list read against different source asserts nothing. The
// resulting HEAD is verified against the pin regardless, so any drift in what
// the remote served fails loudly rather than silently re-baselining the list.
//
// Already-present clones at the right SHA are left alone, so re-running is
// free. Pass `--force` to re-clone from scratch.

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const manifestDir = resolve("corpus");
const cacheDir = resolve(".corpus");
const force = process.argv.includes("--force");

// Bare arguments select which repositories to fetch, e.g.
// `node scripts/corpus/fetch-corpus.mjs tokio nest`. With none, every manifest is
// fetched. CI passes a short list so it clones tens of megabytes rather than
// the ~600 MB the full corpus costs.
const selected = new Set(process.argv.slice(2).filter((arg) => !arg.startsWith("--")));

// `corpus/` also holds the known-failures baseline, which is not a repository.
// Excluded by name rather than by shape, so a genuinely malformed manifest
// still fails loudly instead of being silently skipped.
const NON_MANIFEST = new Set(["known-failures.json"]);

/// [CORPUS-PIN] The only pin a manifest may carry: a full git object name.
const COMMIT_ID_LENGTH = 40;
const COMMIT_ID_ALPHABET = "0123456789abcdef";
const isCommitId = (value) =>
  value.length === COMMIT_ID_LENGTH &&
  [...value].every((character) => COMMIT_ID_ALPHABET.includes(character));
/// How much of a commit id names a clone directory and reads in a log line.
const SHORT_SHA_LENGTH = 12;
/// Ceiling on any one git invocation; a hung fetch must not hang the corpus.
const TIMEOUT_MS = 900_000;

const manifests = readdirSync(manifestDir)
  .filter((name) => name.endsWith(".json") && !NON_MANIFEST.has(name))
  .map((name) => JSON.parse(readFileSync(join(manifestDir, name), "utf8")))
  .filter((manifest) => selected.size === 0 || selected.has(manifest.name));

if (manifests.length === 0) {
  const wanted = selected.size === 0 ? "" : ` matching ${[...selected].join(", ")}`;
  throw new Error(`no corpus manifests in ${manifestDir}${wanted}`);
}

const unknown = [...selected].filter((name) => !manifests.some((m) => m.name === name));
if (unknown.length > 0) throw new Error(`unknown corpus repositories: ${unknown.join(", ")}`);

mkdirSync(cacheDir, { recursive: true });
for (const manifest of manifests) {
  fetchRepo(manifest);
}

function fetchRepo({ name, url, sha }) {
  for (const [field, value] of Object.entries({ name, url, sha })) {
    if (!value) throw new Error(`corpus manifest is missing "${field}"`);
  }
  if (!isCommitId(sha)) {
    throw new Error(
      `${name}: "${sha}" is not a ${COMMIT_ID_LENGTH}-character commit id. ` +
        `Pin the commit, never a tag or a version.`,
    );
  }
  const target = join(cacheDir, `${name}-${sha.slice(0, SHORT_SHA_LENGTH)}`);

  if (existsSync(target)) {
    if (!force && headSha(target) === sha) {
      console.log(`${name}: already at ${sha.slice(0, 12)}`);
      return;
    }
    rmSync(target, { recursive: true, force: true });
  }

  console.log(`${name}: fetching ${url} @ ${sha.slice(0, SHORT_SHA_LENGTH)}`);
  mkdirSync(target, { recursive: true });
  run("git", ["-c", "init.defaultBranch=main", "-C", target, "init", "--quiet"]);
  run("git", ["-C", target, "remote", "add", "origin", url]);
  if (!tryRun("git", ["-C", target, "fetch", "--quiet", "--depth", "1", "origin", sha])) {
    console.log(`${name}: remote will not serve a single commit, fetching in full`);
    run("git", ["-C", target, "fetch", "--quiet", "origin"]);
  }
  run("git", ["-C", target, "checkout", "--quiet", sha]);

  const actual = headSha(target);
  if (actual !== sha) {
    rmSync(target, { recursive: true, force: true });
    throw new Error(
      `${name}: checkout landed on ${actual}, but the manifest pins ${sha}.`,
    );
  }
  console.log(`${name}: verified at ${sha.slice(0, SHORT_SHA_LENGTH)}`);
}

function headSha(repo) {
  const result = spawnSync("git", ["-C", repo, "rev-parse", "HEAD"], {
    encoding: "utf8",
    stdio: "pipe",
  });
  return result.status === 0 ? result.stdout.trim() : "";
}

function tryRun(command, args) {
  return (
    spawnSync(command, args, { encoding: "utf8", stdio: "pipe", timeout: TIMEOUT_MS })
      .status === 0
  );
}

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    stdio: "inherit",
    timeout: TIMEOUT_MS,
  });
  if (result.status !== 0) throw new Error(`${command} ${args.join(" ")} failed`);
}
