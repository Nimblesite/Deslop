//! Size and allocation bounds for blob loading
//! ([PIPELINE-INCREMENTAL-INTEGRITY]).
//!
//! The digest in [`super`] decides whether a blob may be *trusted*;
//! these bounds decide how much memory it may cost to find out. They
//! matter only for a payload whose digest verifies — ordinary corruption
//! is rejected before any of this is reached — and every one of them
//! degrades to a plain miss that re-parses from source, never to an
//! abort.

use std::{
    fs,
    io::{self, Read},
    path::Path,
};

use super::invalid_data;

/// Upper bound on a blob file, enforced before the bytes are read so a
/// corrupt or hostile store entry cannot drive an arbitrarily large
/// allocation out of its file length alone. Far above any blob a real
/// source file produces — the whole store for a 758-file corpus measures
/// well under this.
pub(in crate::fpcache) const MAX_BLOB_BYTES: u64 = 256 * 1024 * 1024;

/// Ceiling on the nodes one blob may decode into, whatever its declared
/// counts say. The byte bound alone is not enough: every encoded node
/// costs 24 bytes minimum on disk but a resident `NormalizedNode` —
/// interned kind pointer, two offsets, a `FileId`, and a child `Vec` — is
/// several times that, so a digest-valid blob at the byte ceiling could
/// still multiply into an allocation many times its file size. Four
/// million nodes is orders of magnitude past any real source file (the
/// whole `deslop-core` tree decodes ~29k fingerprints' worth) and
/// exceeding it costs only a miss: the file re-parses from source and the
/// blob is rewritten ([PIPELINE-INCREMENTAL-INVALIDATION]).
pub(super) const MAX_DECODED_NODES: usize = 4_000_000;

/// Reads the blob at `path` under a hard [`MAX_BLOB_BYTES`] ceiling, or
/// `None` for a missing, unreadable, or oversized file — every one a
/// plain miss.
///
/// One handle does all three jobs — measure, bound, read. Sizing from
/// `fs::metadata` and then calling `fs::read` measures one file and
/// allocates for another: a second binary sharing the store
/// ([PIPELINE-INCREMENTAL-RETENTION]) can replace or extend the entry in
/// between, and the ceiling would then apply only to the stale
/// measurement.
///
/// The read is capped at `len + 1` — the measured length plus a single
/// sentinel byte — not at [`MAX_BLOB_BYTES`]. Both halves matter. The
/// sentinel is what makes growth *observable*: reading exactly `len`
/// bytes cannot distinguish "the whole blob" from "a truncated view of a
/// file that has since grown", and the truncated view is a prefix whose
/// trailing bytes were silently dropped. Capping at `len + 1` rather than
/// the global ceiling is what keeps the allocation exact: the buffer is
/// reserved once, fallibly, at `len + 1`, and the capped read can never
/// grow it — whereas reading toward [`MAX_BLOB_BYTES`] would let a file
/// that grew by megabytes drive `Vec` reallocation far past the
/// reservation on its way to being rejected.
pub(in crate::fpcache) fn read_bounded(path: &Path) -> Option<Vec<u8>> {
    let file = fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    if len > MAX_BLOB_BYTES {
        tracing::warn!(
            path = %path.display(),
            len,
            "fingerprint cache blob exceeds the size bound — treating as miss",
        );
        return None;
    }
    let capacity = usize::try_from(len).ok()?.checked_add(1)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity).ok()?;
    let _read = file
        .take(len.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != len {
        tracing::warn!(
            path = %path.display(),
            expected = len,
            read = bytes.len(),
            "fingerprint cache blob changed size mid-read — treating as miss",
        );
        return None;
    }
    Some(bytes)
}

/// The node allowance for one blob's decode, spent as the tree is walked.
/// Global to the decode, so it bounds the whole tree rather than any one
/// branch — the depth guard (`MAX_AST_DEPTH`) bounds a single path, and a
/// wide-but-shallow tree evades it entirely.
pub(super) struct NodeBudget {
    /// Nodes still permitted before the decode is refused.
    remaining: usize,
}

impl NodeBudget {
    /// A fresh allowance for one blob.
    pub(super) const fn new() -> Self {
        Self {
            remaining: MAX_DECODED_NODES,
        }
    }

    /// Claims one node plus the `children` slots that must follow it, so
    /// an absurd child count is refused *before* its `Vec` is reserved
    /// rather than after the allocation it would drive.
    pub(super) fn claim(&mut self, children: usize) -> io::Result<()> {
        let after_self = self
            .remaining
            .checked_sub(1)
            .ok_or_else(|| invalid_data("cached AST exceeds the decoded-node budget"))?;
        if children > after_self {
            return Err(invalid_data(
                "cached AST child count exceeds the decoded-node budget",
            ));
        }
        self.remaining = after_self;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // [PIPELINE-INCREMENTAL-INTEGRITY] The bounded read allocates exactly
    // once, for exactly the measured length plus the growth sentinel, and
    // returns exactly the file's bytes. The capacity assertion is the
    // point: a read capped at the *global* ceiling instead of `len + 1`
    // would let a file that grew by megabytes reallocate the buffer far
    // past the reservation on its way to being rejected.
    #[test]
    fn the_bounded_read_returns_the_whole_file_without_growing_its_buffer() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("blob.bin");
        let contents = vec![7_u8; 4_096];
        fs::write(&path, &contents)?;

        let read = read_bounded(&path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "admissible blob refused"))?;

        assert_eq!(
            read, contents,
            "the bounded read must return the file's bytes verbatim"
        );
        assert!(
            read.capacity() <= contents.len().saturating_add(1),
            "the buffer must be reserved once at len + 1 and never grown, got \
             capacity {} for a {}-byte file",
            read.capacity(),
            contents.len()
        );
        assert!(
            read_bounded(&tmp.path().join("absent.bin")).is_none(),
            "a missing blob is a plain miss, never an error"
        );
        Ok(())
    }

    // [PIPELINE-INCREMENTAL-INTEGRITY] The decoded-node budget is global
    // to one blob: it bounds a wide-but-shallow tree, which `MAX_AST_DEPTH`
    // cannot see, and it is claimed *including* the child slots that follow
    // a node so an absurd child count is refused before its `Vec` is
    // reserved.
    #[test]
    fn the_decoded_node_budget_is_global_and_refuses_child_counts_before_reserving() {
        assert!(
            NodeBudget::new().claim(MAX_DECODED_NODES).is_err(),
            "a child count that cannot fit in what remains must be refused — \
             this is the check that runs before the child list is reserved"
        );
        assert!(
            NodeBudget::new()
                .claim(MAX_DECODED_NODES.saturating_sub(1))
                .is_ok(),
            "a child count that exactly fits the remaining allowance is \
             admitted, so the guard rejects only the impossible"
        );
        let mut budget = NodeBudget::new();
        assert!(
            (0..MAX_DECODED_NODES).all(|_| budget.claim(0).is_ok()),
            "every node inside the allowance must be admitted"
        );
        assert!(
            budget.claim(0).is_err(),
            "the allowance is spent, so even a childless node is refused — the \
             budget spans the whole recursion, not one node or one branch"
        );
    }
}
