// [GITHUB-DEPENDABOT] Dependabot's two pull-request classes, against the four
// gates a change to `main` must pass.
//
// Routine VERSION bumps target `dependabot-upgrades` and are swept there; they
// must never fire the `main` gates. A SECURITY update is the one Dependabot PR
// GitHub opens against `main` (it ignores `target-branch`), and it must get
// exactly what a human PR gets: CI, dependency review, CodeQL and, when it
// touches the action, the Action self-test. An actor skip once dropped all of
// that for security PRs alone (gh #388); every existing test stayed green
// (gh #394). These assertions read the workflow files themselves.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { repoRoot } from "../lib/repo-root.mjs";
import { leadingSpaces, mappingValues } from "./action-yaml.mjs";

/** The workflows gating a pull request into `main`, and the one job that is dependency review. */
const MAIN_GATES = ["ci.yml", "codeql.yml", "action-selftest.yml"];
const DEPENDENCY_REVIEW_ACTION = "actions/dependency-review-action";
const DEPENDENCY_REVIEW_JOB = "security:";
/** The default branch, and the staging branch routine bumps are swept into. */
const MAIN = "main";
const STAGING = "dependabot-upgrades";
/** What an actor skip looks like in a workflow, in any of its spellings. */
const ACTOR_SKIP_MARKERS = ["github.actor", "dependabot[bot]", "$ACTOR", "${{ github.triggering_actor"];

/** A workflow's text, by file name. */
function workflow(name) {
  return readFileSync(resolve(repoRoot, ".github", "workflows", name), "utf8");
}

/**
 * The branches a workflow's `pull_request` trigger accepts, read from its
 * top-level `on:` block in either list spelling (`[main]` or `- main`).
 */
function pullRequestBranches(source) {
  const lines = source.split("\n");
  const on = lines.findIndex((line) => line === "on:");
  assert.ok(on >= 0, "the workflow has a top-level on: block");
  const trigger = lines.findIndex((line, index) => index > on && line.trim() === "pull_request:");
  assert.ok(trigger > on, "the workflow triggers on pull_request");
  const depth = leadingSpaces(lines[trigger]);
  const block = [];
  for (const line of lines.slice(trigger + 1)) {
    if (line.trim() !== "" && !line.trim().startsWith("#") && leadingSpaces(line) <= depth) break;
    block.push(line);
  }
  const text = block.join("\n");
  const inline = mappingValues(text, "branches").find((value) => value.startsWith("["));
  if (inline) return inline.slice(1, -1).split(",").map((branch) => branch.trim());
  return block.map((line) => line.trim()).filter((line) => line.startsWith("- ")).map((line) => line.slice(2).trim());
}

test("[GITHUB-DEPENDABOT] every main gate runs for a pull request into main, whoever opened it", () => {
  for (const name of MAIN_GATES) {
    const source = workflow(name);
    assert.ok(pullRequestBranches(source).includes(MAIN), `${name} gates pull requests into ${MAIN}`);
    for (const marker of ACTOR_SKIP_MARKERS) {
      assert.ok(!source.includes(marker), `${name} must not gate on the actor (${marker}): a Dependabot security PR into main would skip it`);
    }
  }
});

test("[GITHUB-DEPENDABOT] CI's dependency review runs for a Dependabot security PR", () => {
  const ci = workflow("ci.yml");
  const start = ci.indexOf(`  ${DEPENDENCY_REVIEW_JOB}`);
  assert.ok(start >= 0, "ci.yml has the security job");
  const job = ci.slice(start, ci.indexOf("\n  ", ci.indexOf(DEPENDENCY_REVIEW_ACTION, start)));
  assert.ok(job.includes(DEPENDENCY_REVIEW_ACTION), "the security job is the dependency review");
  for (const condition of mappingValues(job, "if")) {
    for (const marker of ACTOR_SKIP_MARKERS) {
      assert.ok(!condition.includes(marker), `the dependency review must not skip on the actor: if: ${condition}`);
    }
  }
});

test("[GITHUB-DEPENDABOT] every routine version bump targets the staging branch, never main", () => {
  const config = readFileSync(resolve(repoRoot, ".github", "dependabot.yml"), "utf8");
  const ecosystems = mappingValues(config, "- package-ecosystem");
  const targets = mappingValues(config, "target-branch").map((value) => value.replaceAll('"', ""));
  assert.ok(ecosystems.length > 0, "Dependabot watches at least one ecosystem");
  assert.equal(targets.length, ecosystems.length, "every ecosystem names its target branch");
  assert.deepEqual([...new Set(targets)], [STAGING], `every routine bump targets ${STAGING}`);
});

test("[GITHUB-DEPENDABOT] the sweep only ever takes PRs based on the staging branch", () => {
  const branches = pullRequestBranches(workflow("dependabot-automerge.yml"));
  assert.deepEqual(branches, [STAGING], `the sweep must not take a security PR out of ${MAIN}`);
});
