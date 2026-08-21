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

use std::{collections::BTreeSet, fs, path::Path};

use anyhow::{anyhow, Context, Result};
use deslop_test_support::{
    corpus::repo_root,
    skip_policy::{ignored_tests, IgnoredTest},
};

/// The unfinished-feature justification: assertions are intact, the feature
/// behind them is not, and the tracking issue owns the remaining work.
const SKIP_UNFINISHED: &str = "[SKIP-UNFINISHED]";

/// The resource justification: a corpus or embedding suite whose clone,
/// wall time, or peak memory does not fit a hosted runner.
const SKIP_TOO_LARGE_FOR_CI: &str = "[SKIP-TOO-LARGE-FOR-CI]";

/// The only two justifications a skip may claim. "It was breaking CI" is not
/// on this list and never will be.
const CATEGORIES: [&str; 2] = [SKIP_UNFINISHED, SKIP_TOO_LARGE_FOR_CI];

/// How a reason names its tracking issue, how prose mentions the same issue,
/// and how a reason tells the reader to run the test anyway.
const ISSUE_MARKER: &str = "GH #";
const ISSUE_HASH: char = '#';
const RUN_INSTRUCTION: &str = "--ignored";

/// Where plans live, and the extension they carry.
const PLAN_PREFIX: &str = "docs/plans/";
const MARKDOWN_SUFFIX: &str = ".md";

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

/// Every test allowed not to run, with the issue that owns its return.
///
/// Ordered by file then test name, matching `ignored_tests()`. The eleven
/// `corpus_repos` entries are the real-repository gate (gh #422, blocked on
/// the memory work in #166); the three others are assertions that are red on
/// purpose against unfinished fusion and embedding behaviour.
const CURATED_SKIPS: [(&str, &str, u32); 14] = [
    (
        "crates/deslop-lsp/tests/lsp_embedding_determinism.rs",
        "lsp_embedding_refresh_is_bounded_and_reproducible",
        369,
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

/// `(file, test)` of every skip actually present in the tree.
fn present(found: &[IgnoredTest]) -> Vec<(String, String)> {
    found
        .iter()
        .map(|skip| (skip.file.clone(), skip.test.clone()))
        .collect()
}

/// Reads a workspace-relative file.
fn read(relative: &str) -> Result<String> {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).with_context(|| format!("unreadable: {}", path.display()))
}

/// Every `[BRACKETED-ID]` in `text`, in order. Split on the delimiters rather
/// than pattern-matched, and filtered to the shape a spec id has: upper-case,
/// digits, and hyphens.
fn bracketed_ids(text: &str) -> Vec<String> {
    text.split('[')
        .skip(1)
        .filter_map(|rest| rest.split(']').next())
        .filter(|id| !id.is_empty() && id.chars().all(is_spec_id_character))
        .map(ToOwned::to_owned)
        .collect()
}

/// The characters a hierarchical spec id is built from.
fn is_spec_id_character(character: char) -> bool {
    character.is_ascii_uppercase() || character.is_ascii_digit() || character == '-'
}

/// Every issue number `text` mentions as `#<n>`. Loose on purpose: a skip
/// must cite the strict `GH #<n>` form, but the plan it points at is prose
/// and writes the same issue as `#<n>`.
fn issue_mentions(text: &str) -> Vec<u32> {
    text.split(ISSUE_HASH)
        .skip(1)
        .filter_map(|rest| {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect()
}

/// Every `docs/plans/<name>.md` path `text` names.
fn plan_paths(text: &str) -> Vec<String> {
    text.split(PLAN_PREFIX)
        .skip(1)
        .filter_map(|rest| rest.split(MARKDOWN_SUFFIX).next())
        .map(|stem| format!("{PLAN_PREFIX}{stem}{MARKDOWN_SUFFIX}"))
        .collect()
}

/// Every spec id declared anywhere under `docs/specs`, so a skip cannot
/// cross-reference an id that no specification defines.
fn declared_spec_ids() -> Result<BTreeSet<String>> {
    let directory = repo_root().join(SPEC_DIRECTORY);
    let mut declared = BTreeSet::new();
    for entry in fs::read_dir(&directory).context("docs/specs must be readable")? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            declared.extend(bracketed_ids(&read_path(&path)?));
        }
    }
    Ok(declared)
}

/// Reads an absolute path.
fn read_path(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("unreadable: {}", path.display()))
}

#[test]
fn the_ignored_tests_in_the_tree_are_exactly_the_curated_set() -> Result<()> {
    let found = ignored_tests()?;
    assert_eq!(
        present(&found),
        curated(),
        "the set of `#[ignore]`d tests changed. Adding one is a deliberate act: give it a \
         tracking issue, a plan, and an entry in CURATED_SKIPS. Removing one because its fix \
         landed means deleting its entry here too — a skip that outlives its defect protects \
         nothing and reads as coverage."
    );
    assert_eq!(
        found.len(),
        CURATED_SKIPS.len(),
        "a file declares the same test name twice, so the curated set no longer identifies it"
    );
    Ok(())
}

#[test]
fn every_skip_states_exactly_one_allowed_justification() -> Result<()> {
    for skip in ignored_tests()? {
        assert_single_category(&skip);
    }
    Ok(())
}

/// A skip states its justification, and states exactly one of the two the
/// specification allows.
fn assert_single_category(skip: &IgnoredTest) {
    assert!(
        !skip.reason.is_empty(),
        "{}::{} is a bare `#[ignore]`. A skip with no stated reason is a test deleted without a \
         commit message.",
        skip.file,
        skip.test
    );
    let claimed: Vec<&str> = CATEGORIES
        .into_iter()
        .filter(|category| skip.reason.contains(category))
        .collect();
    assert_eq!(
        claimed.len(),
        1,
        "{}::{} claims {claimed:?}; a skip states exactly one of {CATEGORIES:?}. \"it was \
         breaking CI\" is not a category and never will be. Reason: {}",
        skip.file,
        skip.test,
        skip.reason
    );
}

#[test]
fn every_skip_names_the_issue_that_owns_its_return_and_the_curated_set_agrees() -> Result<()> {
    for (skip, (_, _, expected)) in ignored_tests()?.iter().zip(CURATED_SKIPS) {
        let named = issue_mentions(&skip.reason);
        assert!(
            skip.reason.contains(&format!("{ISSUE_MARKER}{expected}")),
            "{}::{} must cite `{ISSUE_MARKER}{expected}` — the issue that says why it does not \
             run and what would let it run again. It cites {named:?}. Reason: {}",
            skip.file,
            skip.test,
            skip.reason
        );
    }
    Ok(())
}

#[test]
fn every_skip_names_a_plan_that_exists_and_covers_its_issue() -> Result<()> {
    for (skip, (_, _, issue)) in ignored_tests()?.iter().zip(CURATED_SKIPS) {
        let plans = plan_paths(&skip.reason);
        assert!(
            !plans.is_empty(),
            "{}::{} names no `{PLAN_PREFIX}*{MARKDOWN_SUFFIX}`. A skip without a plan is a \
             feature abandoned in place. Reason: {}",
            skip.file,
            skip.test,
            skip.reason
        );
        assert_plans_cover(&skip.file, &skip.test, &plans, issue)?;
    }
    Ok(())
}

/// Every plan a skip names must exist, and at least one must discuss the
/// issue the skip hangs on — otherwise the citation is decorative.
fn assert_plans_cover(file: &str, test: &str, plans: &[String], issue: u32) -> Result<()> {
    let mut covering = 0_usize;
    for plan in plans {
        let body = read(plan)
            .with_context(|| format!("{file}::{test} cites {plan}, which is not in the tree"))?;
        covering += usize::from(issue_mentions(&body).contains(&issue));
    }
    assert!(
        covering > 0,
        "{file}::{test} cites {plans:?}, and not one of them mentions \
         `{ISSUE_MARKER}{issue}`. The plan has to say how the skip ends."
    );
    Ok(())
}

#[test]
fn every_skip_cross_references_a_spec_id_that_a_specification_declares() -> Result<()> {
    let declared = declared_spec_ids()?;
    for skip in ignored_tests()? {
        assert_cites_declared_spec_id(&skip, &declared);
    }
    Ok(())
}

/// A skip names at least one spec id besides its own category tag, and every
/// id it names is one a specification actually declares.
fn assert_cites_declared_spec_id(skip: &IgnoredTest, declared: &BTreeSet<String>) {
    let cited: Vec<String> = bracketed_ids(&skip.reason)
        .into_iter()
        .filter(|id| !CATEGORIES.contains(&format!("[{id}]").as_str()))
        .collect();
    assert!(
        !cited.is_empty(),
        "{}::{} cites no spec id, so nothing connects the skipped behaviour to the specification \
         it is supposed to satisfy. Reason: {}",
        skip.file,
        skip.test,
        skip.reason
    );
    let unknown: Vec<&String> = cited.iter().filter(|id| !declared.contains(*id)).collect();
    assert!(
        unknown.is_empty(),
        "{}::{} cites {unknown:?}, which no file under {SPEC_DIRECTORY} declares",
        skip.file,
        skip.test
    );
}

#[test]
fn every_skip_tells_the_reader_how_to_run_it_anyway() -> Result<()> {
    for skip in ignored_tests()? {
        assert!(
            skip.reason.contains(RUN_INSTRUCTION),
            "{}::{} must say how to run it — `{RUN_INSTRUCTION}` — so the assertions stay \
             reachable to whoever picks the issue up. Reason: {}",
            skip.file,
            skip.test,
            skip.reason
        );
    }
    Ok(())
}

#[test]
fn the_categories_this_gate_enforces_are_the_ones_the_specification_defines() -> Result<()> {
    let spec = read(POLICY_SPEC)?;
    for category in CATEGORIES {
        assert!(
            spec.contains(category),
            "{POLICY_SPEC} does not define {category}. Code, specs and tests must agree: this \
             gate would be rejecting skips on a rule nobody wrote down."
        );
    }
    let ids = bracketed_ids(&spec);
    for category in CATEGORIES {
        let bare = category.trim_start_matches('[').trim_end_matches(']');
        assert!(
            ids.iter().any(|id| id == bare),
            "{POLICY_SPEC} mentions {category} only in prose, not as a declared id"
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

/// Every name the scheduled slice selects must be a test the suite declares.
/// `--exact` makes a stale name select nothing rather than something adjacent,
/// and a run that executes zero tests reports green — gh #412, one rename away.
fn assert_slice_resolves(slice: &[String], suite: &[String]) {
    for name in slice {
        assert!(
            suite.contains(name),
            "{MAKEFILE}: {CORPUS_SLICE_VARIABLE} selects `{name}`, which is not a test in \
             {CORPUS_SUITE}. The scheduled run would execute zero tests and report green. \
             The suite declares {suite:?}."
        );
    }
}
