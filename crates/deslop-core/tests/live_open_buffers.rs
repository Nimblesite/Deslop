//! [LIVE-OPEN-BUFFERS] The report follows the editor's open buffer, not the
//! bytes on disk. Drives `AnalysisSession` over the `csharp-small` fixture:
//! open a file, delete its clone in the unsaved buffer, and the clone leaves
//! the report; restore it, and it returns; close the buffer without saving,
//! and the disk is the truth again. Also isolates the edit arithmetic on
//! `OpenBuffers`: UTF-16 columns, ordered edits, clamped columns and
//! whole-buffer replacement.

#![cfg(feature = "live")]

use std::fs;

use anyhow::{anyhow, Result};
use deslop_core::{
    live::LiveError,
    refactor::position::{line_col_for_byte, LineCol},
    report::ReportCluster,
    BufferEdit, OpenBuffers, Report,
};

use crate::common::{copy_fixture, live_session};

/// The two-file C# workspace whose one clone spans both files.
const FIXTURE: &str = "csharp-small";
const EDITED_FILE: &str = "Alpha.cs";
const PEER_FILE: &str = "Beta.cs";
/// Replacement text that removes the edited span outright.
const DELETED: &str = "";
/// One byte inside the deleted clone, used to query the range it covered.
const INSIDE_OCCURRENCE_BYTE: usize = 1;
/// Text the scratch-buffer test writes to disk first.
const ON_DISK: &str = "on disk\n";
/// Text the editor opens over it. The emoji on the middle line makes the
/// UTF-16 column arithmetic observable: it is one character, two code
/// units, four bytes.
const OPENED: &str = "ab\ncd\u{1F600}ef\ngh\n";
/// `OPENED` after the three edits of `ordered_edits`, applied in order.
const AFTER_ORDERED_EDITS: &str = "X\u{1F600}-ef\ngh\n";
/// `AFTER_ORDERED_EDITS` after an insertion whose column overshoots the
/// line: it lands at the line end, before the newline.
const AFTER_CLAMPED_INSERT: &str = "X\u{1F600}-ef!\ngh\n";
/// A whole-buffer replacement.
const WHOLE_REPLACEMENT: &str = "z";
/// A column far past the end of any line in `OPENED`.
const PAST_LINE_END: u32 = 99;

#[test]
fn unsaved_buffer_edits_drive_the_report() -> Result<()> {
    let tmp = copy_fixture(FIXTURE)?;
    let mut session = live_session(tmp.path())?;
    let initial = session.report();
    let cluster = cluster_spanning(&initial, EDITED_FILE, PEER_FILE)?;
    let occurrence = cluster
        .occurrences
        .iter()
        .find(|occurrence| occurrence.path.ends_with(EDITED_FILE))
        .ok_or_else(|| anyhow!("cluster has no occurrence in {EDITED_FILE}: {cluster:?}"))?;
    let (start_byte, end_byte) = (occurrence.start_byte, occurrence.end_byte);
    let probe = start_byte.saturating_add(INSIDE_OCCURRENCE_BYTE);
    let file = tmp.path().join(EDITED_FILE);
    let disk_text = fs::read_to_string(&file)?;

    let opened = session.open_buffer(&file, disk_text.clone())?;
    assert!(
        opened.is_empty(),
        "opening a buffer identical to the disk is a no-op pass: {opened:?}"
    );
    assert_eq!(session.report().clusters, initial.clusters);

    let deletion = ranged(
        line_col_for_byte(&disk_text, start_byte),
        line_col_for_byte(&disk_text, end_byte),
        DELETED,
    );
    let deleted = session.edit_buffer(&file, std::slice::from_ref(&deletion))?;
    assert!(
        !deleted.is_empty(),
        "deleting a clone in the buffer must publish a delta"
    );
    let without = session.report();
    assert!(
        !has_cluster(&without, &cluster.id),
        "the deleted clone must leave the report: {without:?}"
    );
    assert!(
        without
            .clusters
            .iter()
            .flat_map(|cluster| &cluster.occurrences)
            .all(|occurrence| !occurrence.path.ends_with(EDITED_FILE)),
        "no clone may be reported in the file whose copy was deleted: {without:?}"
    );
    assert_eq!(
        without.files_analysed, initial.files_analysed,
        "the edited file is still analysed, it just holds no clone"
    );
    assert!(
        session.report_for_range(&file, probe, probe).is_empty(),
        "a range inside the deleted clone must report nothing"
    );
    assert_eq!(
        fs::read_to_string(&file)?,
        disk_text,
        "the disk is untouched; the deletion lives only in the buffer"
    );

    let restored = session.edit_buffer(&file, &[whole(&disk_text)])?;
    assert!(
        !restored.is_empty(),
        "restoring the clone must publish a delta"
    );
    assert_eq!(
        session.report().clusters,
        initial.clusters,
        "restoring the buffer text reproduces the initial clusters exactly"
    );
    assert!(
        session
            .report_for_range(&file, probe, probe)
            .iter()
            .any(|hit| hit.id == cluster.id),
        "the restored clone answers the range query again"
    );

    let _deleted_again = session.edit_buffer(&file, std::slice::from_ref(&deletion))?;
    assert!(!has_cluster(&session.report(), &cluster.id));
    let closed = session.close_buffer(&file)?;
    assert!(
        !closed.is_empty(),
        "closing an unsaved buffer must publish the return to the disk truth"
    );
    assert_eq!(
        session.report().clusters,
        initial.clusters,
        "closing an unsaved buffer makes the disk the truth again"
    );
    Ok(())
}

#[test]
fn edits_to_a_buffer_the_editor_never_opened_are_refused() -> Result<()> {
    let tmp = copy_fixture(FIXTURE)?;
    let mut session = live_session(tmp.path())?;
    let initial = session.report();
    let file = tmp.path().join(EDITED_FILE);

    let outcome = session.edit_buffer(&file, &[whole(DELETED)]);
    assert!(
        matches!(outcome, Err(LiveError::BufferNotOpen { ref path }) if path == &file),
        "an edit without an open buffer is refused by name: {outcome:?}"
    );
    assert_eq!(
        session.report().clusters,
        initial.clusters,
        "a refused edit changes nothing"
    );
    let closed = session.close_buffer(&file)?;
    assert!(
        closed.is_empty(),
        "closing a buffer that was never open re-reads the disk and changes nothing"
    );
    Ok(())
}

#[test]
fn open_buffers_apply_ordered_utf16_edits_and_fall_back_to_disk() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let file = tmp.path().join("Buffer.cs");
    fs::write(&file, ON_DISK)?;
    let mut buffers = OpenBuffers::default();

    let from_disk = buffers.read(&file)?;
    assert_eq!(
        from_disk.bytes,
        ON_DISK.as_bytes(),
        "a closed path reads the disk"
    );
    assert!(!from_disk.unsaved);
    assert!(
        !buffers.edit(&file, &[whole(WHOLE_REPLACEMENT)]),
        "an edit to a file that is not open is refused"
    );
    assert_eq!(buffers.read(&file)?.bytes, ON_DISK.as_bytes());

    buffers.open(&file, OPENED.to_owned());
    let canonical = fs::canonicalize(&file)?;
    assert_eq!(
        buffers.paths().collect::<Vec<_>>(),
        vec![canonical.as_path()]
    );
    assert!(buffers.edit(&file, &ordered_edits()));
    let edited = buffers.read(&file)?;
    assert_eq!(edited.bytes, AFTER_ORDERED_EDITS.as_bytes());
    assert!(edited.unsaved, "buffer text is marked unsaved");
    assert!(buffers.edit(&file, &[ranged(at(0, 2), at(0, 0), DELETED)]));
    assert_eq!(
        buffers.read(&file)?.bytes,
        AFTER_ORDERED_EDITS.as_bytes(),
        "an end before the start deletes nothing"
    );
    assert!(buffers.edit(
        &file,
        &[ranged(at(0, PAST_LINE_END), at(0, PAST_LINE_END), "!")]
    ));
    assert_eq!(
        buffers.read(&file)?.bytes,
        AFTER_CLAMPED_INSERT.as_bytes(),
        "a column past the line end clamps to the line end"
    );
    assert!(buffers.edit(&file, &[whole(WHOLE_REPLACEMENT)]));
    assert_eq!(buffers.read(&file)?.bytes, WHOLE_REPLACEMENT.as_bytes());
    assert_eq!(
        fs::read_to_string(&file)?,
        ON_DISK,
        "no edit touched the disk"
    );

    assert!(buffers.close(&file));
    assert!(
        !buffers.close(&file),
        "closing twice reports the buffer already gone"
    );
    assert_eq!(buffers.paths().count(), 0);
    let after_close = buffers.read(&file)?;
    assert_eq!(
        after_close.bytes,
        ON_DISK.as_bytes(),
        "a closed path reads the disk again"
    );
    assert!(!after_close.unsaved);
    Ok(())
}

/// The three edits that turn `OPENED` into `AFTER_ORDERED_EDITS`: replace
/// `cd` with `X`, insert `-` after the emoji (column 3 — one unit for `X`,
/// two for the emoji), then delete the first line including its newline.
fn ordered_edits() -> Vec<BufferEdit> {
    vec![
        ranged(at(1, 0), at(1, 2), "X"),
        ranged(at(1, 3), at(1, 3), "-"),
        ranged(at(0, 0), at(1, 0), DELETED),
    ]
}

/// A zero-indexed line / UTF-16 column position.
const fn at(line: u32, character: u32) -> LineCol {
    LineCol { line, character }
}

/// An edit replacing `start..end` with `text`.
fn ranged(start: LineCol, end: LineCol, text: &str) -> BufferEdit {
    BufferEdit {
        range: Some((start, end)),
        text: text.to_owned(),
    }
}

/// An edit replacing the whole buffer with `text`.
fn whole(text: &str) -> BufferEdit {
    BufferEdit {
        range: None,
        text: text.to_owned(),
    }
}

/// True when `report` lists a cluster with `cluster_id`.
fn has_cluster(report: &Report, cluster_id: &str) -> bool {
    report
        .clusters
        .iter()
        .any(|cluster| cluster.id == cluster_id)
}

/// The cluster whose occurrences cover both `first` and `second`.
fn cluster_spanning(report: &Report, first: &str, second: &str) -> Result<ReportCluster> {
    report
        .clusters
        .iter()
        .find(|cluster| {
            let touches = |file: &str| {
                cluster
                    .occurrences
                    .iter()
                    .any(|occurrence| occurrence.path.ends_with(file))
            };
            touches(first) && touches(second)
        })
        .cloned()
        .ok_or_else(|| anyhow!("no cluster spans {first} and {second}: {report:?}"))
}
