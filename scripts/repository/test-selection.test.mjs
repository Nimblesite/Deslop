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
// expressed structurally instead: the expensive suite is its own Cargo test
// target behind `required-features`, so cargo will not build it unless asked,
// and nothing anywhere is selected by name. These tests assert that wiring.
// The Makefile is read line-exactly and the manifest through `cargo metadata`,
// never by pattern-matching source text. Run with `node --test`.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("../..", import.meta.url));
const makefile = readFileSync(resolve(repoRoot, "Makefile"), "utf8").split("\n");

// The Cargo feature that opts a build in to the real-repository corpus suite,
// the package that owns it, and the test target it gates.
const CORPUS_FEATURE = "corpus-repos";
const CORPUS_PACKAGE = "deslop";
const CORPUS_TARGET = "corpus_repos";

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

test("[TEST-SELECTION] the corpus suite is gated by a Cargo feature, not a name", () => {
  const pkg = corpusPackage();
  assert.deepEqual(
    pkg.features[CORPUS_FEATURE],
    [],
    `\`${CORPUS_PACKAGE}\` must declare the \`${CORPUS_FEATURE}\` feature that opts a build in ` +
      "to the real-repository corpus suite",
  );
  const target = pkg.targets.find((entry) => entry.name === CORPUS_TARGET);
  assert.ok(target, `\`${CORPUS_PACKAGE}\` must still declare the \`${CORPUS_TARGET}\` test target`);
  assert.deepEqual(
    target["required-features"],
    [CORPUS_FEATURE],
    `the \`${CORPUS_TARGET}\` target must carry required-features = ["${CORPUS_FEATURE}"] — ` +
      "without it the expensive clone-and-scan suite runs in the release gate",
  );
});

test("[TEST-SELECTION] only the expensive suite is gated, never a cheap namesake", () => {
  const gated = corpusPackage()
    .targets.filter((entry) => entry.kind.includes("test"))
    .filter((entry) => (entry["required-features"] ?? []).length > 0)
    .map((entry) => entry.name)
    .sort();
  assert.deepEqual(
    gated,
    [CORPUS_TARGET],
    `exactly one \`${CORPUS_PACKAGE}\` test target may be feature-gated: the clone-and-scan ` +
      "corpus suite. Gating is by cost, never by a name that happens to read like it — " +
      "`corpus_manifest_contract` reads the pinned manifests off disk and must run in the gate.",
  );
});

test("[TEST-SELECTION] the corpus targets ask for the feature they need", () => {
  for (const target of ["test-corpus", "test-corpus-ci"]) {
    const args = words(recipe(target));
    assert.ok(
      args.includes("--features") && args.includes(CORPUS_FEATURE),
      `\`make ${target}\` must pass --features ${CORPUS_FEATURE}, or cargo skips the gated ` +
        "target and the corpus gate passes by running nothing",
    );
    assert.ok(
      args.includes("--test") && args.includes(CORPUS_TARGET),
      `\`make ${target}\` must select the ${CORPUS_TARGET} target explicitly`,
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
      "would hide them again, and `make test-corpus` runs only `-p deslop --test corpus_repos`.",
  );
  assert.deepEqual(
    Object.keys(pkg.features).sort(),
    [],
    `\`${GATE_SELFTEST_PACKAGE}\` must declare no features: a feature is a way to gate those ` +
      "self-tests out of the release gate again",
  );
});

test("[TEST-SELECTION] the gated corpus target is still compiled every run", () => {
  const args = words(recipe("lint"));
  assert.ok(
    args.includes(`--features`) && args.includes(`${CORPUS_PACKAGE}/${CORPUS_FEATURE}`),
    `\`make lint\` must pass --features ${CORPUS_PACKAGE}/${CORPUS_FEATURE} to clippy. Gating ` +
      "the corpus suite out of `make test` must cost coverage of its *execution*, never of its " +
      "*compilation*: once it stops being built by default, a refactor elsewhere can leave it " +
      "uncompilable and nothing notices until someone runs `make test-corpus`. Commit 77bcbaed5 " +
      "deleted two constants the suite still read, and the corpus gate could not build at all.",
  );
  assert.ok(
    args.includes("--all-targets"),
    "`make lint` must lint every target, or a test file's own defects go unlinted",
  );
});
