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
    path::Path,
    process::Command,
};

use anyhow::{anyhow, Context, Result};
use deslop_test_support::{
    corpus::repo_root,
    skip_contract::{
        bracketed_ids, breaches, registry_diff, Breach, PolicyContext, CATEGORIES, PLAN_PREFIX,
    },
    skip_policy::ignored_tests,
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
/// `corpus_repos` entries are the real-repository gate (gh #422, blocked on
/// the memory work in #166); `corpus_manifest_contract` is the curation those
/// same two oversized repositories block (gh #426); the three others are
/// assertions that are red on purpose against unfinished fusion and embedding
/// behaviour.
const CURATED_SKIPS: [(&str, &str, u32); 15] = [
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
        "crates/deslop/tests/embedding_route_invariance.rs",
        "embeddings_on_reports_every_file_set_embeddings_off_reported",
        356,
    ),
    (
        "crates/deslop/tests/issue_343_sum_clamp_saturation.rs",
        "mid_band_cluster_confidence_never_exceeds_its_strongest_axis",
        369,
    ),
];

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

#[test]
fn every_skip_in_the_tree_satisfies_the_stated_policy() -> Result<()> {
    let context = policy_context()?;
    for (skip, (_, _, issue)) in ignored_tests()?.iter().zip(CURATED_SKIPS) {
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

/// [TEST-ONE-BINARY] The runtime path a corpus test answers to inside the
/// single `suite` binary: `<module>::<test>`, where the module is the corpus
/// suite file's own stem. `--exact` matches this path, not the bare function
/// name.
fn qualified(test: &str) -> String {
    let module = Path::new(CORPUS_SUITE)
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default();
    format!("{module}::{test}")
}

#[test]
fn the_scheduled_corpus_slice_names_tests_that_still_exist() -> Result<()> {
    let suite: Vec<String> = ignored_tests()?
        .into_iter()
        .filter(|skip| skip.file == CORPUS_SUITE)
        .map(|skip| qualified(&skip.test))
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
/// spelled the way `--exact` matches it: the `<module>::<test>` path the
/// single `suite` binary reports, not the bare function name. `--exact` makes
/// a stale or unqualified name select nothing rather than something adjacent,
/// and a run that executes zero tests reports green — gh #412, one rename away.
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
