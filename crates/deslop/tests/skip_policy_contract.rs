//! [TEST-SELECTION-SKIP] The policy every `#[ignore]` in this workspace obeys.
//!
//! `make test` selects no test by name. `cargo test --skip` matches a
//! substring of the *test name*, so `--skip corpus_` dropped the corpus
//! gate's own self-tests and `--skip ollama_` dropped every hermetic
//! mock-server suite, while the gate reported green (gh #412).
//!
//! A test may still be excluded, but only at its own declaration and only
//! when it says why. Those are two different failures: a filter hides a test
//! from the person reading it, and an `#[ignore]` shows them. This gate makes
//! the second one carry its evidence.
//!
//! A skip must state a category, a tracking issue, a spec id, and a plan
//! document that names that issue — the four things a reader needs to decide
//! whether the skip is still earned. The curated set below is a ratchet: a
//! new `#[ignore]` fails this gate until someone adds it here deliberately,
//! and a skip whose fix has landed fails it until someone deletes it.
//!
//! The scan is an AST walk ([TEST-SELECTION-SKIP] in `deslop-test-support`),
//! never a text match, so a comment or string literal mentioning `ignore` is
//! not a skip and a skip wrapped across five lines still is one.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    process::Command,
};

use anyhow::{anyhow, Context, Result};
use deslop_test_support::{
    corpus::repo_root,
    skip_contract::{
        bracketed_ids, breaches, registry_diff, Breach, PolicyContext, CATEGORIES, PLAN_PREFIX,
    },
    skip_policy::{conditional_tests, feature_liveness_pins, ignored_tests},
};
use serde_json::Value;

/// The specification that documents this policy, and the id it is filed
/// under. Code, specs, and tests must agree, so the categories this gate
/// enforces are the categories that document defines.
const POLICY_SPEC: &str = "docs/specs/release.md";
const SPEC_DIRECTORY: &str = "docs/specs";

/// The Makefile, the variable naming the resource-bounded slice the scheduled
/// corpus workflow runs, and the suite those names must resolve inside.
const MAKEFILE: &str = "Makefile";
const CORPUS_SLICE_VARIABLE: &str = "CORPUS_TESTS";
const CORPUS_SUITE: &str = "crates/deslop/tests/corpus_repos.rs";

/// The package whose corpus targets the Makefile invokes, and the recipe
/// tokens that identify one of those invocations.
const DESLOP_PACKAGE: &str = "deslop";
const CARGO_TEST: &str = "cargo test";
const PACKAGE_FLAG: &str = "-p deslop";
const TEST_TARGET_FLAG: &str = "--test";
/// The `cargo metadata` target kind that names an integration-test binary.
const TEST_TARGET_KIND: &str = "test";

/// Every test allowed not to run, with the issue that owns its return.
///
/// Ordered by file then test name, matching `ignored_tests()`. The eleven
/// `corpus_repos` entries and `perf_sample` are the real-repository gate
/// (gh #422, blocked on the memory work in #166); `corpus_manifest_contract`
/// is the curation those
/// same two oversized repositories block (gh #426); the two gh #369 entries
/// are red on purpose against unfinished fusion and embedding behaviour. The
/// two gh #432 entries are the fused-score follow-ups' own accuracy
/// pins, skipped in flight per `docs/plans/fused-score-followups.md` — each
/// returns when its issue lands.
///
/// Those counts are prose, and prose drifts. [`SKIPS_PER_ISSUE`] is what
/// stops it drifting silently.
const CURATED_SKIPS: [(&str, &str, u32); 17] = [
    (
        "crates/deslop-lsp/tests/lsp_embedding_determinism.rs",
        "lsp_embedding_refresh_is_bounded_and_reproducible",
        369,
    ),
    (
        "crates/deslop/tests/corpus_manifest_contract.rs",
        "every_manifest_curates_a_non_vacuous_scan_scope",
        426,
    ),
    (
        "crates/deslop/tests/corpus_repos.rs",
        "corpus_determinism_jellyfin_csharp",
        422,
    ),
    (
        "crates/deslop/tests/corpus_repos.rs",
        "corpus_determinism_nest_typescript",
        422,
    ),
    (
        "crates/deslop/tests/corpus_repos.rs",
        "corpus_django_python",
        422,
    ),
    (
        "crates/deslop/tests/corpus_repos.rs",
        "corpus_flutter_dart",
        422,
    ),
    ("crates/deslop/tests/corpus_repos.rs", "corpus_fsharp", 422),
    ("crates/deslop/tests/corpus_repos.rs", "corpus_hugo_go", 422),
    (
        "crates/deslop/tests/corpus_repos.rs",
        "corpus_jellyfin_csharp",
        422,
    ),
    (
        "crates/deslop/tests/corpus_repos.rs",
        "corpus_laravel_php",
        422,
    ),
    (
        "crates/deslop/tests/corpus_repos.rs",
        "corpus_nest_typescript",
        422,
    ),
    (
        "crates/deslop/tests/corpus_repos.rs",
        "corpus_react_javascript",
        422,
    ),
    (
        "crates/deslop/tests/corpus_repos.rs",
        "corpus_tokio_rust",
        422,
    ),
    (
        "crates/deslop/tests/issue_343_sum_clamp_saturation.rs",
        "mid_band_cluster_confidence_never_exceeds_its_strongest_axis",
        369,
    ),
    (
        "crates/deslop/tests/operator_drift_is_not_duplication.rs",
        "an_operator_only_difference_never_reaches_the_act_now_line",
        432,
    ),
    (
        "crates/deslop/tests/operator_drift_is_not_duplication.rs",
        "the_real_clone_outranks_every_operator_family",
        432,
    ),
    (
        "crates/deslop/tests/perf_sample.rs",
        "perf_sample_bounded_scan",
        422,
    ),
];

/// How many curated skips each tracking issue owns.
///
/// The compiler checks the declared length of [`CURATED_SKIPS`]; the
/// breakdown above it was only a sentence, and it went stale — twenty-two gh
/// #432–#435 entries when the registry held nine across #432–#434 and none
/// for #435. That is a wrong answer to the question a reader is actually
/// asking: which plan still owns this block of silence, and how much of it.
const SKIPS_PER_ISSUE: [(u32, usize); 4] = [(369, 2), (422, 12), (426, 1), (432, 2)];

/// How many skips each issue owns, counted from the registry itself.
fn skips_by_issue() -> BTreeMap<u32, usize> {
    CURATED_SKIPS
        .iter()
        .map(|(_, _, issue)| (*issue, skips_owned_by(*issue)))
        .collect()
}

/// The registry rows credited to one tracking issue.
fn skips_owned_by(issue: u32) -> usize {
    CURATED_SKIPS
        .iter()
        .filter(|(_, _, owner)| *owner == issue)
        .count()
}

/// [TEST-SELECTION-SKIP] The registry agrees with what it says about itself.
/// A per-issue count is the one number the array's declared length cannot
/// check, which is why it is the one that drifted.
#[test]
fn every_tracking_issue_owns_the_number_of_skips_the_registry_declares() {
    let counted = skips_by_issue();
    let declared: BTreeMap<u32, usize> = SKIPS_PER_ISSUE.iter().copied().collect();
    assert_eq!(
        counted, declared,
        "SKIPS_PER_ISSUE disagrees with CURATED_SKIPS. A skip was added, deleted, \
         or changed hands: correct the breakdown, never the tally it describes."
    );
    assert_eq!(
        declared.len(),
        SKIPS_PER_ISSUE.len(),
        "two rows of SKIPS_PER_ISSUE name the same issue, so one of them was never read"
    );
    assert_eq!(
        CURATED_SKIPS.len(),
        CURATED_SKIPS
            .iter()
            .filter(|(_, _, issue)| declared.contains_key(issue))
            .count(),
        "SKIPS_PER_ISSUE omits a tracking issue that CURATED_SKIPS credits"
    );
    assert!(
        !counted.values().any(|owned| *owned == 0),
        "an issue is credited with no skips at all, so it owns nothing and should be deleted"
    );
}

/// `(file, test)` of every curated skip, for set comparison.
fn curated() -> Vec<(String, String)> {
    CURATED_SKIPS
        .iter()
        .map(|(file, test, _)| ((*file).to_owned(), (*test).to_owned()))
        .collect()
}

/// Reads a workspace-relative file.
fn read(relative: &str) -> Result<String> {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).with_context(|| format!("unreadable: {}", path.display()))
}

/// Every markdown file in a workspace-relative directory, keyed by its own
/// workspace-relative path.
fn markdown_in(directory: &str) -> Result<BTreeMap<String, String>> {
    let absolute = repo_root().join(directory);
    let mut found = BTreeMap::new();
    for entry in fs::read_dir(&absolute).with_context(|| format!("{directory} must be readable"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "md") {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            let body = fs::read_to_string(&path)
                .with_context(|| format!("unreadable: {}", path.display()))?;
            let previous = found.insert(format!("{directory}{name}"), body);
            assert!(previous.is_none(), "{directory}{name} listed twice");
        }
    }
    Ok(found)
}

/// The policy's view of this tree: the spec ids some specification declares,
/// and the body of every plan a skip could cite.
fn policy_context() -> Result<PolicyContext> {
    let mut declared_spec_ids = BTreeSet::new();
    for body in markdown_in(&format!("{SPEC_DIRECTORY}/"))?.values() {
        declared_spec_ids.extend(bracketed_ids(body));
    }
    assert!(
        declared_spec_ids.len() > 50,
        "only {} spec ids found under {SPEC_DIRECTORY}; the specifications did not load, so \
         every cross-reference check below would pass or fail for the wrong reason",
        declared_spec_ids.len()
    );
    Ok(PolicyContext {
        declared_spec_ids,
        plans: markdown_in(PLAN_PREFIX)?,
    })
}

#[test]
fn the_ignored_tests_in_the_tree_are_exactly_the_curated_set() -> Result<()> {
    let found = ignored_tests()?;
    let present: Vec<(String, String)> = found
        .iter()
        .map(|skip| (skip.file.clone(), skip.test.clone()))
        .collect();
    assert_registry_matches(&present);
    assert_eq!(
        found.len(),
        CURATED_SKIPS.len(),
        "a file declares the same test name twice, so the curated set no longer identifies it"
    );
    assert_eq!(
        present,
        curated(),
        "CURATED_SKIPS must stay ordered by file then test name, the order its own \
         doc comment promises and `ignored_tests()` returns. A registry a reader \
         cannot scan against the tree hides a misfiled row in plain sight."
    );
    Ok(())
}

/// Both directions of the registry, which fail for opposite reasons.
fn assert_registry_matches(present: &[(String, String)]) {
    let (unregistered, stale) = registry_diff(present, &curated());
    assert!(
        unregistered.is_empty(),
        "{unregistered:?} carry `#[ignore]` and are not in CURATED_SKIPS. Adding a skip is a \
         deliberate act: give it a tracking issue, a plan, and an entry here."
    );
    assert!(
        stale.is_empty(),
        "{stale:?} are registered as skipped and no longer carry `#[ignore]`. A skip that \
         outlives its defect reads as coverage nobody has — delete the entry."
    );
}

/// The tracking issue [`CURATED_SKIPS`] credits one `(file, test)` to.
///
/// Looked up by name, never by position. Pairing the two lists with `zip`
/// held only while they agreed element for element, so the first divergence
/// silently shifted every later row onto the wrong issue: a stale #107 entry
/// made this gate demand `GH #434` of `report_golden.rs`, whose skip
/// correctly cites #432. That reports a real breach against an innocent test
/// and, in the other direction, judges a mis-filed skip against whichever
/// issue happened to line up with it.
fn registered_issue(file: &str, test: &str) -> Option<u32> {
    CURATED_SKIPS
        .iter()
        .find(|(curated_file, curated_test, _)| *curated_file == file && *curated_test == test)
        .map(|(_, _, issue)| *issue)
}

#[test]
fn every_skip_in_the_tree_satisfies_the_stated_policy() -> Result<()> {
    let context = policy_context()?;
    for skip in &ignored_tests()? {
        let issue = registered_issue(&skip.file, &skip.test).ok_or_else(|| {
            anyhow!(
                "{}::{} carries `#[ignore]` and CURATED_SKIPS credits it to no issue",
                skip.file,
                skip.test
            )
        })?;
        let breached = breaches(skip, issue, &context);
        assert!(
            breached.is_empty(),
            "{}::{} breaches [TEST-SELECTION-SKIP]:\n{}\nReason: {}",
            skip.file,
            skip.test,
            explain(&breached),
            skip.reason
        );
    }
    Ok(())
}

/// The breaches as a bulleted list, so one run names every missing part.
fn explain(breached: &[Breach]) -> String {
    breached
        .iter()
        .map(|breach| format!("  - {}", breach.explain()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_policy_this_gate_enforces_is_the_one_the_specification_writes_down() -> Result<()> {
    let spec = read(POLICY_SPEC)?;
    let ids = bracketed_ids(&spec);
    for category in CATEGORIES {
        let bare = category.trim_start_matches('[').trim_end_matches(']');
        assert!(
            ids.iter().any(|id| id == bare),
            "{POLICY_SPEC} does not declare {category}. Code, specs and tests must agree: this \
             gate would be rejecting skips on a rule nobody wrote down."
        );
    }
    Ok(())
}

/// The test names `CORPUS_TESTS` hands to the scheduled corpus workflow.
fn scheduled_slice() -> Result<Vec<String>> {
    let makefile = read(MAKEFILE)?;
    let declaration = makefile
        .lines()
        .find(|line| line.starts_with(CORPUS_SLICE_VARIABLE))
        .ok_or_else(|| anyhow!("{MAKEFILE} no longer declares {CORPUS_SLICE_VARIABLE}"))?;
    Ok(declaration
        .split('=')
        .nth(1)
        .unwrap_or_default()
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect())
}

/// Test-target binaries cargo actually builds for the `deslop` package.
///
/// Read from `cargo metadata` so the manifest is parsed by cargo itself
/// rather than by matching the text of a structured document.
fn declared_test_targets() -> Result<BTreeSet<String>> {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(repo_root())
        .output()
        .context("failed to run `cargo metadata`")?;
    anyhow::ensure!(
        output.status.success(),
        "`cargo metadata` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout)
        .context("`cargo metadata` did not emit valid JSON")?;
    Ok(test_target_names(&metadata))
}

/// The `test` target names declared by the `deslop` package in `metadata`.
fn test_target_names(metadata: &Value) -> BTreeSet<String> {
    metadata
        .get("packages")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|package| package.get("name").and_then(Value::as_str) == Some(DESLOP_PACKAGE))
        .filter_map(|package| package.get("targets").and_then(Value::as_array))
        .flatten()
        .filter(|target| target_is_a_test(target))
        .filter_map(|target| target.get("name").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

/// True when a `cargo metadata` target is an integration-test binary.
fn target_is_a_test(target: &Value) -> bool {
    target
        .get("kind")
        .and_then(Value::as_array)
        .is_some_and(|kinds| {
            kinds
                .iter()
                .any(|kind| kind.as_str() == Some(TEST_TARGET_KIND))
        })
}

/// Every `--test <target>` the Makefile hands cargo for the `deslop` package.
fn makefile_test_targets() -> Result<BTreeSet<String>> {
    let makefile = read(MAKEFILE)?;
    Ok(makefile
        .lines()
        .filter(|line| line.contains(CARGO_TEST) && line.contains(PACKAGE_FLAG))
        .filter_map(target_after_test_flag)
        .collect())
}

/// The token following `--test` on one recipe line.
fn target_after_test_flag(line: &str) -> Option<String> {
    let mut tokens = line.split_whitespace();
    while let Some(token) = tokens.next() {
        if token == TEST_TARGET_FLAG {
            return tokens.next().map(ToOwned::to_owned);
        }
    }
    None
}

/// [TEST-ONE-BINARY] `autotests = false` leaves `crates/deslop/Cargo.toml`
/// declaring exactly one `[[test]]` binary, so a Makefile naming any other
/// target dies on `no test target named ...` before it clones a repository.
/// The corpus gate then fails for a reason that has nothing to do with the
/// corpus, which is how it came to run zero repositories (gh #347).
#[test]
fn every_corpus_make_target_names_a_cargo_test_target_that_exists() -> Result<()> {
    let declared = declared_test_targets()?;
    let invoked = makefile_test_targets()?;
    assert!(
        !invoked.is_empty(),
        "{MAKEFILE} no longer invokes `{CARGO_TEST} {PACKAGE_FLAG} {TEST_TARGET_FLAG} ...`, so \
         nothing here pins the corpus gate to a target that exists"
    );
    for target in &invoked {
        assert!(
            declared.contains(target),
            "{MAKEFILE}: `{TEST_TARGET_FLAG} {target}` names no cargo test target in the \
             `{DESLOP_PACKAGE}` package, so the corpus gate exits before it scans anything. \
             The package declares {declared:?}."
        );
    }
    Ok(())
}

#[test]
fn the_scheduled_corpus_slice_names_tests_that_still_exist() -> Result<()> {
    let suite: Vec<String> = ignored_tests()?
        .into_iter()
        .filter(|skip| skip.file == CORPUS_SUITE)
        .map(|skip| skip.test)
        .collect();
    let slice = scheduled_slice()?;
    assert!(
        !slice.is_empty(),
        "{MAKEFILE}: {CORPUS_SLICE_VARIABLE} names no test, so the scheduled corpus workflow \
         runs nothing and reports green over zero repositories"
    );
    assert_slice_resolves(&slice, &suite);
    Ok(())
}

/// Every name the scheduled slice selects must be a test the suite declares,
/// spelled the way `--exact` actually matches it: the bare function name.
///
/// This assertion used to qualify both sides as `<module>::<test>`, on the
/// premise that every test answers inside one `suite` binary. That premise is
/// false for this suite — `crates/deslop/Cargo.toml` declares `corpus_repos`
/// as its own `[[test]]` target, so its file is that binary's crate root and
/// its tests are top level. Qualifying both sides made the comparison agree
/// with itself and pass while the Makefile named tests that resolve to
/// nothing: `--exact` selected zero tests and the run reported green over
/// zero repositories. That is gh #412 exactly, reintroduced by the very gate
/// written to prevent it, which is why the names are compared unqualified
/// here and why the target layout is asserted rather than assumed.
fn assert_slice_resolves(slice: &[String], suite: &[String]) {
    for name in slice {
        assert!(
            suite.contains(name),
            "{MAKEFILE}: {CORPUS_SLICE_VARIABLE} selects `{name}`, which `--exact` resolves to \
             no test in {CORPUS_SUITE}. The scheduled run would execute zero tests and report \
             green. The suite declares {suite:?}."
        );
    }
}

/// Every `#[cfg]`-gated test, with the platform or feature that gates it.
///
/// [TEST-SELECTION-SKIP] says `#[ignore]` is the only mechanism that may
/// keep a test out of `make test`, because a skip has to be visible to the
/// person reading the test and to the gate that reads them all. A `#[cfg]`
/// is neither: it removes the test from the build, so it costs coverage of
/// the test's *compilation* as well as its execution — the exact failure
/// `77bcbaed5` caused and the spec cites.
///
/// The seven `unix` entries are platform scope: they compile and run on the
/// platform CI uses, and the Windows job covers the other side. The
/// `profiling` entry is different in kind and is tracked, not blessed —
/// see [`FEATURE_GATED_TEST`].
const CONDITIONAL_TESTS: [(&str, &str); 8] = [
    (
        "crates/deslop-lsp/tests/observability_heartbeat.rs",
        "profile_dir_writes_non_empty_firefox_profile_on_shutdown",
    ),
    (
        "crates/deslop-lsp/tests/state_file_and_ipc.rs",
        "current_state_file_loads_and_incremental_updates_continue",
    ),
    (
        "crates/deslop-lsp/tests/state_file_and_ipc.rs",
        "ipc_socket_handles_find_similar_request",
    ),
    (
        "crates/deslop-lsp/tests/state_file_and_ipc.rs",
        "ipc_socket_handles_list_models_request",
    ),
    (
        "crates/deslop-lsp/tests/state_file_and_ipc.rs",
        "ipc_socket_handles_refresh_report_request",
    ),
    (
        "crates/deslop-lsp/tests/state_file_and_ipc.rs",
        "ipc_socket_returns_method_not_found_for_unknown_method",
    ),
    (
        "crates/deslop-lsp/tests/state_file_and_ipc.rs",
        "state_file_updated_after_file_change",
    ),
    (
        "crates/deslop/tests/cli/cache_and_debug.rs",
        "cache_write_failure_is_degraded_not_fatal",
    ),
];

/// The one conditional test whose predicate names a cargo feature.
///
/// `deslop-lsp` declares `default = []`, so `profiling` is off unless a
/// command opts in — and for as long as nothing did, this test was not
/// skipped but *absent*: never compiled, never linted, never covered, and
/// never reported as missing. `--all-features` linting proved it, surfacing
/// two `missing_docs` violations in `crates/deslop-lsp/src/profiling.rs`
/// that the ordinary lint had never seen.
///
/// The Makefile's `_TEST_FEATURES` now enables it, and
/// [`FEATURE_LIVENESS_PIN`] is what keeps that true.
const FEATURE_GATED_TEST: (&str, &str, &str) = (
    "crates/deslop-lsp/tests/observability_heartbeat.rs",
    "profile_dir_writes_non_empty_firefox_profile_on_shutdown",
    "profiling",
);

/// The unconditional test that proves the feature above is enabled by
/// whatever command is running this suite.
///
/// A static scan cannot answer that question: whether `feature =
/// "profiling"` holds is decided by the command, not by the source, and
/// reading the command back out of the Makefile would only move the
/// guess. An unconditional `#[test]` asserting `cfg!(feature = "..")` is
/// compiled into the same target as the gated test and fails in any run
/// where the feature is off, so it answers the question by being run.
const FEATURE_LIVENESS_PIN: &str = "crates/deslop-lsp/tests/observability_heartbeat.rs";

/// No test is gated by a `#[cfg]` that nobody has accounted for.
///
/// [TEST-SELECTION-SKIP] Adding one fails here until someone adds it
/// deliberately — the same bargain [`CURATED_SKIPS`] strikes for
/// `#[ignore]`, and for the same reason: a skip nobody can see is a test
/// that protects nothing while reporting that it does.
#[test]
fn every_cfg_gated_test_is_accounted_for() -> Result<()> {
    let found: BTreeSet<(String, String)> = conditional_tests()?
        .into_iter()
        .map(|(file, test, _)| (file, test))
        .collect();
    let curated: BTreeSet<(String, String)> = CONDITIONAL_TESTS
        .iter()
        .map(|(file, test)| ((*file).to_owned(), (*test).to_owned()))
        .collect();
    assert_eq!(
        found.difference(&curated).collect::<Vec<_>>(),
        Vec::<&(String, String)>::new(),
        "a `#[cfg]` on a test keeps it out of `make test` without the \
         [TEST-SELECTION-SKIP] registry ever seeing it, and costs coverage \
         of its compilation too. Use `#[ignore = \"..\"]`, which the spec \
         mandates and which leaves the target inside --all-targets."
    );
    assert_eq!(
        curated.difference(&found).collect::<Vec<_>>(),
        Vec::<&(String, String)>::new(),
        "a curated conditional test no longer carries a `#[cfg]`; remove it \
         from CONDITIONAL_TESTS so the list keeps meaning something"
    );
    Ok(())
}

/// Every cargo feature a `#[cfg]`-gated test depends on is proved live by
/// an unconditional pin in the same test target.
///
/// This is the half [`every_cfg_gated_test_is_accounted_for`] cannot
/// reach. A registry entry records that a `#[cfg]` exists; it says nothing
/// about whether the required command satisfies it, so a feature-gated
/// test can sit in the registry looking accounted for while compiling
/// nowhere — which is exactly what `profiling` did. The pin closes it by
/// running: drop the feature from `_TEST_FEATURES` and the pin fails
/// rather than the gated test silently disappearing.
#[test]
fn every_feature_gated_test_is_proved_live_by_a_pin_beside_it() -> Result<()> {
    let (file, test, feature) = FEATURE_GATED_TEST;
    let condition = conditional_tests()?
        .into_iter()
        .find(|(found_file, found_test, _)| found_file == file && found_test == test)
        .map(|(_, _, condition)| condition)
        .ok_or_else(|| anyhow!("{file}::{test} no longer carries a `#[cfg]`"))?;
    assert!(
        condition.contains(feature),
        "{file}::{test} is pinned as the feature-gated case, so it must be \
         `{feature}` that gates it: {condition}"
    );
    let pins = feature_liveness_pins()?;
    assert!(
        pins.contains(&(FEATURE_LIVENESS_PIN.to_owned(), feature.to_owned())),
        "{file}::{test} compiles only with `{feature}` on, and nothing in \
         {FEATURE_LIVENESS_PIN} asserts it is. Add an unconditional \
         `#[test]` there asserting `cfg!(feature = \"{feature}\")`, or the \
         gated test goes back to being absent rather than skipped the next \
         time the feature set changes. Pins found: {pins:?}"
    );
    Ok(())
}
