//! File-backed signature storage ([PERF-FLUTTER-TODO-MEMORY]).
//!
//! A corpus-scale cold build produces millions of `MinHash`
//! signatures — over 3 GiB for the Flutter corpus — and a resident
//! population busts any per-repo memory ceiling sized to the
//! repository. The arena stores them in one append-only file: workers
//! reserve contiguous blocks through an atomic cursor and write them
//! with positioned writes, readers fetch single signatures with
//! positioned reads. No `unsafe`, no memory mapping — the kernel page
//! cache does the warming.
//!
//! Blocks are immutable once written, which is what makes the live
//! path affordable: the band-tag index computed over an arena block
//! stays valid forever, and incremental edits layer small resident
//! segments on top (see [`crate::pipeline::session::store`]).

use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use crate::lsh::{Signature, SignatureLookup, SIGNATURE_LEN};

/// One reserved, immutable run of signatures in the arena file:
/// a byte offset and a signature count.
#[derive(Debug, Clone, Copy)]
pub struct ArenaBlock {
    /// Byte offset of the block's first signature.
    pub offset: u64,
    /// Signatures in the block.
    pub len: usize,
}

/// The append-only signature file plus its block table.
#[derive(Debug)]
pub struct SignatureArena {
    /// Backing file; positioned reads and writes only, so clones and
    /// cross-thread sharing never race a cursor.
    file: File,
    /// Where the file lives — removed when the arena drops.
    path: PathBuf,
    /// Next free byte; reserved by `fetch_add`.
    cursor: AtomicU64,
    /// Blocks in write order, matched to segment metadata by the
    /// store. Guarded because `Vec` growth is not atomic.
    blocks: Mutex<Vec<ArenaBlock>>,
}

impl Drop for SignatureArena {
    fn drop(&mut self) {
        let _ignored = std::fs::remove_file(&self.path);
    }
}

/// Byte size of one signature.
const SIGNATURE_BYTES: usize = SIGNATURE_LEN * std::mem::size_of::<u64>();

/// Byte size of one signature lane.
const LANE_BYTES: usize = std::mem::size_of::<u64>();

impl SignatureArena {
    /// Creates the arena as a fresh file under `directory`.
    ///
    /// # Errors
    ///
    /// Returns the filesystem error when the file cannot be created.
    pub fn create(directory: &Path) -> io::Result<Self> {
        let path = directory.join("signature-arena.bin");
        // Read **and** write: `File::create` is write-only on Unix,
        // and positioned reads against it fail with `EBADF`.
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        Ok(Self {
            file,
            path,
            cursor: AtomicU64::new(0),
            blocks: Mutex::new(Vec::new()),
        })
    }

    /// Reserves room for `count` signatures and returns the block to
    /// write them into. Reservation is a single atomic add, so any
    /// number of workers can reserve concurrently.
    #[must_use]
    pub fn reserve(&self, count: usize) -> ArenaBlock {
        let bytes = u64::try_from(count.saturating_mul(SIGNATURE_BYTES)).unwrap_or(u64::MAX);
        let offset = self.cursor.fetch_add(bytes, Ordering::Relaxed);
        ArenaBlock {
            offset,
            len: count,
        }
    }

    /// Writes `signatures` into the reserved `block` with positioned
    /// writes. The caller must pass the block [`Self::reserve`]
    /// returned for exactly this run.
    ///
    /// # Errors
    ///
    /// Returns the filesystem error when a positioned write fails.
    pub fn write_block(&self, block: ArenaBlock, signatures: &[Signature]) -> io::Result<()> {
        debug_assert_eq!(block.len, signatures.len());
        // One reused staging buffer: signatures are `u64` arrays, the
        // file API is bytes, and a per-signature allocation would put
        // millions of 1 KiB vectors through the allocator.
        let mut staging = Vec::with_capacity(SIGNATURE_BYTES);
        let mut offset = block.offset;
        for signature in signatures {
            staging.clear();
            staging.extend(signature.iter().flat_map(|lane| lane.to_ne_bytes()));
            write_at(&self.file, &staging, offset)?;
            offset = offset.saturating_add(u64::try_from(SIGNATURE_BYTES).unwrap_or(u64::MAX));
        }
        Ok(())
    }

    /// Records `block` in the block table. Called once per block, in
    /// the order the owning segments will be read.
    ///
    /// # Errors
    ///
    /// Returns the lock-poisoning error when a writer panicked while
    /// holding the table lock.
    pub fn record(&self, block: ArenaBlock) -> Result<(), ArenaError> {
        self.blocks
            .lock()
            .map_err(|_| ArenaError::PoisonedTable)?
            .push(block);
        Ok(())
    }

    /// Reads the signature at `byte_offset` into `out`.
    ///
    /// # Errors
    ///
    /// Returns the filesystem error when a positioned read fails or
    /// comes up short.
    pub fn read_at(&self, byte_offset: u64, out: &mut Signature) -> Result<(), ArenaError> {
        let mut staging = vec![0_u8; SIGNATURE_BYTES];
        read_exact_at(&self.file, &mut staging, byte_offset)?;
        for (lane, chunk) in out
            .iter_mut()
            .zip(staging.chunks_exact(LANE_BYTES).map(<[u8; LANE_BYTES]>::try_from))
        {
            if let Ok(bytes) = chunk {
                *lane = u64::from_ne_bytes(bytes);
            }
        }
        Ok(())
    }

    /// The arena's backing path, for diagnostics.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// A positioned read that fills `buffer` exactly, retrying short reads.
fn read_exact_at(file: &File, buffer: &mut [u8], mut offset: u64) -> io::Result<()> {
    let mut filled = 0_usize;
    while filled < buffer.len() {
        let target = buffer
            .get_mut(filled..)
            .unwrap_or(&mut []);
        let read = read_at_os(file, target, offset)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "signature arena ended mid-signature",
            ));
        }
        filled = filled.saturating_add(read);
        offset = offset.saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
    }
    Ok(())
}

/// One positioned read; returns how many bytes moved.
fn read_at_os(file: &File, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.read_at(buffer, offset)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        file.seek_read(buffer, offset)
    }
}

/// One positioned write; returns how many bytes moved.
fn write_at(file: &File, buffer: &[u8], offset: u64) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.write_all_at(buffer, offset)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let mut moved = 0_usize;
        while moved < buffer.len() {
            let target = buffer.get(moved..).unwrap_or(&[]);
            let extra = u64::try_from(moved).unwrap_or(u64::MAX);
            let written = file.seek_write(target, offset.saturating_add(extra))?;
            if written == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "signature arena write made no progress",
                ));
            }
            moved = moved.saturating_add(written);
        }
        Ok(())
    }
}

/// Arena failures: positioned-IO errors or a poisoned block table.
#[derive(Debug, thiserror::Error)]
pub enum ArenaError {
    /// The filesystem refused a positioned read or write.
    #[error("signature arena io: {0}")]
    Io(#[from] io::Error),
    /// A writer panicked while holding the block table lock.
    #[error("signature arena block table poisoned")]
    PoisonedTable,
}

/// A read-only view over arena blocks plus resident segments, in index
/// order — the [`SignatureLookup`] the pipeline measures through. Owns
/// its metadata so it can outlive the builder that assembled it.
#[derive(Debug, Clone)]
pub struct ArenaView {
    /// The shared arena, when any block is file-backed.
    arena: Option<Arc<SignatureArena>>,
    /// Segments in index order.
    segments: Vec<Segment>,
    /// Cumulative signature count before each segment (one longer than
    /// `segments`).
    starts: Vec<usize>,
}

/// One segment of the view: a slice of the arena file or resident
/// signatures.
#[derive(Debug, Clone)]
pub enum Segment {
    /// `block.len` signatures at the arena block.
    Arena(ArenaBlock),
    /// Resident signatures — live edits and small corpora.
    Ram(Vec<Signature>),
}

impl Segment {
    /// The segment's arena block, when it is file-backed.
    #[must_use]
    pub fn arena_block(&self) -> Option<ArenaBlock> {
        match self {
            Segment::Arena(block) => Some(*block),
            Segment::Ram(_) => None,
        }
    }

    /// Builds a file-backed segment.
    #[must_use]
    pub fn from_block(block: ArenaBlock) -> Self {
        Segment::Arena(block)
    }

    /// Builds a resident segment.
    #[must_use]
    pub fn resident(signatures: Vec<Signature>) -> Self {
        Segment::Ram(signatures)
    }
}

impl ArenaView {
    /// An empty view.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            arena: None,
            segments: Vec::new(),
            starts: vec![0],
        }
    }

    /// Builds a view from `segments`, attaching `arena` when any
    /// segment is file-backed.
    #[must_use]
    pub fn new(arena: Option<Arc<SignatureArena>>, segments: Vec<Segment>) -> Self {
        let mut starts = Vec::with_capacity(segments.len().saturating_add(1));
        let mut running = 0_usize;
        starts.push(0);
        for segment in &segments {
            running = running.saturating_add(segment.len());
            starts.push(running);
        }
        Self {
            arena,
            segments,
            starts,
        }
    }
}

impl Segment {
    /// Signatures in this segment.
    fn len(&self) -> usize {
        match self {
            Segment::Arena(block) => block.len,
            Segment::Ram(resident) => resident.len(),
        }
    }
}

impl SignatureLookup for ArenaView {
    fn kind(&self) -> &'static str {
        "ArenaView"
    }

    fn len(&self) -> usize {
        self.starts.last().copied().unwrap_or(0)
    }

    fn read_into(&self, index: usize, out: &mut Signature) -> bool {
        if index >= self.len() {
            return false;
        }
        let segment = self.starts.partition_point(|&start| start <= index).saturating_sub(1);
        let within = index.saturating_sub(self.starts.get(segment).copied().unwrap_or(0));
        match self.segments.get(segment) {
            Some(Segment::Ram(resident)) => match resident.get(within) {
                Some(signature) => {
                    out.copy_from_slice(signature);
                    true
                }
                None => false,
            },
            Some(Segment::Arena(block)) => {
                let Some(arena) = self.arena.as_ref() else {
                    return false;
                };
                let skip = u64::try_from(within.saturating_mul(SIGNATURE_BYTES)).unwrap_or(u64::MAX);
                arena.read_at(block.offset.saturating_add(skip), out).is_ok()
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    //! [PERF-FLUTTER-TODO-MEMORY] Arena pins: reserved blocks land at
    //! disjoint offsets, written signatures read back exactly, mixed
    //! arena/ram views stay positionally aligned, and reads past the
    //! population are refusals, not panics.

    use super::*;
    use crate::lsh::ZEROED_SIGNATURE;

    /// Two distinguishable signatures.
    fn signature(fill: u64) -> Signature {
        [fill; SIGNATURE_LEN]
    }

    /// A fresh arena directory under the temp dir, tagged per test.
    fn arena_dir(tag: &str) -> Result<PathBuf, ArenaError> {
        let directory = std::env::temp_dir().join(format!("arena-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&directory)
            .map_err(ArenaError::Io)
            .map(|()| directory)
    }

    /// Removes the arena directory; the arena file itself went with
    /// the drop.
    fn clean_up(directory: &Path) {
        let _ignored = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn reserved_blocks_never_overlap() -> Result<(), ArenaError> {
        let directory = arena_dir("reserve")?;
        let arena = SignatureArena::create(&directory)?;
        let first = arena.reserve(2);
        let second = arena.reserve(3);
        assert_eq!(first.offset, 0, "first block starts at zero");
        assert_eq!(
            second.offset,
            u64::try_from(2 * SIGNATURE_BYTES).unwrap_or(u64::MAX),
            "second block starts after the first's bytes"
        );
        drop(arena);
        clean_up(&directory);
        Ok(())
    }

    #[test]
    fn written_signatures_read_back_exactly() -> Result<(), ArenaError> {
        let directory = arena_dir("io")?;
        let arena = SignatureArena::create(&directory)?;
        let block = arena.reserve(2);
        arena.write_block(block, &[signature(7), signature(9)])?;
        let mut out = ZEROED_SIGNATURE;
        let second_offset = block
            .offset
            .saturating_add(u64::try_from(SIGNATURE_BYTES).unwrap_or(u64::MAX));
        arena.read_at(second_offset, &mut out)?;
        assert_eq!(out, signature(9), "second signature reads back exactly");
        drop(arena);
        clean_up(&directory);
        Ok(())
    }

    #[test]
    fn mixed_view_keeps_index_alignment() -> Result<(), ArenaError> {
        let directory = arena_dir("view")?;
        let arena = Arc::new(SignatureArena::create(&directory)?);
        let block = arena.reserve(1);
        arena.write_block(block, &[signature(5)])?;
        let view = ArenaView::new(
            Some(Arc::clone(&arena)),
            vec![
                Segment::Ram(vec![signature(1), signature(2)]),
                Segment::Arena(block),
                Segment::Ram(vec![signature(8)]),
            ],
        );
        assert_eq!(view.len(), 4, "two ram, one arena, one ram");
        let mut out = ZEROED_SIGNATURE;
        for (index, fill) in [(0, 1), (1, 2), (2, 5), (3, 8)] {
            assert!(view.read_into(index, &mut out), "index {index} exists");
            assert_eq!(out, signature(fill), "index {index} carries fill {fill}");
        }
        assert!(!view.read_into(4, &mut out), "past the end is a refusal");
        drop(arena);
        clean_up(&directory);
        Ok(())
    }
}
