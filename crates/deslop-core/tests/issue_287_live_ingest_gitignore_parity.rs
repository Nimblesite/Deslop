//! Regression tests for #287 ([PIPELINE-DISCOVER-FILES], [LIVE-WATCHER]).
//!
//! `discover_files` prunes the tree with `WalkBuilder::standard_filters(true)`
//! — `.gitignore`, `.ignore`, and hidden dot-directories. The live ingest path
//! (`apply_changes` → `apply_one_change`) applied only the extension map and
//! [`ExclusionConfig`], so discovery's file set was a strict *subset* of what
//! the live path admitted. Any ignored source file that received a filesystem
//! event was registered into the live corpus and never evicted.
//!
//! In production this took a 221-file / 81,249-fingerprint corpus to 4,613,840
//! fingerprints and 9,701,871 pairs — 15.5 GB RSS and ~2-hour analysis passes.
//!
//! Black-box only: drives the public [`AnalysisSession`] surface and asserts
//! against the rendered [`Report`], exactly as a watcher or an LSP `didSave`
//! would feed it.

#![cfg(feature = "live")]

use std::{fs, path::Path, sync::Arc};

use anyhow::{Context, Result};
use deslop_core::{embedding::test_support::StubProvider, live::AnalysisSession};

use crate::common::*;

/// C# that clusters with the fixture's `Alpha.cs`, so an admitted copy is
/// guaranteed to surface as a live occurrence rather than being filtered out
/// downstream for being uninteresting.
const DUPLICATE_SOURCE: &str = r"namespace Vendored
{
    public class Processor
    {
        public int Compute(int input)
        {
            if (input < 0)
            {
                return 0;
            }
            int total = 0;
            for (int index = 0; index < input; index = index + 1)
            {
                total = total + index;
            }
            return total;
        }
    }
}
";

/// Writes `DUPLICATE_SOURCE` to `root/dir/Ignored.cs`, creating `dir`.
fn write_ignored_source(root: &Path, dir: &str) -> Result<std::path::PathBuf> {
    let target_dir = root.join(dir);
    fs::create_dir_all(&target_dir).with_context(|| format!("mkdir {dir}"))?;
    let file = target_dir.join("Ignored.cs");
    fs::write(&file, DUPLICATE_SOURCE).with_context(|| format!("write {dir}/Ignored.cs"))?;
    Ok(file)
}

/// Builds a workspace whose ignored trees each hold a duplicate-heavy C# file.
///
/// Covers the three `standard_filters(true)` mechanisms the live path missed:
/// a `.gitignore` rule (which the `ignore` crate honours only inside a repo,
/// hence the marker `.git` directory), an `.ignore` rule (honoured with no
/// repo at all), and a hidden dot-directory.
fn workspace_with_ignored_trees() -> Result<(tempfile::TempDir, Vec<std::path::PathBuf>)> {
    let tmp = copy_fixture("csharp-small")?;
    let root = tmp.path();

    // `ignore` only applies .gitignore rules inside a repository. Create the
    // marker directory directly — no git invocation.
    fs::create_dir_all(root.join(".git")).context("mkdir .git")?;
    fs::write(root.join(".gitignore"), "/gitignored/\n").context("write .gitignore")?;
    fs::write(root.join(".ignore"), "/dotignored/\n").context("write .ignore")?;

    let paths = vec![
        write_ignored_source(root, "gitignored")?,
        write_ignored_source(root, "dotignored")?,
        write_ignored_source(root, ".hidden-tool")?,
    ];
    Ok((tmp, paths))
}

/// The ignored trees must be invisible to discovery, and must *stay* invisible
/// when the live ingest path is handed their paths directly — which is exactly
/// what the file watcher, an LSP `didSave`, and the freshness tracker all do.
///
/// Before the fix, `apply_changes` registered all three files, inflating
/// `files_analysed` and surfacing occurrences discovery had deliberately pruned.
#[tokio::test(flavor = "multi_thread")]
async fn live_ingest_rejects_paths_discovery_excludes() -> Result<()> {
    let (tmp, ignored_paths) = workspace_with_ignored_trees()?;
    let provider = Arc::new(StubProvider::new());
    let mut session = AnalysisSession::new(tmp.path().to_path_buf(), 15, false, None, provider)
        .context("session")?;

    let before = session.report();
    let files_before = before.files_analysed;
    assert_eq!(
        files_before, 2,
        "discovery must see only Alpha.cs and Beta.cs; the gitignored, \
         dot-ignored and hidden trees must be pruned by the walker. \
         files_analysed={files_before}",
    );
    for component in ["gitignored", "dotignored", ".hidden-tool"] {
        assert_eq!(
            occurrences_with_component(&before, component),
            0,
            "discovery must not surface occurrences under `{component}`: {:?}",
            before.clusters,
        );
    }

    // Feed the ignored paths straight into the live ingest path, exactly as a
    // filesystem event or an editor save does.
    let _delta = session
        .apply_changes(&ignored_paths)
        .context("apply ignored paths")?;

    let after = session.report();
    assert_eq!(
        after.files_analysed,
        files_before,
        "live ingest must admit no file discovery would have excluded; \
         corpus grew from {files_before} to {} after feeding {} ignored paths",
        after.files_analysed,
        ignored_paths.len(),
    );
    for component in ["gitignored", "dotignored", ".hidden-tool"] {
        assert_eq!(
            occurrences_with_component(&after, component),
            0,
            "live ingest must not surface occurrences under `{component}` — \
             discovery prunes it, so the live path must too: {:?}",
            after.clusters,
        );
    }
    Ok(())
}

/// The parity must not go stale: an ignore-rule edit mid-session re-scopes
/// the live gate in both directions. Adding `/vendored/` to `.gitignore`
/// evicts the already-admitted tree exactly as a fresh discovery would prune
/// it; emptying the rule re-admits it. Fed through `apply_changes`, exactly
/// as the watcher delivers the `.gitignore` event.
#[tokio::test(flavor = "multi_thread")]
async fn gitignore_edit_mid_session_rescopes_live_ingest() -> Result<()> {
    let tmp = copy_fixture("csharp-small")?;
    let root = tmp.path();
    fs::create_dir_all(root.join(".git")).context("mkdir .git")?;
    let _vendored = write_ignored_source(root, "vendored")?;
    let provider = Arc::new(StubProvider::new());
    let mut session =
        AnalysisSession::new(root.to_path_buf(), 15, false, None, provider).context("session")?;

    let before = session.report();
    assert_eq!(
        before.files_analysed, 3,
        "with no ignore rule, discovery must admit Alpha.cs, Beta.cs and \
         vendored/Ignored.cs; files_analysed={}",
        before.files_analysed,
    );

    let gitignore = root.join(".gitignore");
    fs::write(&gitignore, "/vendored/\n").context("write .gitignore rule")?;
    let _delta = session
        .apply_changes(std::slice::from_ref(&gitignore))
        .context("apply .gitignore addition")?;

    let evicted = session.report();
    assert_eq!(
        evicted.files_analysed, 2,
        "adding `/vendored/` to .gitignore mid-session must evict the \
         admitted tree; files_analysed={}",
        evicted.files_analysed,
    );
    assert_eq!(
        occurrences_with_component(&evicted, "vendored"),
        0,
        "no vendored occurrence may survive the .gitignore edit: {:?}",
        evicted.clusters,
    );

    fs::write(&gitignore, "").context("empty .gitignore")?;
    let _delta = session
        .apply_changes(&[gitignore])
        .context("apply .gitignore removal")?;

    let readmitted = session.report();
    assert_eq!(
        readmitted.files_analysed, 3,
        "emptying .gitignore must re-admit the tree via re-discovery; \
         files_analysed={}",
        readmitted.files_analysed,
    );
    Ok(())
}

/// Repeated events on ignored paths must not accumulate. This is the property
/// that actually failed in production: the corpus grew monotonically across
/// 259 passes because every build touching an ignored tree re-fed it and
/// nothing ever evicted it.
#[tokio::test(flavor = "multi_thread")]
async fn live_ingest_corpus_is_invariant_across_repeated_ignored_events() -> Result<()> {
    let (tmp, ignored_paths) = workspace_with_ignored_trees()?;
    let provider = Arc::new(StubProvider::new());
    let mut session = AnalysisSession::new(tmp.path().to_path_buf(), 15, false, None, provider)
        .context("session")?;

    let baseline = session.report().files_analysed;
    assert_eq!(baseline, 2, "baseline corpus must be the two fixture files");

    for round in 1..=5 {
        // Rewrite so each round is a genuine content-modify event.
        for path in &ignored_paths {
            fs::write(path, format!("{DUPLICATE_SOURCE}// round {round}\n"))
                .with_context(|| format!("rewrite round {round}"))?;
        }
        let _delta = session
            .apply_changes(&ignored_paths)
            .with_context(|| format!("apply round {round}"))?;

        let files_now = session.report().files_analysed;
        assert_eq!(
            files_now, baseline,
            "corpus must stay invariant under repeated ignored-path events; \
             round {round} grew it from {baseline} to {files_now}",
        );
    }
    Ok(())
}
