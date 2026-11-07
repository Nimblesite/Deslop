//! [LIVE-CONFIG-LIVE] Corpus-scoping files outside the analysed workspace stay inert.
//!
//! A production `deslop-lsp` over an 80 GB Osprey checkout burned 6h48m of CPU across 784 passes: every write to an `osprey.toml` or `.gitignore` inside a gitignored git worktree (`.claude/worktrees/…`) or an E2E scratch tree (`.osprey/e2e/…`) re-discovered all 1,422 files and re-rendered the whole report, although discovery never descends into those directories and the report could not change.

#![cfg(feature = "live")]

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use deslop_core::live::AnalysisSession;

use crate::common::{copy_fixture, live_session, occurrences_with_component};

const FIXTURE: &str = "csharp-small";
const WORKSPACE_FILE_COUNT: usize = 2;
const READMITTED_FILE_COUNT: usize = 3;
const ROUNDS: u8 = 3;
const IGNORED_DIR: &str = "ignored";
const EXCLUDED_DIR: &str = "excluded";
const HIDDEN_DIR: &str = ".worktrees";
const DESLOP_TOML: &str = "[defaults]\nexclude = [\"excluded/**\"]\n";
const ROOT_IGNORE: &str = "/ignored/\n";
const OSPREY_MANIFEST: &str = "osprey.toml";
/// Every corpus-scoping file name the live session watches for.
const SCOPING_FILES: [&str; 3] = [OSPREY_MANIFEST, ".gitignore", ".ignore"];

/// Copies the fixture, scopes `excluded/` and `ignored/` out of it, and plants every scoping file in each out-of-scope directory, the way a git worktree or test scratch tree carries its own copies.
fn workspace_with_out_of_scope_scoping_files() -> Result<(tempfile::TempDir, Vec<PathBuf>)> {
    let workspace = copy_fixture(FIXTURE)?;
    let root = workspace.path();
    fs::write(root.join(".deslop.toml"), DESLOP_TOML).context("write .deslop.toml")?;
    fs::write(root.join(".ignore"), ROOT_IGNORE).context("write .ignore")?;
    let mut planted = Vec::new();
    for dir in [IGNORED_DIR, EXCLUDED_DIR, HIDDEN_DIR] {
        let nested = root.join(dir).join("nested");
        fs::create_dir_all(&nested).with_context(|| format!("mkdir {dir}"))?;
        fs::write(root.join(dir).join("Skipped.cs"), source_of(root)?)?;
        for name in SCOPING_FILES {
            planted.push(root.join(dir).join(name));
            planted.push(nested.join(name));
        }
    }
    Ok((workspace, planted))
}

/// The fixture's clone source, so a wrongly admitted `Skipped.cs` would visibly join a cluster.
fn source_of(root: &Path) -> Result<String> {
    fs::read_to_string(root.join("Alpha.cs")).context("read Alpha.cs")
}

/// Rewrites every planted file and feeds them all through one pass, as a worktree checkout does.
fn touch_round(session: &mut AnalysisSession, planted: &[PathBuf], round: u8) -> Result<bool> {
    for path in planted {
        fs::write(path, format!("# round {round}\n")).with_context(|| format!("round {round}"))?;
    }
    let delta = session.apply_changes(planted).context("apply round")?;
    Ok(delta.clusters_added.is_empty()
        && delta.clusters_removed.is_empty()
        && delta.clusters_updated.is_empty())
}

/// [LIVE-CONFIG-LIVE] Scoping-file churn inside ignored, excluded and hidden directories must not re-render, while the same files at the workspace root still re-scope the corpus.
#[test]
fn scoping_files_outside_the_corpus_never_republish_the_report() -> Result<()> {
    let (workspace, planted) = workspace_with_out_of_scope_scoping_files()?;
    let root = workspace.path();
    let mut session = live_session(root)?;
    let baseline = session.report();
    let baseline_generation = session.generation();
    assert_eq!(baseline.files_analysed, WORKSPACE_FILE_COUNT, "only Alpha.cs and Beta.cs are in scope");
    assert!(!baseline.clusters.is_empty(), "the fixture's clone pair must cluster");

    for round in 1..=ROUNDS {
        assert!(touch_round(&mut session, &planted, round)?, "round {round}: delta must be empty");
        assert_eq!(
            session.generation(),
            baseline_generation,
            "round {round}: scoping files under ignored, excluded or hidden directories cannot change the corpus, so the report must not be re-rendered",
        );
    }
    let after = session.report();
    assert_eq!(after.files_analysed, WORKSPACE_FILE_COUNT);
    assert_eq!(after.clusters, baseline.clusters, "the kept report must be the baseline report");

    // The control: a root manifest is in scope and must still re-render.
    let manifest = root.join(OSPREY_MANIFEST);
    fs::write(&manifest, "# root manifest\n").context("write root manifest")?;
    let _manifest_delta = session.apply_changes(&[manifest]).context("apply root manifest")?;
    assert_eq!(session.generation(), baseline_generation.saturating_add(1), "a root osprey.toml re-scopes the corpus");
    assert_eq!(session.report().files_analysed, WORKSPACE_FILE_COUNT);

    // The control: dropping the root ignore rule re-admits ignored/Skipped.cs.
    let ignore = root.join(".ignore");
    fs::write(&ignore, "").context("clear .ignore")?;
    let _ignore_delta = session.apply_changes(&[ignore]).context("apply root .ignore")?;
    assert_eq!(session.generation(), baseline_generation.saturating_add(2), "a root .ignore edit re-scopes the corpus");
    let readmitted = session.report();
    assert_eq!(readmitted.files_analysed, READMITTED_FILE_COUNT, "ignored/Skipped.cs must rejoin the corpus");
    assert!(
        occurrences_with_component(&readmitted, IGNORED_DIR) > 0,
        "the re-admitted copy of Alpha.cs must join a cluster: {:?}",
        readmitted.clusters,
    );
    assert_eq!(occurrences_with_component(&readmitted, EXCLUDED_DIR), 0, "excluded/ stays out");
    assert_eq!(occurrences_with_component(&readmitted, HIDDEN_DIR), 0, "hidden directories stay out");
    Ok(())
}
