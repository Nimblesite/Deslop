//! Regression tests for #299 ([LIVE-SCHEDULER-NOOP], [LIVE-SCHEDULER],
//! [LIVE-WATCHER]).
//!
//! `PipelineSession::update_files` ran the full LSH → embedding →
//! candidate-pair → transitive-closure → rank → render pipeline over the
//! *entire* corpus on every pass, including passes where every changed path
//! was filtered out before it could touch a single analysed file. The parse
//! stage honoured the incremental cache; everything downstream did not.
//!
//! In production one `deslop-lsp` instance burned **11h17m of CPU across
//! 1086 passes** on a 172-file workspace, 139 of 157 logged passes reporting
//! `0 added, 0 removed, 0 updated` — a byte-identical report, re-derived from
//! 422,711 fingerprints and 366,765 candidate pairs, roughly every 30
//! seconds. Each one also bumped the generation and broadcast
//! `report/changed`, so every subscriber re-fetched a report that had not
//! changed.
//!
//! Black-box only: drives the public [`AnalysisSession`] surface, exactly as
//! the watcher, the freshness tracker, and an LSP `didSave` feed it.

#![cfg(feature = "live")]

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};

mod common;
use crate::common::*;

/// C# that clusters with the fixture's `Alpha.cs`, so if any gate were to
/// wrongly admit one of the inert paths the corpus would visibly grow rather
/// than failing silently.
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

/// Builds a workspace whose every extra path is *inert* — it can generate
/// filesystem events forever without ever entering the corpus. One path per
/// gate in `apply_one_change`, so a regression in any single gate is caught:
///
/// * `build.log` — unsupported extension.
/// * `excluded/Skipped.cs` — matched by `.deslop.toml` `exclude`.
/// * `ignored/Skipped.cs` — matched by an `.ignore` rule.
/// * `Ghost.cs` — never created; the deletion/temp-file event for a path the
///   corpus never held (editor atomic saves produce these constantly).
///
/// Returns the temp dir alongside the inert paths and the subset of them that
/// exists on disk and can therefore be rewritten between rounds.
fn workspace_with_inert_paths() -> Result<(tempfile::TempDir, Vec<PathBuf>, Vec<PathBuf>)> {
    let tmp = copy_fixture("csharp-small")?;
    let root = tmp.path();

    fs::write(
        root.join(".deslop.toml"),
        "[defaults]\nexclude = [\"excluded/**\"]\n",
    )
    .context("write .deslop.toml")?;
    // `.ignore` is honoured with no repository present, unlike `.gitignore`.
    fs::write(root.join(".ignore"), "/ignored/\n").context("write .ignore")?;

    let build_log = root.join("build.log");
    fs::write(&build_log, "linking...\n").context("write build.log")?;
    let mut on_disk = vec![build_log];
    for dir in ["excluded", "ignored"] {
        let target = root.join(dir);
        fs::create_dir_all(&target).with_context(|| format!("mkdir {dir}"))?;
        let file = target.join("Skipped.cs");
        fs::write(&file, DUPLICATE_SOURCE).with_context(|| format!("write {dir}/Skipped.cs"))?;
        on_disk.push(file);
    }

    let mut inert = on_disk.clone();
    inert.push(root.join("Ghost.cs"));
    Ok((tmp, inert, on_disk))
}

/// The core regression: a pass that mutates no analysed file must not publish
/// a new report generation, and must not disturb the report it kept.
///
/// The generation is the observable proxy for the wasted work — it is bumped
/// in the same breath as the render, so a stable generation across repeated
/// inert passes is exactly the property that was missing in production. Five
/// rounds, because the production failure was a *sustained* stream of inert
/// events, not one.
#[tokio::test(flavor = "multi_thread")]
async fn inert_paths_never_publish_a_new_generation() -> Result<()> {
    let (tmp, inert, on_disk) = workspace_with_inert_paths()?;
    let mut session = live_session(tmp.path())?;

    let baseline_generation = session.generation();
    let baseline = session.report();
    assert_eq!(
        baseline.files_analysed, 2,
        "baseline corpus must be exactly Alpha.cs and Beta.cs; the excluded, \
         ignored and unsupported paths must never be discovered. \
         files_analysed={}",
        baseline.files_analysed,
    );
    assert!(
        !baseline.clusters.is_empty(),
        "fixture must produce at least one cluster, or a preserved report \
         proves nothing",
    );

    for round in 1..=5 {
        for path in &on_disk {
            fs::write(path, format!("{DUPLICATE_SOURCE}// round {round}\n"))
                .with_context(|| format!("rewrite round {round}"))?;
        }
        let delta = session
            .apply_changes(&inert)
            .with_context(|| format!("apply round {round}"))?;

        assert_eq!(
            session.generation(),
            baseline_generation,
            "round {round}: a pass that mutates no analysed file must reuse \
             the previous report, not re-render and publish generation {}",
            session.generation(),
        );
        assert!(
            delta.clusters_added.is_empty()
                && delta.clusters_removed.is_empty()
                && delta.clusters_updated.is_empty(),
            "round {round}: inert paths must yield an empty delta, got {delta:?}",
        );
    }

    let after = session.report();
    assert_eq!(
        after.files_analysed, baseline.files_analysed,
        "the preserved report must still describe the original corpus",
    );
    assert_eq!(
        after.clusters.len(),
        baseline.clusters.len(),
        "skipping the render must preserve the clusters, not drop them",
    );
    for component in ["excluded", "ignored"] {
        assert_eq!(
            occurrences_with_component(&after, component),
            0,
            "no occurrence may appear under `{component}`: {:?}",
            after.clusters,
        );
    }
    Ok(())
}

/// The control. A fix that simply stopped bumping the generation would pass
/// the test above and break the entire live loop, so prove the same session
/// still publishes when an analysed file genuinely changes — and that it does
/// so *from* the generation the inert rounds left untouched.
#[tokio::test(flavor = "multi_thread")]
async fn a_real_edit_still_publishes_after_inert_rounds() -> Result<()> {
    let (tmp, inert, _on_disk) = workspace_with_inert_paths()?;
    let mut session = live_session(tmp.path())?;

    let baseline_generation = session.generation();
    let _inert_delta = session.apply_changes(&inert).context("apply inert")?;
    assert_eq!(
        session.generation(),
        baseline_generation,
        "inert pass must leave the generation alone",
    );

    let edited = tmp.path().join("Beta.cs");
    fs::write(
        &edited,
        b"namespace Beta { public class Differ { public int Run(int x) { return x + 1; } } }\n",
    )
    .context("write Beta")?;
    let delta = session
        .apply_changes(std::slice::from_ref(&edited))
        .context("apply edit")?;

    assert_eq!(
        session.generation(),
        baseline_generation.saturating_add(1),
        "a real edit must publish exactly one new generation",
    );
    assert!(
        !(delta.clusters_added.is_empty()
            && delta.clusters_removed.is_empty()
            && delta.clusters_updated.is_empty()),
        "a real edit must produce a non-empty delta, got {delta:?}",
    );
    Ok(())
}

/// A deletion of a file the corpus *does* hold is a real mutation and must
/// still re-render — the early-out keys off whether the corpus changed, never
/// off whether the path still exists.
#[tokio::test(flavor = "multi_thread")]
async fn deleting_an_analysed_file_still_publishes() -> Result<()> {
    let (tmp, _inert, _on_disk) = workspace_with_inert_paths()?;
    let mut session = live_session(tmp.path())?;

    let baseline_generation = session.generation();
    let doomed = tmp.path().join("Beta.cs");
    fs::remove_file(&doomed).context("remove Beta")?;
    let _delta = session
        .apply_changes(std::slice::from_ref(&doomed))
        .context("apply deletion")?;

    assert_eq!(
        session.generation(),
        baseline_generation.saturating_add(1),
        "removing an analysed file must publish a new generation",
    );
    assert_eq!(
        session.report().files_analysed,
        1,
        "the deleted file must leave the corpus",
    );
    Ok(())
}
