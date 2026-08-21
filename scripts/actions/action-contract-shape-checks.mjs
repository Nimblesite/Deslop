// Contract checks for the static shape of action.yml, the self-test workflow,
// and the documented pins. [ACTION-TESTS]
//
// Imported for its side effects by test-action-contract.mjs, which prints the
// suite total.

import { readFileSync } from "node:fs";
import assert from "node:assert/strict";

import { check } from "./action-contract-harness.mjs";
import { resolveVersion } from "./action-resolve-artifact.mjs";
import { actionPinDocs, readActionPins, PIN_PLACEHOLDER } from "../release/stamp-release-version.mjs";
import * as releasesData from "../site/src/_data/releases.js";

const action = readFileSync("action.yml", "utf8");

check("action.yml declares the Marketplace-required metadata", () => {
  for (const field of ["name: Deslop.live", "description:", "author: Nimblesite", "using: composite"]) {
    assert.ok(action.includes(field), `action.yml is missing ${field}`);
  }
  assert.ok(action.includes("icon: copy"), "branding icon must be a Feather icon name");
  assert.ok(action.includes("color: purple"), "branding colour must be one of the nine allowed values");
});

// GitHub refuses to list an action whose name matches a user or organization,
// unless that account is the publisher. A dormant unrelated org holds `deslop`,
// so the bare product name is permanently unlistable here — a substring check on
// "name: Deslop" would accept it and the rejection would only surface in the
// publish form, after the tag was cut. Assert the whole line. [ACTION-METADATA]
check("the Marketplace name is not the org-colliding bare product name", () => {
  const declared = action.split("\n").find((line) => line.startsWith("name:"));
  assert.equal(declared, "name: Deslop.live", "action.yml name line must be exactly `name: Deslop.live`");
});

check("every documented input is declared", () => {
  const inputs = [
    "path", "version", "fail-over", "no-fail-over", "min-nodes", "config", "diff",
    "only-changed", "cache", "output", "nojson", "notext", "nohtml", "log-level",
    "upload-artifact", "artifact-name",
  ];
  for (const input of inputs) assert.ok(action.includes(`\n  ${input}:`), `action.yml lost input ${input}`);
});

// The diff flags exist on the CLI as [CLI-ARG-DIFF] / [CLI-ARG-ONLY-CHANGED];
// the action must hand them through verbatim, and only when actually set — a
// bare `--diff ""` would be a usage error on every run.
check("the diff inputs are forwarded to the CLI only when set", () => {
  assert.ok(
    action.includes('if [ -n "${DIFF}" ]; then args+=(--diff "${DIFF}"); fi'),
    "the run step must forward a non-empty diff input as --diff",
  );
  assert.ok(
    action.includes('if [ "${ONLY_CHANGED}" = "true" ]; then args+=(--only-changed); fi'),
    "the run step must forward only-changed: \"true\" as --only-changed",
  );
  assert.ok(action.includes("DIFF: ${{ inputs.diff }}"), "the diff input must reach the run step through env");
  assert.equal(
    action.split("ONLY_CHANGED: ${{ inputs.only-changed }}").length - 1,
    2,
    "the run step and the report step must both receive the only-changed input through env",
  );
});

// `--only-changed` without `--diff` is a CLI usage error (exit 2,
// [CLI-ARG-ONLY-CHANGED]); the action fails it before a CLI is downloaded,
// with the input names the caller wrote and the status the CLI would have
// exited. [ACTION-GATE]
check("only-changed without a diff fails fast with the CLI's usage status", () => {
  const guard = "if: inputs.only-changed == 'true' && inputs.diff == ''";
  const guardAt = action.indexOf(guard);
  assert.ok(guardAt >= 0, "action.yml lost the only-changed-without-diff guard");
  assert.ok(
    guardAt < action.indexOf("action-resolve-artifact.mjs"),
    "the guard must run before any CLI is resolved or downloaded",
  );
  const guardBody = action.slice(guardAt, action.indexOf("- name:", guardAt));
  assert.ok(guardBody.includes("exit 2"), "the guard must exit 2, the status the CLI would have used");
  assert.ok(guardBody.includes("::error::"), "the guard must emit a workflow error annotation");
});

// A breach message names the population the percentage is *of*: under
// `only-changed` the gate reads duplicated added lines over added lines
// ([METRICS-DIFF-SCOPE]), so naming the repo-wide population there would
// attribute the failure to the whole tree when only the diff was measured.
// [ACTION-GATE]
check("a diff-gated breach names the added-lines population", () => {
  assert.ok(
    action.includes('if [ "${SCOPE}" = "added-lines" ]; then'),
    "the gate step must branch its breach message on gate-scope",
  );
  assert.ok(
    action.includes("% of added lines are duplicated"),
    "the diff-scoped breach must name the added-lines population",
  );
  assert.ok(
    action.includes("% of analyzed lines are duplicated"),
    "the repo-wide breach must name the analyzed-lines population",
  );
  assert.ok(
    action.includes("SCOPE: ${{ steps.report.outputs.gate-scope }}"),
    "the gate scope must come from the report step, which read it out of the JSON",
  );
  assert.ok(
    action.includes("MEASURED: ${{ steps.report.outputs.gate-percent }}"),
    "the measured figure must be the one the gate actually used",
  );
  assert.ok(
    action.includes("CEILING: ${{ steps.report.outputs.gate-threshold-percent }}"),
    "the ceiling must be the one the gate actually used",
  );
});

// Phase 10's self-test exit criterion: a legacy-heavy fixture passes the gate
// under `only-changed` with a clean diff. [ACTION-TESTS] [METRICS-DIFF-SCOPE]
check("the self-test carries the diff-gate leg, version-gated on the release", () => {
  const selftest = readFileSync(".github/workflows/action-selftest.yml", "utf8");
  assert.ok(
    selftest.includes("if: needs.contract.outputs.diff-flags == 'true'"),
    "the diff-gate job must be skipped until a published release carries the diff flags",
  );
  assert.ok(
    selftest.includes('only-changed: "true"'),
    "the diff-gate leg must run the action under only-changed",
  );
  assert.ok(selftest.includes("diff: empty.patch"), "the diff-gate leg must scope to the empty diff");
  // Both directions, or the job proves only that the gate can pass —
  // which a gate that never fires would also satisfy. [ACTION-GATE]
  assert.ok(
    selftest.includes("diff: change.patch"),
    "the diff-gate job must also run a diff that adds duplication",
  );
  assert.ok(
    selftest.includes("SCOPE: ${{ steps.breach.outputs.gate-scope }}"),
    "the breaching leg must assert the gate scope the action published",
  );
  // Both legs must build the breaching patch with the same script. A shell
  // twin of it here counted lines with `wc -l`, which counts terminators —
  // on a fixture with no trailing newline the hunk header declared one line
  // fewer than the body carried and the parser refused the patch outright,
  // failing the gate proof for a reason unrelated to the gate. [ACTION-GATE]
  assert.ok(
    selftest.includes("node scripts/actions/action-copy-patch.mjs"),
    "the breaching leg must build its patch with the shared copy-patch script, never a shell twin",
  );
  assert.ok(
    readFileSync("scripts/actions/test-action-diff-gate.mjs", "utf8").includes(
      'from "./action-copy-patch.mjs"',
    ),
    "the branch-built proof must gate on the same patch the runner leg does",
  );
});

// The branch-executed counterpart: the workflow leg above installs a
// published release, so the pre-release action path needs a proof that
// runs against the freshly built CLI. [ACTION-GATE] [METRICS-DIFF-SCOPE]
check("the branch-built action diff-gate proof runs in the deployment gate", () => {
  const proof = readFileSync("scripts/actions/test-action-diff-gate.mjs", "utf8");
  assert.ok(
    proof.includes('stepBody(readFileSync("action.yml", "utf8"), "Run deslop")'),
    "the proof must execute the action's own step body, never a re-implementation of it",
  );
  assert.ok(
    proof.includes('readOutputs(outputPrefix, exitCode, true)'),
    "the proof must publish outputs through the real action-read-outputs script",
  );
  const makefile = readFileSync("Makefile", "utf8");
  assert.ok(
    makefile.includes("node scripts/actions/test-action-diff-gate.mjs"),
    "deployment-verify must run the branch-built action diff-gate proof",
  );
});

// The list is derived from the helper, never hand-maintained beside it. A
// hand-written list is how the three gate outputs came to be computed,
// consumed by the action's own gate step, and never exported: both the
// declaration and the "every output" check listed the same older seven, so
// the contract test agreed with the bug. `steps.<id>.outputs.gate-scope` read
// empty for every caller, and the hosted self-test's assertion on it could
// only fail after a release, not before one.
check("every output the helper emits is declared and wired", () => {
  const helper = readFileSync(
    new URL("./action-read-outputs.mjs", import.meta.url),
    "utf8",
  );
  const emitted = [...helper.matchAll(/"([a-z][a-z-]*)":/g)].map((match) => match[1]);
  const outputs = [
    "duplication-percent", "cluster-count", "threshold-percent",
    "exit-code", "report-json", "report-text", "report-html",
    "gate-scope", "gate-percent", "gate-threshold-percent",
  ];
  for (const output of outputs) {
    assert.ok(
      emitted.includes(output),
      `action-read-outputs.mjs no longer emits ${output}`,
    );
    assert.ok(
      action.includes(`value: \${{ steps.report.outputs.${output} }}`),
      `action.yml output ${output} is not wired to the report step`,
    );
    assert.ok(
      new RegExp(`^  ${output}:$`, "m").test(action),
      `action.yml does not declare ${output} in its public outputs block`,
    );
  }
  for (const output of emitted) {
    assert.ok(
      outputs.includes(output),
      `action-read-outputs.mjs emits ${output}, which no contract check covers`,
    );
  }
});

// The advertised-but-unsuppliable stdin diff must fail closed at the
// composite boundary. A `uses:` step has no stdin, so `--diff -` reads an
// empty patch, and `--only-changed` then measures 0/0 = 0% and passes any
// ceiling while omitting every cluster in the tree — a changed-code false
// negative at the merge gate the feature exists to be.
check("the action rejects the stdin diff form before downloading a CLI", () => {
  assert.ok(
    action.includes("if: inputs.diff == '-'"),
    "action.yml must guard the diff: \"-\" input",
  );
  const guard = action.indexOf("if: inputs.diff == '-'");
  const resolve = action.indexOf("name: Resolve the deslop release");
  assert.ok(
    guard < resolve,
    "the stdin-diff guard must run before the CLI is resolved and downloaded",
  );
  assert.ok(
    /if: inputs\.diff == '-'[\s\S]{0,600}?exit 2/.test(action),
    "the stdin-diff guard must exit 2, the CLI's own usage-error status",
  );
  assert.ok(
    !/or "-" to read the diff from stdin/.test(action),
    "action.yml must not advertise a stdin diff it cannot supply",
  );
});

// Every nested third-party action must be pinned to an immutable commit, not a
// movable tag ([SWR-SEC-ACTION-PINNING]) — a re-pointed tag in a dependency
// would execute arbitrary code inside every consumer's workflow.
check("every nested action is pinned to a full-length commit SHA", () => {
  for (const marker of ["actions/upload-artifact@", "actions/cache/restore@", "actions/cache/save@"]) {
    const name = marker.slice(0, -1);
    const start = action.indexOf(marker);
    assert.ok(start >= 0, `action.yml lost its ${name} step`);
    const pin = action.slice(start + marker.length).split(" ")[0].trim();
    assert.equal(pin.length, 40, `${name} must be pinned to a 40-character SHA, found "${pin}"`);
    assert.ok(
      [...pin].every((character) => "0123456789abcdef".includes(character)),
      `${name} pin "${pin}" is not hexadecimal`,
    );
  }
});

// The parse store must be restored before the CLI runs and saved after it, from
// the scan root the `path` input names — never the repository root — under a
// per-run key whose prefix fallback lets each run restore the newest
// same-version store and save its own successor. [ACTION-CACHE]
check("the parse store is cached around the run, keyed per version and run", () => {
  const key = "key: deslop-${{ steps.resolve.outputs.version }}-${{ runner.os }}-${{ github.run_id }}";
  assert.equal(action.split(key).length - 1, 2, "restore and save must share the exact per-run key");
  assert.ok(
    action.includes("deslop-${{ steps.resolve.outputs.version }}-${{ runner.os }}-\n"),
    "restore-keys must fall back to the version+OS prefix so a new run restores the newest store",
  );
  assert.equal(
    action.split("path: ${{ inputs.path }}/.deslop/cache").length - 1,
    2,
    "restore and save must target .deslop/cache under the scan root, not the repository root",
  );
  const restoreAt = action.indexOf("actions/cache/restore@");
  const runAt = action.indexOf('deslop "${args[@]}"');
  assert.ok(restoreAt >= 0 && restoreAt < runAt, "the store must be restored before deslop runs");
  assert.ok(action.indexOf("actions/cache/save@") > runAt, "the store must be saved after deslop runs");
});

check("the cache is opt-out and a storeless run skips the save instead of failing it", () => {
  assert.equal(
    action.split("if: inputs.cache != 'false'").length - 1,
    3,
    "the restore, the existence probe, and the save must all honour the cache input",
  );
  assert.ok(
    action.includes("if: inputs.cache != 'false' && steps.store.outputs.exists == 'true'"),
    "the save must be skipped when no store exists — actions/cache/save fails on a missing path",
  );
  assert.ok(
    action.includes('if [ -d "${SCAN_PATH}/.deslop/cache" ]'),
    "store existence must be probed under the scan root the path input names",
  );
});

// Every other layer passes `version:` explicitly, so this one line is the whole
// derivation path for a Marketplace consumer. Drop it and `resolveVersion` still
// passes every test above while the action silently demands a `version:` input
// from everyone who pinned it by tag. [ACTION-VERSION]
check("the pinned ref reaches the resolver", () => {
  assert.ok(
    action.includes("ACTION_REF: ${{ github.action_ref }}"),
    "the resolve step must pass github.action_ref through env",
  );
  assert.ok(
    action.includes('"${VERSION_INPUT}" "${ACTION_REF}"'),
    "action-resolve-artifact.mjs must receive the version input and the action ref, in that order",
  );
});

// The tag's README is the body of the Marketplace listing, and the tag is what
// `stamp-release-version.mjs` never gets to rewrite — it stamps the build, not
// the commit. So whatever is committed here is the workflow every listing
// visitor copies. A committed version therefore cannot be kept true: v0.30.0
// shipped a listing advertising `@v0.27.0`. This asserts the property that makes
// that impossible rather than merely detectable — no pin names a version at all.
// The site pages resolve one when they are built, after the release exists; the
// README, which GitHub serves raw, names the placeholder. Both are checkable
// offline, on every PR, which the freshness check they replace was not — it
// needed the newest release, so it ran only where the network was up.
// [ACTION-VERSION]
const sanctionedPins = new Set([PIN_PLACEHOLDER, "{{ releases.pin }}"]);

check("no documented pin commits a version", () => {
  const pins = actionPinDocs.flatMap((doc) =>
    readActionPins(readFileSync(doc, "utf8")).map((token) => ({ doc, token })),
  );
  assert.ok(pins.length >= actionPinDocs.length, `every doc in ${actionPinDocs.join(", ")} must show a pin`);
  for (const { doc, token } of pins) {
    // Proven against the resolver the action actually runs, not a second copy of
    // its version rule: a token it refuses to derive a version from is a token
    // that cannot be a stale version.
    assert.throws(
      () => resolveVersion("", token),
      `${doc} pins v${token} — a committed version rots between releases and hands every visitor ` +
        `a workflow installing an older CLI. Use {{ releases.pin }} on a built page, ${PIN_PLACEHOLDER} in the README`,
    );
    assert.ok(
      sanctionedPins.has(token),
      `${doc} pins v${token}, which resolves to nothing — expected ${[...sanctionedPins].join(" or ")}`,
    );
  }
});

// What the rendered pin actually depends on, and the way it fails silently.
//
// Eleventy hands a template the module's *namespace* when a `_data` ESM file
// exports anything besides `default` — it never calls the function, so
// `releases` becomes `{default: fn, ...}` and every consumer reads undefined.
// Nunjucks prints undefined as the empty string, so the documented pin renders
// as a bare `@v` with no version: a snippet that fails outright rather than one
// that merely installs an older CLI. It took the whole releases page down with
// it in the same build, silently — no warning, exit 0. Nothing downstream can
// catch it, because the markdown source is correct and only the render is
// wrong. [ACTION-VERSION-DOCS]
check("the data module backing the rendered pin exports only a default", () => {
  assert.deepEqual(
    Object.keys(releasesData).filter((name) => name !== "default"),
    [],
    "site/src/_data/releases.js must export nothing but `default` — a named export makes Eleventy " +
      "skip the function and render every `{{ releases.* }}` as empty, including the documented pin",
  );
  assert.equal(typeof releasesData.default, "function", "Eleventy calls the default export to build the data");
});

// The README is served raw and cannot resolve an expression; a built page must
// not fall back to the placeholder when it can render the real number.
check("each surface pins the form it can actually resolve", () => {
  const [readme, ...builtPages] = actionPinDocs;
  for (const token of readActionPins(readFileSync(readme, "utf8"))) {
    assert.equal(token, PIN_PLACEHOLDER, `${readme} is served raw by GitHub, so it cannot render ${token}`);
  }
  for (const doc of builtPages) {
    const tokens = readActionPins(readFileSync(doc, "utf8"));
    assert.ok(tokens.length > 0, `${doc} must show a pin`);
    for (const token of tokens) {
      assert.equal(token, "{{ releases.pin }}", `${doc} is built after the release, so it must render the version`);
    }
  }
});

check("the gate re-raises the CLI status rather than swallowing it", () => {
  assert.ok(action.includes('exit "${EXIT_CODE}"'), "the gate step must re-raise the CLI exit code");
  assert.ok(
    action.includes("if: steps.run.outputs.exit-code != '0'"),
    "the gate step must run for every non-zero status",
  );
});

// GitHub injects `-e` into composite `shell: bash` steps, so a bare `deslop`
// invocation would abort the run step on a breach before the status reaches
// GITHUB_OUTPUT — leaving every output empty and skipping the report, upload,
// and gate steps. [ACTION-GATE]
check("a breach cannot abort the run step before the status is captured", () => {
  assert.ok(
    action.includes('deslop "${args[@]}" || status=$?'),
    "the run step must capture the CLI status with an errexit-proof || guard",
  );
  assert.ok(
    action.includes('echo "exit-code=${status}" >> "${GITHUB_OUTPUT}"'),
    "the captured status must be written to GITHUB_OUTPUT",
  );
  assert.ok(
    !action.includes('echo "exit-code=$?"'),
    "the status must come from the guard variable, not $? after a guarded call",
  );
});

// Git Bash's GNU tar parses the `D:` drive prefix of an absolute
// ${RUNNER_TEMP} archive path as a remote host and cannot read the Windows
// `.zip` at all — extraction must cd into the staging directory and use the
// runner's System32 bsdtar on Windows. [ACTION-VERIFY]
check("extraction survives Git Bash tar on Windows", () => {
  assert.ok(
    action.includes('cd "${RUNNER_TEMP}/deslop"'),
    "the install step must extract from inside the staging directory",
  );
  assert.ok(
    action.includes('"$(cygpath -u "${SYSTEMROOT}")/System32/tar.exe" -xf "${ARCHIVE}"'),
    "Windows must extract with the System32 bsdtar, by relative archive name",
  );
  assert.ok(
    !action.includes('tar -xf "${RUNNER_TEMP}'),
    "no tar invocation may pass an absolute drive-letter archive path",
  );
});
