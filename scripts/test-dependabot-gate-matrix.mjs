// [GITHUB-DEPENDABOT-SECURITY-GATES] Dependabot event-matrix contract.
//
// Two classes of Dependabot pull request exist, and they reach opposite halves
// of this repository's CI:
//
//   * ROUTINE VERSION BUMP — `.github/dependabot.yml` gives every ecosystem
//     `target-branch: dependabot-upgrades`, so the PR is opened against the
//     staging branch. The four `main`-only gates must NOT be instantiated for
//     it (that is the whole point of the staging branch), and the sweep in
//     dependabot-automerge.yml must be.
//   * SECURITY BUMP — GitHub ignores `target-branch` for security updates and
//     always opens them against the default branch. The sweep's base filter is
//     `dependabot-upgrades` only, so nothing carries this PR away: it merges to
//     `main`. It must therefore clear exactly the gates a human PR clears.
//
// An actor short-circuit (`github.actor != 'dependabot[bot]'`) inverts that: it
// cannot fire on a routine bump, because the workflow is never triggered for
// one, and it always fires on the security bump — the one PR class opened
// because of a known vulnerability. #388 shipped that hole across all four
// gates; this suite is the assertion that keeps it closed (#394).
//
// Everything below is read from a real YAML parse, never from a text match, so
// a reformat cannot make it pass and a re-added skip cannot hide behind
// indentation.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parse } from "yaml";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));

// The four gates a pull request against `main` must clear. `ci.yml`'s build and
// test matrix is reached through the `changes` classifier — every heavy job
// keys off its `code`/`site` outputs, so an actor short-circuit inside the
// classifier disables all of them at once, which is how #388 hid.
const GATES = [
  { workflow: "ci.yml", job: "changes", gate: "build/test/coverage matrix (via the changes classifier)" },
  { workflow: "ci.yml", job: "security", gate: "dependency review [GITHUB-DEP-REVIEW]" },
  { workflow: "codeql.yml", job: "analyze", gate: "CodeQL [GITHUB-CODE-SCANNING]" },
  { workflow: "action-selftest.yml", job: "contract", gate: "Action self-test [ACTION-TESTS]" },
];

// The write scopes each gate needs to do its job. A Dependabot-triggered run
// gets a READ-ONLY GITHUB_TOKEN unless the workflow raises it with an explicit
// `permissions:` block, so these declarations are what make the gates work on
// exactly the PR class this suite exists for. Deleting one breaks the gate on
// Dependabot PRs and nowhere else — silently.
const REQUIRED_WRITE_SCOPES = [
  { workflow: "ci.yml", job: "security", scope: "pull-requests", why: "dependency-review posts its PR summary" },
  { workflow: "codeql.yml", job: "analyze", scope: "security-events", why: "CodeQL uploads SARIF to code scanning" },
];

// Every context that identifies who opened the pull request. Checking only
// `github.actor` would leave the skip one rename away from returning: the same
// short-circuit written against `github.event.pull_request.user.login` behaves
// identically and would sail past a narrower assertion. The bare name
// `dependabot[bot]` is flagged too — none of the four gates has any legitimate
// reason to name the bumper in an expression at all.
const AUTHOR_CONTEXTS = [
  "github.actor",
  "github.triggering_actor",
  "github.event.pull_request.user.login",
  "github.event.sender.login",
  "dependabot[bot]",
];

const SWEEP_WORKFLOW = "dependabot-automerge.yml";
const STAGING_BRANCH = "dependabot-upgrades";
const DEFAULT_BRANCH = "main";

const tests = [
  securityBumpAgainstMainClearsEveryGate,
  securityBumpGatesRaiseTheTokenAboveDependabotReadOnly,
  routineVersionBumpNeverInstantiatesTheMainGates,
  everyEcosystemTargetsTheStagingBranch,
  theSweepIsTheOnlyActorGatedJobAndStagingOnly,
];

let failed = 0;
for (const test of tests) {
  try {
    test();
    console.log(`ok ${test.name}`);
  } catch (error) {
    failed++;
    console.error(`not ok ${test.name}`);
    console.error(`  ${error instanceof Error ? error.message : String(error)}`);
  }
}

if (failed > 0) {
  console.error(`\n${failed} dependabot gate matrix test(s) failed`);
  process.exit(1);
}
console.log(`\n${tests.length} dependabot gate matrix tests passed`);

// ---------------------------------------------------------------- the matrix

// A security bump lands on `main` and no sweep removes it, so every gate must
// both fire for it and be incapable of excusing itself because of who opened it.
function securityBumpAgainstMainClearsEveryGate() {
  const skips = [...gatesByWorkflow()].flatMap(([workflow, gates]) => actorSkipsIn(workflow, gates));
  if (skips.length > 0) {
    throw new Error(
      `${skips.length} actor short-circuit(s) skip a gate for the only Dependabot PR class that can reach ` +
        `${DEFAULT_BRANCH} — a security bump. Routine bumps target ${STAGING_BRANCH} and never trigger these ` +
        `workflows, so each skip saves nothing and costs the CVE fix its gate (#388):\n  ${skips.join("\n  ")}`,
    );
  }
}

// Proves the workflow still fires for a PR against `main` and still defines the
// jobs being gated, then reports every place it consults the author.
function actorSkipsIn(workflow, gates) {
  const parsed = loadWorkflow(workflow);
  assertIncludes(
    pullRequestBases(parsed, workflow),
    DEFAULT_BRANCH,
    `${workflow} must stay subscribed to pull requests against ${DEFAULT_BRANCH}, or ${gates.join(" and ")} never runs on a Dependabot security bump`,
  );
  for (const { job } of GATES.filter((gate) => gate.workflow === workflow)) assertJobExists(parsed, job, workflow);
  return authorReferences(parsed).map(
    ({ path, expression }) => `${workflow}:${path} — \`${expression}\` skips ${gates.join(" and ")}`,
  );
}

// The gates `ci.yml` owns are two independent jobs, so a single short-circuit
// there costs both. Grouping keeps the failure message honest about which.
function gatesByWorkflow() {
  return GATES.reduce(
    (grouped, { workflow, gate }) => grouped.set(workflow, [...(grouped.get(workflow) ?? []), gate]),
    new Map(),
  );
}

// The gates that need write scopes must declare them at job level: the token on
// a Dependabot-triggered run is read-only by default, so an inherited default
// would leave dependency review unable to comment and CodeQL unable to upload.
function securityBumpGatesRaiseTheTokenAboveDependabotReadOnly() {
  for (const { workflow, job, scope, why } of REQUIRED_WRITE_SCOPES) {
    const parsed = loadWorkflow(workflow);
    const permissions = assertJobExists(parsed, job, workflow).permissions;
    if (!permissions || typeof permissions !== "object") {
      throw new Error(
        `${workflow} job \`${job}\` declares no job-level permissions block; a Dependabot-triggered run then ` +
          `gets a read-only token and ${why} fails on exactly the PR class this gate exists for`,
      );
    }
    if (permissions[scope] !== "write") {
      throw new Error(
        `${workflow} job \`${job}\` must declare \`${scope}: write\` (${why}); got \`${scope}: ${permissions[scope]}\``,
      );
    }
  }
}

// The staging branch's purpose is that the expensive matrix does NOT run per
// bump. Subscribing any gate to it would burn the matrix on every routine bump
// and re-create the cost the actor skip was mistakenly written to avoid.
function routineVersionBumpNeverInstantiatesTheMainGates() {
  for (const { workflow, gate } of GATES) {
    assertExcludes(
      pullRequestBases(loadWorkflow(workflow), workflow),
      STAGING_BRANCH,
      `${workflow} must not subscribe to pull requests against ${STAGING_BRANCH}: ${gate} is paid once, on the ` +
        `${STAGING_BRANCH} -> ${DEFAULT_BRANCH} consolidation PR, not on every routine bump`,
    );
  }
}

// The premise of the whole matrix: routine bumps miss the gates only because
// every ecosystem points them at staging. Drop one `target-branch` and that
// ecosystem's bumps start arriving on `main` unannounced.
function everyEcosystemTargetsTheStagingBranch() {
  const config = parse(readFileSync(resolve(repoRoot, ".github/dependabot.yml"), "utf8"));
  const updates = config?.updates;
  if (!Array.isArray(updates) || updates.length === 0) {
    throw new Error(".github/dependabot.yml declares no `updates:` entries");
  }
  for (const entry of updates) {
    const where = entry?.directory ?? JSON.stringify(entry?.directories);
    if (entry?.["target-branch"] !== STAGING_BRANCH) {
      throw new Error(
        `.github/dependabot.yml ecosystem \`${entry?.["package-ecosystem"]}\` at ${where} must set ` +
          `\`target-branch: ${STAGING_BRANCH}\`; got \`${entry?.["target-branch"]}\`. Without it, routine version ` +
          `bumps open against ${DEFAULT_BRANCH} and every gate above runs per bump.`,
      );
    }
  }
}

// The sweep is the one job for which "is this Dependabot?" is a real question,
// and it must stay staging-only: a `main` subscription would hang a permanently
// `skipped` check on every human PR, and would also start sweeping the security
// bump away from the gates the tests above just proved it must clear.
function theSweepIsTheOnlyActorGatedJobAndStagingOnly() {
  const parsed = loadWorkflow(SWEEP_WORKFLOW);
  const bases = pullRequestBases(parsed, SWEEP_WORKFLOW);
  assertIncludes(bases, STAGING_BRANCH, `${SWEEP_WORKFLOW} must fire on bumps opened against ${STAGING_BRANCH}`);
  assertExcludes(
    bases,
    DEFAULT_BRANCH,
    `${SWEEP_WORKFLOW} must not subscribe to pull requests against ${DEFAULT_BRANCH}: its job is actor-gated, an ` +
      `if:-skipped job still reports a skipped check on every human PR, and sweeping a security bump would strip ` +
      `it of the gates it is required to clear`,
  );
  const actorGates = authorReferences(parsed).map(({ expression }) => expression);
  if (!actorGates.some((expression) => expression.includes(`github.actor == 'dependabot[bot]'`))) {
    throw new Error(
      `${SWEEP_WORKFLOW} must still refuse to act for any actor but Dependabot; found actor gates: ${JSON.stringify(actorGates)}`,
    );
  }
}

// ------------------------------------------------------------------- parsing

function loadWorkflow(name) {
  return parse(readFileSync(resolve(repoRoot, ".github/workflows", name), "utf8"));
}

// GitHub's `on:` key is a plain string under the YAML 1.2 core schema this
// parser uses. Asserting it rather than assuming it keeps a parser change from
// turning every trigger assertion below into a vacuous pass on `undefined`.
function pullRequestBases(parsed, name) {
  const triggers = parsed?.on;
  if (!triggers || typeof triggers !== "object") {
    throw new Error(`${name} has no parsable \`on:\` block (got ${JSON.stringify(triggers)})`);
  }
  const branches = triggers.pull_request?.branches;
  if (!Array.isArray(branches)) {
    throw new Error(`${name} must filter \`on.pull_request.branches\` explicitly; got ${JSON.stringify(branches)}`);
  }
  return branches;
}

function assertJobExists(parsed, job, name) {
  const definition = parsed?.jobs?.[job];
  if (!definition) throw new Error(`${name} has no job \`${job}\` (jobs: ${Object.keys(parsed?.jobs ?? {}).join(", ")})`);
  return definition;
}

// Every place a workflow can consult who opened the pull request: a bare `if:`
// (implicitly an expression), an interpolated `${{ }}` anywhere in the tree —
// including the `env:` binding that carried the actor into ci.yml's classifier
// shell — collected with their YAML paths so a failure names the exact site.
function authorReferences(parsed) {
  return [...expressionsIn(parsed, [])].filter(({ expression }) =>
    AUTHOR_CONTEXTS.some((context) => expression.includes(context)),
  );
}

function* expressionsIn(node, path) {
  if (typeof node === "string") return yield* interpolations(node, path.join("."));
  if (Array.isArray(node)) {
    for (const [index, item] of node.entries()) yield* expressionsIn(item, [...path, `[${index}]`]);
    return;
  }
  if (!node || typeof node !== "object") return;
  for (const [key, value] of Object.entries(node)) {
    if (key === "if" && typeof value === "string") yield { path: [...path, key].join("."), expression: value };
    else yield* expressionsIn(value, [...path, key]);
  }
}

// Split on the delimiters rather than matching a pattern: the expression is an
// embedded language inside a YAML scalar, and its boundaries are literal.
function* interpolations(scalar, path) {
  for (const fragment of scalar.split("${{").slice(1)) {
    const end = fragment.indexOf("}}");
    if (end !== -1) yield { path, expression: fragment.slice(0, end).trim() };
  }
}

// ---------------------------------------------------------------- assertions

function assertIncludes(values, wanted, message) {
  if (!values.includes(wanted)) throw new Error(`${message} (found: ${JSON.stringify(values)})`);
}

function assertExcludes(values, unwanted, message) {
  if (values.includes(unwanted)) throw new Error(`${message} (found: ${JSON.stringify(values)})`);
}
