//! [CORPUS-PIN] Every corpus repository is pinned to a commit id, never a
//! version.
//!
//! A tag is a name, and a name can be re-pointed at different source. A curated
//! duplicate list, or a judged clone register, describes line ranges in one
//! exact tree; read the same list against a re-cut tag and every range names
//! something else, while the run stays green. The pin is therefore the commit
//! object name in full, and it is the only pin a manifest is allowed to carry —
//! a `tag` field sitting beside it is an invitation to fetch by the weaker one.
//!
//! [TEST-SELECTION-SKIP] These read JSON and shell text only, need no clone on
//! disk, and carry no `#[ignore]`: they run in `make test`.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use deslop_test_support::{corpus::repo_root, read_json};
use serde_json::Value;

/// The two manifest sets: scan targets for the resource suite, and the judged
/// clone registers. Both name upstream repositories, so both must pin commits.
const MANIFEST_DIRS: [&str; 2] = ["corpus", "corpus/register"];
/// Files in those directories that describe no single repository.
const NOT_A_REPOSITORY: [&str; 3] = ["known-failures", "score-thresholds", "judging-queue"];
/// Repositories queued for a first judging pass. They are scanned like a
/// register but carry no verdicts yet, so they live in their own list.
const JUDGING_QUEUE: &str = "corpus/judging-queue.json";
const REGISTER_DIR: &str = "corpus/register";
const QUEUED_REPOSITORIES: &str = "repositories";
const NAME: &str = "name";
/// The pin field, and its length as a full git object name.
const SHA: &str = "sha";
const COMMIT_ID_LENGTH: usize = 40;
/// The alphabet a lowercase git object name is written in.
const COMMIT_ID_ALPHABET: &str = "0123456789abcdef";
/// The weaker pin that must not sit beside the commit id.
const VERSION_PIN: &str = "tag";
/// Scripts that fetch corpus repositories. Each must ask for a commit.
const FETCH_SCRIPTS: [&str; 2] = ["scripts/corpus/target-repos.sh", "scripts/corpus/fetch-corpus.mjs"];
/// The git argument that fetches a name rather than a commit.
const FETCH_BY_NAME: &str = "--branch";

/// Every repository manifest, as `(directory, stem, document)`.
fn manifests() -> Result<Vec<(String, String, Value)>> {
    let root = repo_root();
    let mut found = Vec::new();
    for directory in MANIFEST_DIRS {
        for entry in fs::read_dir(root.join(directory))
            .with_context(|| format!("{directory} must be readable"))?
        {
            let path = entry?.path();
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_owned();
            if path.extension().is_some_and(|ext| ext == "json")
                && !NOT_A_REPOSITORY.contains(&stem.as_str())
            {
                found.push((directory.to_owned(), stem, read_json(&path)?));
            }
        }
    }
    found.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));
    Ok(found)
}


/// Whether `candidate` is a full, lowercase, hexadecimal git object name.
fn is_commit_id(candidate: &str) -> bool {
    candidate.len() == COMMIT_ID_LENGTH
        && candidate
            .chars()
            .all(|character| COMMIT_ID_ALPHABET.contains(character))
}

#[test]
fn every_corpus_repository_is_pinned_to_a_full_commit_id() -> Result<()> {
    let found = manifests()?;
    assert!(
        found.len() >= MANIFEST_DIRS.len(),
        "no repository manifests were read at all"
    );
    for (directory, stem, manifest) in &found {
        let pin = manifest.get(SHA).and_then(Value::as_str).unwrap_or_default();
        assert!(
            is_commit_id(pin),
            "{directory}/{stem}.json pins `{pin}`, which is not a {COMMIT_ID_LENGTH}-character \
             commit id. A version label names source that upstream can move; a commit id is \
             the source"
        );
    }
    Ok(())
}

#[test]
fn no_corpus_repository_carries_a_version_beside_its_commit() -> Result<()> {
    for (directory, stem, manifest) in manifests()? {
        assert!(
            manifest.get(VERSION_PIN).is_none(),
            "{directory}/{stem}.json still carries `{VERSION_PIN}`. Two pins mean the weaker \
             one eventually gets used, and a re-cut tag then re-baselines the whole list in \
             silence"
        );
    }
    Ok(())
}

#[test]
fn nothing_fetches_a_corpus_repository_by_name() -> Result<()> {
    let root = repo_root();
    for script in FETCH_SCRIPTS {
        let source = fs::read_to_string(root.join(script))
            .with_context(|| format!("unreadable: {script}"))?;
        assert!(
            !source.contains(FETCH_BY_NAME),
            "{script} still fetches with `{FETCH_BY_NAME}`, which resolves a tag or branch at \
             fetch time. Fetch the pinned commit and verify HEAD against it"
        );
    }
    Ok(())
}

/// Every queued repository, as `(name, document)`.
fn queued() -> Result<Vec<(String, Value)>> {
    let queue: Value = read_json(&repo_root().join(JUDGING_QUEUE))?;
    let repositories = queue
        .get(QUEUED_REPOSITORIES)
        .and_then(Value::as_array)
        .with_context(|| format!("{JUDGING_QUEUE} must list `{QUEUED_REPOSITORIES}`"))?;
    Ok(repositories
        .iter()
        .map(|repository| {
            let name = repository
                .get(NAME)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            (name, repository.clone())
        })
        .collect())
}

#[test]
fn every_queued_repository_is_pinned_to_a_full_commit_id() -> Result<()> {
    let found = queued()?;
    assert!(
        !found.is_empty(),
        "{JUDGING_QUEUE} queues nothing, so no new language can ever enter the register"
    );
    for (name, repository) in &found {
        assert!(
            !name.is_empty(),
            "a queued repository has no `{NAME}`, so nothing can refer to it"
        );
        let pin = repository
            .get(SHA)
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(
            is_commit_id(pin),
            "{JUDGING_QUEUE} pins {name} by `{pin}`, which is not a {COMMIT_ID_LENGTH}-character \
             commit id. The judge and the scan must read one identical tree"
        );
    }
    Ok(())
}

#[test]
fn a_queued_repository_has_no_register_yet() -> Result<()> {
    let root = repo_root();
    for (name, _) in queued()? {
        let register = root.join(REGISTER_DIR).join(format!("{name}.json"));
        assert!(
            !register.exists(),
            "{name} is queued for a first judging pass and already has {}. A queue that never \
             drains scans the same repository twice and buys nothing",
            register.display()
        );
    }
    Ok(())
}
