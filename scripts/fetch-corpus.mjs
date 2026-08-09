// Clones every corpus repository pinned by `corpus/*.json` into `.corpus/`
// (git-ignored) so the `corpus_*` accuracy and resource suite runs against
// real code at a fixed commit.
//
// Determinism is the whole point: a manifest names both a tag and the SHA
// that tag pointed at when the corpus was curated. The clone goes through
// the tag (cheap, shallow) and the resulting HEAD is then verified against
// the pinned SHA, so a moved or re-cut upstream tag fails loudly instead of
// silently re-baselining the curated duplicate list.
//
// Already-present clones at the right SHA are left alone, so re-running is
// free. Pass `--force` to re-clone from scratch.

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const manifestDir = resolve("corpus");
const cacheDir = resolve(".corpus");
const force = process.argv.includes("--force");

// `corpus/` also holds the known-failures baseline, which is not a repository.
// Excluded by name rather than by shape, so a genuinely malformed manifest
// still fails loudly instead of being silently skipped.
const NON_MANIFEST = new Set(["known-failures.json"]);

const manifests = readdirSync(manifestDir)
  .filter((name) => name.endsWith(".json") && !NON_MANIFEST.has(name))
  .map((name) => JSON.parse(readFileSync(join(manifestDir, name), "utf8")));

if (manifests.length === 0) throw new Error(`no corpus manifests in ${manifestDir}`);

mkdirSync(cacheDir, { recursive: true });
for (const manifest of manifests) {
  fetchRepo(manifest);
}

function fetchRepo({ name, url, tag, sha }) {
  for (const [field, value] of Object.entries({ name, url, tag, sha })) {
    if (!value) throw new Error(`corpus manifest is missing "${field}"`);
  }
  const target = join(cacheDir, `${name}-${tag}`);

  if (existsSync(target)) {
    if (!force && headSha(target) === sha) {
      console.log(`${name}: already at ${sha.slice(0, 12)}`);
      return;
    }
    rmSync(target, { recursive: true, force: true });
  }

  console.log(`${name}: cloning ${url} @ ${tag}`);
  run("git", ["clone", "--depth", "1", "--branch", tag, url, target]);

  const actual = headSha(target);
  if (actual !== sha) {
    rmSync(target, { recursive: true, force: true });
    throw new Error(
      `${name}: tag ${tag} resolved to ${actual}, but the manifest pins ${sha}. ` +
        `Upstream moved the tag. Re-verify the curated duplicate list against the ` +
        `new commit before updating corpus/${name}.json.`,
    );
  }
  console.log(`${name}: verified at ${sha.slice(0, 12)}`);
}

function headSha(repo) {
  const result = spawnSync("git", ["-C", repo, "rev-parse", "HEAD"], {
    encoding: "utf8",
    stdio: "pipe",
  });
  return result.status === 0 ? result.stdout.trim() : "";
}

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    stdio: "inherit",
    timeout: 900_000,
  });
  if (result.status !== 0) throw new Error(`${command} ${args.join(" ")} failed`);
}
