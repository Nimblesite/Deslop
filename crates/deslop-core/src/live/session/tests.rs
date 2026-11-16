//! [LIVE-CACHE-SEED] What the seed window holds while the cold pass runs.

use std::sync::Arc;

use anyhow::{Context as _, Result};

use super::AnalysisSession;
use crate::embedding::{EmbeddingMode, NoopProvider};

/// Subtree-size floor shared by the cold and the seeded session.
const MIN_NODES: u32 = 30;
/// Edits a user makes, and a watcher reports, before the cold pass lands.
const KEYSTROKES: usize = 500;

/// Memory leak: every change in the seed window appended its path to the
/// replay queue, so a file typed into while a large workspace cold-scans was
/// held once per keystroke and watcher flush, and re-analysed that many times
/// when the pipeline installed.
#[test]
fn seed_window_queues_each_changed_path_once_however_often_it_changes() -> Result<()> {
    let root = tempfile::tempdir()?;
    let _cold = AnalysisSession::new(
        root.path().to_path_buf(),
        MIN_NODES,
        false,
        None,
        Arc::new(NoopProvider::new()),
    )?;
    let mut seeded = AnalysisSession::try_seeded_from_cache(
        root.path().to_path_buf(),
        MIN_NODES,
        false,
        None,
        Arc::new(NoopProvider::new()),
        EmbeddingMode::Off,
    )
    .context("the cold session must persist a usable report seed")?;
    assert!(seeded.is_seed_only(), "no pipeline is installed yet");

    let edited = root.path().join("edited.rs");
    let saved = root.path().join("saved.rs");
    for _ in 0..KEYSTROKES {
        let _delta = seeded.apply_changes(&[edited.clone(), saved.clone()])?;
    }

    assert_eq!(
        seeded.pending_changes.len(),
        2,
        "each changed path is queued once, not once per change"
    );
    assert!(seeded.pending_changes.contains(&edited));
    assert!(seeded.pending_changes.contains(&saved));
    assert!(seeded.is_seed_only(), "queuing never installs a pipeline");
    Ok(())
}
