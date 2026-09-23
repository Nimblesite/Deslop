//! Where a snippet's code already lives in the workspace
//! ([MCP-TOOL-FINDSIMILAR-EXISTING]): the fingerprints that carry a
//! snippet's normalised subtrees, as ranges and as published occurrences.
//! Split from the parent module, which owns the session's lifecycle.

use std::path::PathBuf;

use super::PipelineSession;
use crate::{
    ast::ByteRange,
    cluster_filters::ParseCache,
    fingerprint::Fingerprint,
    report::ReportOccurrence,
    report_hide::{MemberFile, ReportHide},
    report_render::{located_occurrence, LineIndex, Located},
};

impl PipelineSession {
    /// Resolves subtree digests to the live occurrences that carry
    /// them — the workspace path and byte range of every fingerprint
    /// whose normalised-subtree hash is in `hashes`
    /// ([PIPELINE-FINGERPRINT-MERKLE]).
    ///
    /// The join `find_similar` runs for snippet input: a snippet has no
    /// workspace identity, so it can only be located by content. The
    /// public cluster id is *not* content alone ([PIPELINE-DETERMINISM],
    /// gh #430 — it mixes the members' paths so same-shape findings in
    /// different files never share one), so a caller that matched
    /// `encode_short_id(hash) == cluster.id` returned empty for every
    /// snippet the moment ids stopped being bare member digests. The
    /// occurrence ranges this returns are joined against the report
    /// through [`Report`] range lookups, which is the same join the
    /// open-range variant of `find_similar` uses.
    #[must_use]
    pub fn subtree_occurrences(&self, hashes: &[[u8; 32]]) -> Vec<(PathBuf, ByteRange)> {
        self.carrying(hashes)
            .filter_map(|found| {
                self.path_for(found.file_id)
                    .map(|path| (path.to_path_buf(), found.byte_range))
            })
            .collect()
    }

    /// [MCP-TOOL-FINDSIMILAR-EXISTING] Every outermost place whose
    /// normalised code equals one of `hashes`, as published occurrences:
    /// workspace-relative path, bytes, lines and the report's `hidden`
    /// decision. A match strictly inside another match of the same file
    /// is the same place read at a smaller scale, so only the outermost
    /// is listed.
    #[must_use]
    pub fn existing_occurrences(&self, hashes: &[[u8; 32]]) -> Vec<ReportOccurrence> {
        let parse_cache = ParseCache::new();
        let hide = ReportHide::new(&self.exclusion, &self.parsers, &parse_cache);
        let carrying: Vec<&Fingerprint> = self.carrying(hashes).collect();
        let mut found: Vec<ReportOccurrence> = outermost(&carrying)
            .into_iter()
            .filter_map(|member| self.published(member, &hide))
            .collect();
        found.sort_by(|left, right| {
            (&left.path, left.start_byte).cmp(&(&right.path, right.start_byte))
        });
        found
    }

    /// The live fingerprints whose normalised-subtree hash is in `hashes`.
    fn carrying<'s>(&'s self, hashes: &'s [[u8; 32]]) -> impl Iterator<Item = &'s Fingerprint> {
        self.store
            .fingerprints()
            .iter()
            .filter(move |found| hashes.contains(&found.hash))
    }

    /// `member` as a published occurrence, hidden exactly when the report
    /// would hide its file.
    fn published(&self, member: &Fingerprint, hide: &ReportHide<'_>) -> Option<ReportOccurrence> {
        let absolute = self.path_for(member.file_id)?;
        let source = self.sources.get(&member.file_id)?;
        let hidden = hide.hides(&MemberFile {
            id: member.file_id,
            path: Some(absolute),
            language: self
                .file_languages
                .get(&member.file_id)
                .copied()
                .unwrap_or(""),
            source: Some(source),
        });
        let line_index = LineIndex::new(source);
        let located = Located {
            absolute: Some(absolute),
            range: member.byte_range,
            line_index: Some(&line_index),
            hidden,
        };
        Some(located_occurrence(&located, &self.root))
    }
}

/// The matches no other match of the same file strictly encloses, each
/// range once.
fn outermost<'f>(matches: &[&'f Fingerprint]) -> Vec<&'f Fingerprint> {
    let mut kept: Vec<&Fingerprint> = Vec::new();
    for member in matches {
        let enclosed = matches.iter().any(|other| {
            other.file_id == member.file_id && other.byte_range.strictly_encloses(member.byte_range)
        });
        let repeated = kept
            .iter()
            .any(|seen| seen.file_id == member.file_id && seen.byte_range == member.byte_range);
        if !enclosed && !repeated {
            kept.push(member);
        }
    }
    kept
}
