// [ACTION-PRE-COMMIT] The remote pre-commit/prek hook manifest.
//
// pre-commit reads `.pre-commit-hooks.yaml` from the tag a user pins, so the
// manifest is a published contract: a renamed id or a hook that starts passing
// filenames breaks every repository that adopted it. These assertions read the
// manifest line by line with the same helpers the action gates use.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { repoRoot } from "../lib/repo-root.mjs";
import { mappingValues } from "./action-yaml.mjs";

const MANIFEST = readFileSync(resolve(repoRoot, ".pre-commit-hooks.yaml"), "utf8");
const README = readFileSync(resolve(repoRoot, "README.md"), "utf8");

/** The one hook the manifest publishes, and the fields pre-commit runs it with. */
const HOOK_ID = "deslop";
/** The exact `- repo:` value a user pins; compared whole, never as a substring. */
const REPO_URL = "https://github.com/Nimblesite/Deslop";
const EXPECTED_FIELDS = {
  name: "deslop",
  entry: "deslop",
  language: "system",
  pass_filenames: "false",
  always_run: "true",
  require_serial: "true",
};

/** Hook ids, as `- id: <value>` list entries. */
function hookIds(source) {
  return mappingValues(source, "- id");
}

test("[ACTION-PRE-COMMIT] the manifest publishes exactly one hook, `deslop`", () => {
  assert.deepEqual(hookIds(MANIFEST), [HOOK_ID]);
});

test("[ACTION-PRE-COMMIT] the hook runs the PATH CLI over the whole repository, alone", () => {
  for (const [field, value] of Object.entries(EXPECTED_FIELDS)) {
    assert.deepEqual(mappingValues(MANIFEST, field), [value], `${field} must be ${value}`);
  }
  assert.equal(mappingValues(MANIFEST, "description").length, 1, "the hook describes itself once");
  for (const field of ["files", "types", "args"]) {
    assert.deepEqual(mappingValues(MANIFEST, field), [], `${field} would narrow the scan or fix its options`);
  }
});

test("[ACTION-PRE-COMMIT] the README's hook snippet names the published hook and a real threshold flag", () => {
  const snippetIds = mappingValues(README, "- id");
  assert.ok(snippetIds.includes(HOOK_ID), "the README shows `- id: deslop`");
  const repos = mappingValues(README, "- repo");
  assert.ok(repos.some((repo) => repo === REPO_URL), "the snippet pins this repository");
  assert.ok(README.includes('args: ["--fail-over", "5.0"]'), "the snippet passes the threshold the CI section documents");
});
