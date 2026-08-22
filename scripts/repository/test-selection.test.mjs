// Test-selection contract. [TEST-SELECTION]
//
// `cargo test`'s `--skip` is a substring match on the *test name*, not a suite
// selector. `make test` used to pass `--skip ollama_ --skip corpus_`, which
// silently dropped every hermetic test whose name merely contained those
// strings — the corpus precision / scope / confidence gate self-tests, the
// mock-Ollama embedding suites, and the unreachable-endpoint fallback tests.
// A gate whose own self-tests never run rots in silence (gh #412).
//
// The intent — "do not clone real repositories during the release gate" — is
// stated at each test instead, as `#[ignore = ".."]` under
// [TEST-SELECTION-SKIP]. That is deliberately the opposite of a filter: a
// filter hides a test from the person reading it, an `#[ignore]` shows them,
// and `skip_policy_contract` holds the stated reason to the policy.
//
// It also keeps the target inside `--all-targets`, which `required-features`
// did not: skipping must cost coverage of a test's *execution*, never of its
// *compilation*. Commit 77bcbaed5 left the corpus suite uncompilable for weeks
// because feature-gating had removed it from every default build.
//
// These tests assert that wiring. The Makefile is read line-exactly and the
// manifests through `cargo metadata`, never by pattern-matching source text.
// Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("../..", import.meta.url));
const makefile = readFileSync(resolve(repoRoot, "Makefile"), "utf8").split("\n");

// The package and dedicated Cargo test target that own the real-repository
// corpus suite.
const CORPUS_PACKAGE = "deslop";
const CORPUS_TARGET = "corpus_repos";
const CORPUS_TEST_PREFIX = "corpus_";
const ANALYZE_TARGET = "_ci-analyze";

// The make variable naming the resource-bounded slice the scheduled corpus
// workflow runs, and how libtest selects the skipped suite: `--ignored`, plus
// the flag that makes a positional filter an exact match, not a substring one.
const CORPUS_SLICE_VARIABLE = "CORPUS_TESTS";
const IGNORED_FLAG = "--ignored";
const EXACT_FLAG = "--exact";

// The exact substrings the release gate used to filter on. Each names hermetic
// tests: the Ollama suites drive an in-process mock server or a deliberately
// dead endpoint, and the `corpus_*` support units parse fixture source, clone
// nothing and reach no network.
const BANNED_NAME_FILTERS = ["ollama_", "corpus_"];

// Recipe lines of a make target — every block declaring it, because a target
// may be declared twice (once to export an environment variable, once for the
// recipe). Everything from each declaration up to the next line starting in
// column 0. Line-exact, no pattern matching.
function recipe(target) {
  const blocks = [];
  makefile.forEach((line, index) => {
    if (!line.startsWith(`${target}:`)) return;
    const rest = makefile.slice(index + 1);
    const end = rest.findIndex((next) => next.length > 0 && !next.startsWith("\t") && !next.startsWith(" "));
    blocks.push((end < 0 ? rest : rest.slice(0, end)).join("\n"));
  });
  assert.ok(blocks.length > 0, `Makefile no longer declares a \`${target}\` target`);
  return blocks.join("\n");
}

// The right-hand side of a make variable, as words. `?=` and `=` both count;
// the name is matched at the start of the line, never as a substring.
function variable(name) {
  const line = makefile.find(
    (entry) => entry.startsWith(`${name} `) || entry.startsWith(`${name}=`),
  );
  assert.ok(line, `Makefile no longer declares \`${name}\``);
  const [, value] = line.split("=");
  return words(value ?? "");
}

// Whitespace-separated words of a recipe, so `--skip` is matched as an
// argument rather than as a substring of prose. Split on the three characters
// make and the shell treat as separators — no pattern matching.
function words(body) {
  return body
    .split("\n")
    .flatMap((line) => line.split("\t"))
    .flatMap((chunk) => chunk.split(" "))
    .filter((word) => word.length > 0);
}

// The crate that owns the corpus gate's own precision / scope / confidence
// self-tests. `--skip corpus_` hid every one of them.
const GATE_SELFTEST_PACKAGE = "deslop-test-support";

// The cargo flag that keeps every target compiled, the flag that selects
// features, and the make variable single-sourcing the feature set the release
// gate compiles with — referenced by `test`, `lint`, `coverage` and the shard
// runner so none of them can drift from the others.
const ALL_TARGETS_FLAG = "--all-targets";
const FEATURES_FLAG = "--features";
const FEATURES_VARIABLE = "_TEST_FEATURES";
const FEATURES_REFERENCE = `$(${FEATURES_VARIABLE})`;

// The commit that feature-gated the corpus target out of every default build.
const UNCOMPILABLE_CORPUS_COMMIT = "77bcbaed5";

// The workspace as cargo itself resolves it — the authoritative parse of the
// manifests, so this contract cannot drift from what cargo actually builds.
function workspacePackages() {
  const raw = execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
    cwd: repoRoot,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  return JSON.parse(raw).packages;
}

function packageNamed(name) {
  const found = workspacePackages().find((entry) => entry.name === name);
  assert.ok(found, `the workspace no longer contains a \`${name}\` package`);
  return found;
}

function corpusPackage() {
  return packageNamed(CORPUS_PACKAGE);
}

// The value a recipe hands cargo's feature flag, as one word — the make
// variable reference itself, not its expansion, so a second literal copy of
// the list is a visible difference rather than a coincidental match.
function featureSet(target) {
  const args = words(recipe(target));
  const flag = args.indexOf(FEATURES_FLAG);
  assert.ok(flag >= 0, `\`make ${target}\` no longer passes \`${FEATURES_FLAG}\` at all`);
  return args[flag + 1];
}

test("[TEST-SELECTION] the release gate selects no test by name", () => {
  const args = words(recipe("test"));
  assert.ok(
    !args.includes("--skip"),
    "`make test` must not pass --skip: cargo matches it as a substring of the test name, so it " +
      "drops hermetic tests that merely mention a service (gh #412). Gate expensive suites with " +
      "a Cargo test target + required-features instead.",
  );
  assert.ok(
    !args.includes("--exclude"),
    "`make test` must not exclude a workspace crate: the corpus gate's own self-tests live in " +
      "deslop-test-support and must run in the release gate",
  );
  assert.ok(
    args.includes("--workspace"),
    "`make test` must run the whole workspace so deslop-test-support's gate self-tests execute",
  );
  assert.ok(
    args.includes("--all-targets"),
    "`make test` must run every target, not just the default set",
  );
  for (const filtered of BANNED_NAME_FILTERS) {
    assert.ok(
      !args.some((word) => word.includes(filtered)),
      `\`make test\` must not name \`${filtered}\` — name-substring selection is what hid the ` +
        "gate's own self-tests (gh #412)",
    );
  }
});

test("[TEST-SELECTION-SKIP] no test target is gated out of the default build", () => {
  const gated = corpusPackage()
    .targets.filter((entry) => entry.kind.includes("test"))
    .filter((entry) => (entry["required-features"] ?? []).length > 0)
    .map((entry) => entry.name)
    .sort();
  assert.deepEqual(
    gated,
    [],
    `no \`${CORPUS_PACKAGE}\` test target may carry required-features. A gated target leaves ` +
      "`--all-targets` entirely, so `make test` and `make lint` stop compiling it and a refactor " +
      "elsewhere can leave it uncompilable with nothing to notice until someone runs " +
      "`make test-corpus`. Commit 77bcbaed5 deleted two constants the corpus suite still read " +
      "and did exactly that. Skipping is `#[ignore]`, which keeps the target built and linted.",
  );
  assert.deepEqual(
    Object.keys(corpusPackage().features).sort(),
    [],
    `\`${CORPUS_PACKAGE}\` must declare no features: a feature is how the corpus target left ` +
      "the default build in the first place",
  );
});

test("[TEST-SELECTION-SKIP] the corpus target is still declared and still built", () => {
  const target = corpusPackage().targets.find((entry) => entry.name === CORPUS_TARGET);
  assert.ok(target, `\`${CORPUS_PACKAGE}\` must still declare the \`${CORPUS_TARGET}\` test target`);
  assert.ok(
    target.test,
    `the \`${CORPUS_TARGET}\` target must keep \`test = true\`, or its skips become invisible ` +
      "to `skip_policy_contract` and the suite silently stops existing",
  );
});

test("[TEST-SELECTION-SKIP] the corpus targets select by --ignored, never by name", () => {
  for (const target of ["test-corpus", "test-corpus-ci"]) {
    const args = words(recipe(target));
    assert.ok(
      args.includes(IGNORED_FLAG),
      `\`make ${target}\` must pass ${IGNORED_FLAG}: the corpus tests are skipped at their ` +
        "declaration, so nothing else selects them and the target would run zero tests green",
    );
    assert.ok(
      args.includes("--test") && args.includes(CORPUS_TARGET),
      `\`make ${target}\` must select Cargo's ${CORPUS_TARGET} target explicitly, or the recipe ` +
        "runs a different test binary and can report green without scanning a repository.",
    );
    assert.ok(
      !args.includes("--skip"),
      `\`make ${target}\` must not pass --skip — substring selection is the defect (gh #412)`,
    );
  }
});

test("[TEST-SELECTION-SKIP] the scheduled slice matches test names exactly", () => {
  const args = words(recipe("test-corpus-ci"));
  assert.ok(
    args.includes(EXACT_FLAG),
    "`make test-corpus-ci` runs a resource-bounded slice named in CORPUS_TESTS, so it must pass " +
      `${EXACT_FLAG}. Without it libtest matches each name as a substring: renaming a test makes ` +
      "the filter select nothing, and a run that executes zero tests reports green (gh #412).",
  );
  const slice = variable(CORPUS_SLICE_VARIABLE);
  assert.ok(
    slice.length > 0,
    `${CORPUS_SLICE_VARIABLE} must still name the scheduled slice, or the corpus workflow runs ` +
      "nothing and the summary reports a green run over zero repositories",
  );
  for (const name of slice) {
    assert.ok(
      name.startsWith(CORPUS_TEST_PREFIX),
      `${CORPUS_SLICE_VARIABLE} names \`${name}\`, which ${EXACT_FLAG} resolves to nothing: ` +
        `inside the ${CORPUS_TARGET} binary every corpus test starts with ` +
        `\`${CORPUS_TEST_PREFIX}\`. The scheduled run would execute zero repositories and report green.`,
    );
  }
});

test("[TEST-SELECTION] the gate's own self-tests are inside the gate's workspace", () => {
  const pkg = packageNamed(GATE_SELFTEST_PACKAGE);
  const lib = pkg.targets.find((entry) => entry.kind.includes("lib"));
  assert.ok(lib, `\`${GATE_SELFTEST_PACKAGE}\` must still expose a lib target`);
  assert.equal(
    lib.test,
    true,
    `\`${GATE_SELFTEST_PACKAGE}\` must keep unit tests enabled on its lib target: it holds the ` +
      "corpus precision, scope and confidence contracts — the tests that decide whether a corpus " +
      "result means anything. `--skip corpus_` already hid them once (gh #412); `test = false` " +
      `would hide them again, and \`make test-corpus\` runs only \`-p ${CORPUS_PACKAGE} --test ` +
      `${CORPUS_TARGET}\`.`,
  );
  assert.deepEqual(
    Object.keys(pkg.features).sort(),
    [],
    `\`${GATE_SELFTEST_PACKAGE}\` must declare no features: a feature is a way to gate those ` +
      "self-tests out of the release gate again",
  );
});

test("[TEST-SELECTION-SKIP] an ignored test is still compiled and linted every run", () => {
  assert.ok(
    words(recipe(ANALYZE_TARGET)).includes(ALL_TARGETS_FLAG),
    `\`make ${ANALYZE_TARGET}\` must lint every target. ` +
      "`#[ignore]` costs coverage of a test's *execution*, " +
      "never of its *compilation*: the target stays in `--all-targets` and clippy still reads it.",
  );
  assert.ok(
    words(recipe("test")).includes(ALL_TARGETS_FLAG),
    "`make test` must build every target, so a skipped suite still fails the gate when it stops " +
      `compiling — the failure mode of commit ${UNCOMPILABLE_CORPUS_COMMIT}`,
  );
  assert.equal(
    featureSet(ANALYZE_TARGET),
    featureSet("test"),
    `\`make ${ANALYZE_TARGET}\` must compile the feature set \`make test\` runs. Clippy only reads the code ` +
      "the features it was given compile, so a feature the release gate enables and lint omits " +
      "is a module nobody ever lints — `deslop-lsp/profiling` was exactly that module.",
  );
  assert.equal(
    featureSet(ANALYZE_TARGET),
    FEATURES_REFERENCE,
    `both must reach it through \`${FEATURES_REFERENCE}\`: two literal copies of the same list ` +
      "agree until the day one of them is edited",
  );
  assert.ok(
    variable(FEATURES_VARIABLE).length > 0,
    `\`${FEATURES_VARIABLE}\` must name at least one feature, or every recipe referencing it ` +
      "compiles the default build while claiming to compile the matrix",
  );
});

test("[TEST-SELECTION-SKIP] no feature can gate the corpus suite out of a build", () => {
  const pkg = corpusPackage();
  assert.deepEqual(
    Object.keys(pkg.features).sort(),
    [],
    `\`${CORPUS_PACKAGE}\` must declare no features. \`required-features\` may only name a ` +
      "feature of its own package, so a package that declares none has no way to remove a " +
      "target from the default build — which is what left the corpus suite uncompilable for " +
      `weeks in commit ${UNCOMPILABLE_CORPUS_COMMIT}. Read from the manifest cargo itself ` +
      "resolves, so it holds whatever flags a recipe happens to pass.",
  );
  for (const target of pkg.targets) {
    assert.deepEqual(
      target["required-features"] ?? target.required_features ?? [],
      [],
      `\`${CORPUS_PACKAGE}\`'s \`${target.name}\` target must carry no required-features: ` +
        `that is the exact mechanism that dropped \`${CORPUS_TARGET}\` out of ` +
        `\`${ALL_TARGETS_FLAG}\``,
    );
  }
});
