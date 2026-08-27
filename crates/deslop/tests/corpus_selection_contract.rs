//! [CORPUS-CI] [TEST-SELECTION-SKIP] The scheduled corpus gate must select
//! tests that exist.
//!
//! `make test-corpus-ci` hands every name in the Makefile's `CORPUS_TESTS` to
//! libtest's `--exact` filter. `--exact` matches a whole test name or nothing,
//! and libtest exits 0 when a filter selects nothing. A stale, misspelled or
//! over-qualified name therefore makes the scheduled corpus workflow report
//! green having scanned no repository at all — gh #412, which
//! `docs/specs/corpus.md` §[CORPUS-CI] records as fixed.
//!
//! It came back through a different door. `CORPUS_TESTS` was written
//! `corpus_repos::corpus_tokio_rust`: a module path that would be right if the
//! corpus suite were a module of the `suite` binary. It is not.
//! `crates/deslop/Cargo.toml` gives it its own `[[test]]` target, so the file
//! is that binary's crate root and every test in it answers to its bare name.
//! Measured against the compiled binary, each of the three scheduled names
//! produced `running 0 tests ... 11 filtered out` and exit 0. The workflow's
//! `full` dispatch fed the substring `corpus_` into the same `--exact` loop
//! and did the same.
//!
//! Nothing policed selection. `skip_policy_contract` polices skips; the
//! test-selection contract in `scripts/repository` checked only that a name
//! begins with `corpus_`, which `corpus_repos::corpus_tokio_rust` does. This
//! gate checks the one thing that decides whether a repository is scanned:
//! that the name resolves to a test.
//!
//! Both slices are read from the Makefile, which is the single source of the
//! names — the workflow passes none of its own, so the two cannot drift.

use std::{collections::BTreeSet, ffi::OsStr, fs};

use anyhow::{anyhow, ensure, Context, Result};
use deslop_test_support::{corpus::repo_root, skip_policy::ignored_tests};

/// The build file that names both corpus slices.
const MAKEFILE: &str = "Makefile";
/// The suite whose tests those names must resolve inside.
const CORPUS_SUITE: &str = "crates/deslop/tests/corpus_repos.rs";
/// The manifest that gives the suite its own test target, which is why its
/// test names carry no module path.
const CORPUS_MANIFEST: &str = "crates/deslop/Cargo.toml";

/// The tests the scheduled run selects, and the repositories it clones.
const SCHEDULED_TESTS: &str = "CORPUS_TESTS";
const SCHEDULED_REPOS: &str = "CORPUS_REPOS";
/// The same pair for the `full` dispatch, which must cover the whole corpus.
const FULL_TESTS: &str = "CORPUS_TESTS_FULL";
const FULL_REPOS: &str = "CORPUS_REPOS_FULL";

/// Make's three assignment operators, so a variable is read at its own
/// declaration rather than wherever its name happens to appear.
const ASSIGNMENTS: [&str; 3] = ["=", "?=", ":="];
/// The path separator a test name would carry if the suite were a module.
const MODULE_SEPARATOR: &str = "::";
/// The libtest flag that makes a positional filter a whole-name match.
const EXACT_FLAG: &str = "--exact";

/// Where the pinned repositories are declared, one manifest each.
const CORPUS_DIRECTORY: &str = "corpus";
const MANIFEST_EXTENSION: &str = "json";
/// The one file in that directory that is not a repository.
const BASELINE_FILE: &str = "known-failures";

/// Every name the corpus test binary can be asked for.
///
/// The corpus recipes select with `--ignored`, and libtest run that way
/// executes only ignored tests, so the selectable set is exactly the ignored
/// tests declared in the suite — read off the AST, never matched out of the
/// text. They are bare because [`CORPUS_MANIFEST`] makes the suite file a
/// test binary's crate root.
fn selectable() -> Result<BTreeSet<String>> {
    let names: BTreeSet<String> = ignored_tests()?
        .into_iter()
        .filter(|skip| skip.file == CORPUS_SUITE)
        .map(|skip| skip.test)
        .collect();
    ensure!(
        !names.is_empty(),
        "{CORPUS_SUITE} declares no ignored test, so every corpus selector resolves to nothing \
         and the scheduled workflow reports green over zero repositories"
    );
    Ok(names)
}

/// True when `line` is `variable`'s own declaration.
fn assigns(line: &str, variable: &str) -> bool {
    line.strip_prefix(variable)
        .map(str::trim_start)
        .is_some_and(|rest| {
            ASSIGNMENTS
                .iter()
                .any(|operator| rest.starts_with(operator))
        })
}

/// The whitespace-separated words a Makefile variable is assigned.
fn words(makefile: &str, variable: &str) -> Result<Vec<String>> {
    let line = makefile
        .lines()
        .find(|line| assigns(line, variable))
        .ok_or_else(|| {
            anyhow!(
                "{MAKEFILE} declares no `{variable}`. The corpus slices are named there and \
                 nowhere else, so a missing one leaves the workflow selecting nothing"
            )
        })?;
    let (_name, value) = line
        .split_once('=')
        .ok_or_else(|| anyhow!("{MAKEFILE}: `{variable}` is declared with no value"))?;
    Ok(value.split_whitespace().map(ToOwned::to_owned).collect())
}

/// The Makefile, read whole.
fn makefile() -> Result<String> {
    let path = repo_root().join(MAKEFILE);
    fs::read_to_string(&path).with_context(|| format!("unreadable: {}", path.display()))
}

/// Why one selector matches no test, in the words a reader needs to fix it.
fn diagnosis(name: &str, selectable: &BTreeSet<String>) -> String {
    let bare = name.rsplit(MODULE_SEPARATOR).next().unwrap_or(name);
    if bare != name && selectable.contains(bare) {
        return format!(
            "`{bare}` is the real name. {CORPUS_MANIFEST} declares the corpus suite as its own \
             `[[test]]` target, so the file is that binary's crate root and its tests carry no \
             `{MODULE_SEPARATOR}` module path"
        );
    }
    let prefixed: Vec<&String> = selectable
        .iter()
        .filter(|test| test.starts_with(name) && *test != name)
        .collect();
    if !prefixed.is_empty() {
        return format!(
            "`{name}` is a prefix of {prefixed:?} and the name of none of them. {EXACT_FLAG} \
             matches whole names, never substrings"
        );
    }
    format!("{CORPUS_SUITE} declares no test called `{name}`")
}

/// Asserts one selector names a test the corpus binary will run.
fn assert_resolves(variable: &str, name: &str, selectable: &BTreeSet<String>) {
    assert!(
        selectable.contains(name),
        "{MAKEFILE}: {variable} selects `{name}`, which {EXACT_FLAG} resolves to no test — {}. \
         libtest exits 0 when a filter selects nothing, so the corpus workflow reports green \
         having scanned no repository (gh #412). The suite declares {selectable:?}.",
        diagnosis(name, selectable)
    );
}

/// Every repository pinned by a `corpus/<name>.json` manifest.
fn pinned_repositories() -> Result<BTreeSet<String>> {
    let directory = repo_root().join(CORPUS_DIRECTORY);
    let entries =
        fs::read_dir(&directory).with_context(|| format!("unreadable: {}", directory.display()))?;
    let mut found = BTreeSet::new();
    for entry in entries {
        let path = entry?.path();
        let is_manifest = path.extension().and_then(OsStr::to_str) == Some(MANIFEST_EXTENSION);
        let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or_default();
        if is_manifest && stem != BASELINE_FILE {
            let _inserted = found.insert(stem.to_owned());
        }
    }
    ensure!(!found.is_empty(), "{CORPUS_DIRECTORY} pins no repository");
    Ok(found)
}

/// [CORPUS-CI] The slice the scheduled run scans every night selects real
/// tests. Each name is matched with `--exact`, so a name that is merely
/// close selects nothing and the run is green over zero repositories.
#[test]
fn the_scheduled_corpus_slice_selects_tests_that_exist() -> Result<()> {
    let suite = selectable()?;
    let slice = words(&makefile()?, SCHEDULED_TESTS)?;
    assert!(
        !slice.is_empty(),
        "{MAKEFILE}: {SCHEDULED_TESTS} names no test, so the scheduled corpus workflow runs \
         nothing and reports green over zero repositories"
    );
    for name in &slice {
        assert_resolves(SCHEDULED_TESTS, name, &suite);
    }
    let repositories = words(&makefile()?, SCHEDULED_REPOS)?;
    let pinned = pinned_repositories()?;
    for repository in &repositories {
        assert!(
            pinned.contains(repository),
            "{MAKEFILE}: {SCHEDULED_REPOS} clones `{repository}`, which no \
             {CORPUS_DIRECTORY}/<name>.{MANIFEST_EXTENSION} pins. The fetch fails before any \
             test runs. Pinned: {pinned:?}."
        );
    }
    Ok(())
}

/// [CORPUS-CI] The `full` dispatch is full. Its whole purpose is covering the
/// repositories the nightly slice is too small to reach (#331 Dart, #336
/// F#), so a name missing from it is a defect nothing ever scans for.
#[test]
fn the_full_corpus_dispatch_selects_every_corpus_test() -> Result<()> {
    let suite = selectable()?;
    let full: BTreeSet<String> = words(&makefile()?, FULL_TESTS)?.into_iter().collect();
    for name in &full {
        assert_resolves(FULL_TESTS, name, &suite);
    }
    let unselected: Vec<&String> = suite.difference(&full).collect();
    assert!(
        unselected.is_empty(),
        "{MAKEFILE}: {FULL_TESTS} is the `full` dispatch, so it must select every test in \
         {CORPUS_SUITE}. It leaves {unselected:?} unrun — those repositories are scanned by \
         nothing, on any schedule."
    );
    Ok(())
}

/// [CORPUS-CI] The `full` dispatch clones every repository it will scan. A
/// test whose clone was never fetched fails on a missing directory, which
/// reads as a corpus defect and is not one.
#[test]
fn the_full_corpus_dispatch_clones_every_pinned_repository() -> Result<()> {
    let pinned = pinned_repositories()?;
    let cloned: BTreeSet<String> = words(&makefile()?, FULL_REPOS)?.into_iter().collect();
    let missing: Vec<&String> = pinned.difference(&cloned).collect();
    assert!(
        missing.is_empty(),
        "{MAKEFILE}: {FULL_REPOS} omits {missing:?}, which {CORPUS_DIRECTORY} pins. The `full` \
         dispatch would select their tests and then fail on an absent clone."
    );
    let unpinned: Vec<&String> = cloned.difference(&pinned).collect();
    assert!(
        unpinned.is_empty(),
        "{MAKEFILE}: {FULL_REPOS} clones {unpinned:?}, which no \
         {CORPUS_DIRECTORY}/<name>.{MANIFEST_EXTENSION} pins"
    );
    Ok(())
}
