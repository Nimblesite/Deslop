//! Exact-fingerprint frontier reuse ([FUSED-CONTENT-GATE-MEMO]).

use std::{collections::HashMap, hash::BuildHasher, sync::Arc};

use super::{
    frontier::member_content, has_hard_contradiction, measure_core_contents, pair_evidence,
    ContentEvidence, MemberContent, PairScope,
};
use crate::{ast::NormalizedNode, fingerprint::Fingerprint, state::FileId};

/// Reuses the parsed content frontier of one fingerprint during a render.
#[derive(Default)]
pub(crate) struct ContentMeasurer {
    /// Exact endpoint identity includes file, range, shape and node mass.
    endpoints: HashMap<Fingerprint, Option<Arc<MemberContent>>>,
    /// Cache reads observed by deterministic work-count tests.
    #[cfg(test)]
    hits: usize,
}

/// Bounds retained frontiers while still admitting later endpoints to the memo.
const CONTENT_FRONTIER_MEMO_MAX: usize = 1_024;

impl ContentMeasurer {
    /// Returns the endpoint's content frontier when its tree and bytes resolve.
    pub(super) fn member<S: BuildHasher, L: BuildHasher>(
        &mut self,
        fingerprint: &Fingerprint,
        trees: &HashMap<FileId, &NormalizedNode>,
        sources: &HashMap<FileId, Vec<u8>, S>,
        languages: &HashMap<FileId, &'static str, L>,
    ) -> Option<Arc<MemberContent>> {
        if let Some(cached) = self.endpoints.get(fingerprint) {
            #[cfg(test)]
            {
                self.hits = self.hits.saturating_add(1);
            }
            return cached.clone();
        }
        let measured = member_content(fingerprint, trees, sources, languages).map(Arc::new);
        if self.endpoints.len() == CONTENT_FRONTIER_MEMO_MAX {
            self.endpoints.clear();
        }
        let _ = self.endpoints.insert(fingerprint.clone(), measured.clone());
        measured
    }

    /// Cache reads completed without rebuilding a frontier.
    #[cfg(test)]
    pub(crate) const fn hits(&self) -> usize {
        self.hits
    }

    /// Resolves two endpoint frontiers once for their whole and core evidence.
    pub(crate) fn pair<S: BuildHasher, L: BuildHasher>(
        &mut self,
        endpoints: (&Fingerprint, &Fingerprint),
        trees: &HashMap<FileId, &NormalizedNode>,
        sources: &HashMap<FileId, Vec<u8>, S>,
        languages: &HashMap<FileId, &'static str, L>,
    ) -> ContentPair {
        ContentPair {
            left: self.member(endpoints.0, trees, sources, languages),
            right: self.member(endpoints.1, trees, sources, languages),
        }
    }
}

/// Cached whole frontiers of one pair, reused by the aligned-core gate.
pub(crate) struct ContentPair {
    /// First whole-endpoint frontier, when resolvable.
    left: Option<Arc<MemberContent>>,
    /// Second whole-endpoint frontier, when resolvable.
    right: Option<Arc<MemberContent>>,
}

impl ContentPair {
    /// Whether both endpoint frontiers resolved. This is the same
    /// measured bit whole-pair evidence would report, without running
    /// agreement, rename, or contradiction calculations
    /// ([FUSED-CONTENT-GATE-MEMO]).
    pub(crate) fn resolved(&self) -> bool {
        self.left.is_some() && self.right.is_some()
    }

    /// [FUSED-SHARED-SUBTREE-HARD-CONTENT] Whether every aligned core
    /// must fail: an unresolved endpoint cannot yield measured core
    /// evidence, and a hard whole-endpoint contradiction is carried
    /// unchanged into every core verdict.
    pub(crate) fn rejects_every_core<S: BuildHasher>(
        &self,
        sources: &HashMap<FileId, Vec<u8>, S>,
    ) -> bool {
        match self.whole_refs() {
            Some(whole) => has_hard_contradiction(whole, sources),
            None => true,
        }
    }

    /// Measures the whole pair with the canonical content evidence algebra.
    pub(crate) fn whole<S: BuildHasher>(
        &self,
        sources: &HashMap<FileId, Vec<u8>, S>,
        scope: PairScope,
    ) -> ContentEvidence {
        pair_evidence(self.whole_refs(), sources, scope)
    }

    /// Measures shared aligned code with the same resolved whole frontiers.
    pub(crate) fn core<S: BuildHasher>(
        &self,
        core: &[(Fingerprint, Fingerprint)],
        sources: &HashMap<FileId, Vec<u8>, S>,
        scope: PairScope,
    ) -> ContentEvidence {
        self.whole_refs()
            .map_or_else(ContentEvidence::unmeasured, |whole| {
                measure_core_contents(core, whole, sources, scope)
            })
    }

    /// Both frontiers, or no measured pair if either endpoint is unresolved.
    fn whole_refs(&self) -> Option<(&MemberContent, &MemberContent)> {
        self.left.as_deref().zip(self.right.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use super::{ContentMeasurer, CONTENT_FRONTIER_MEMO_MAX};
    use crate::{
        ast::{ByteRange, NormalizedNode},
        content::{measure_aligned_core, measure_pair_content_indexed, ContentEvidence, PairScope},
        fingerprint::{collect_fingerprints, Fingerprint},
        lang::LanguageParser,
        registry_fixtures::rust_pair_ids,
        state::FileId,
    };

    const SOURCE: &str = "fn accumulate(bound: u32) -> u32 { if bound == 0 { return 0; } let mut running = 0; for step in 0..bound { running = running + step; } running } fn aggregate(limit: u32) -> u32 { if limit == 0 { return 0; } let mut total = 0; for cursor in 0..limit { total = total + cursor; total = total + 2; } total }";
    const EVERY_SUBTREE: usize = 1;
    const LANGUAGE: &str = "rust";
    const UNRESOLVABLE_OFFSET: usize = usize::MAX;

    // [FUSED-CONTENT-GATE-MEMO] A cheap resolution bit must mean exactly
    // what the full content algebra's `measured` bit means.
    #[test]
    fn resolved_matches_whole_evidence_for_present_and_missing_endpoints() -> Result<(), String> {
        let (tree, fingerprints) = fixture()?;
        let endpoint = fingerprints.first().ok_or("endpoint exists")?;
        let trees = HashMap::from([(tree.file_id, &tree)]);
        let (sources, languages) = inputs(tree.file_id);
        let mut measurer = ContentMeasurer::default();
        let scope = PairScope {
            same_file: true,
            interior: false,
            core: false,
        };
        let resolved = measurer.pair((endpoint, endpoint), &trees, &sources, &languages);
        assert!(resolved.resolved(), "a present endpoint resolves");
        assert_eq!(
            resolved.resolved(),
            resolved.whole(&sources, scope).measured
        );
        let mut missing = endpoint.clone();
        missing.byte_range = ByteRange {
            start: UNRESOLVABLE_OFFSET,
            end: UNRESOLVABLE_OFFSET,
        };
        let unresolved = measurer.pair((endpoint, &missing), &trees, &sources, &languages);
        assert!(
            !unresolved.resolved(),
            "an invalid byte range cannot resolve"
        );
        assert_eq!(
            unresolved.resolved(),
            unresolved.whole(&sources, scope).measured
        );
        Ok(())
    }

    fn fixture() -> Result<(NormalizedNode, Vec<Fingerprint>), String> {
        let file_id = rust_pair_ids().0;
        let tree = crate::lang::rust_lang::RustParser
            .parse_and_normalize(SOURCE.as_bytes(), file_id)
            .map_err(|error| error.to_string())?;
        let fingerprints = collect_fingerprints(&tree, EVERY_SUBTREE);
        Ok((tree, fingerprints))
    }

    fn inputs(file_id: FileId) -> (HashMap<FileId, Vec<u8>>, HashMap<FileId, &'static str>) {
        (
            HashMap::from([(file_id, SOURCE.as_bytes().to_vec())]),
            HashMap::from([(file_id, LANGUAGE)]),
        )
    }

    fn same_range_pair(fingerprints: &[Fingerprint]) -> Option<(&Fingerprint, &Fingerprint)> {
        fingerprints.iter().find_map(|first| {
            fingerprints
                .iter()
                .find(|second| first.byte_range == second.byte_range && first.hash != second.hash)
                .map(|second| (first, second))
        })
    }

    // [FUSED-CONTENT-GATE-MEMO] One byte span can name two normalised
    // structures. Both need their own reusable frontier.
    #[test]
    fn same_range_fingerprints_keep_distinct_cached_frontiers() -> Result<(), String> {
        let (tree, fingerprints) = fixture()?;
        let (wrapper, child) = same_range_pair(&fingerprints).ok_or("same-range pair exists")?;
        let trees = HashMap::from([(tree.file_id, &tree)]);
        let (sources, languages) = inputs(tree.file_id);
        let mut measurer = ContentMeasurer::default();
        let first = measurer
            .member(wrapper, &trees, &sources, &languages)
            .ok_or("wrapper resolves")?;
        let second = measurer
            .member(child, &trees, &sources, &languages)
            .ok_or("child resolves")?;
        let repeated = measurer
            .member(wrapper, &trees, &sources, &languages)
            .ok_or("wrapper reuses")?;
        assert_eq!(first.shape, wrapper.hash);
        assert_eq!(second.shape, child.hash);
        assert!(
            !Arc::ptr_eq(&first, &second),
            "different fingerprints need distinct frontiers"
        );
        assert!(
            Arc::ptr_eq(&first, &repeated),
            "repeat endpoint should reuse its frontier"
        );
        Ok(())
    }

    // [FUSED-CONTENT-GATE-MEMO] A full cache must still accept later
    // endpoints rather than permanently recomputing the long tail.
    #[test]
    fn later_frontiers_are_reused_after_cache_rotation() -> Result<(), String> {
        let (tree, fingerprints) = fixture()?;
        let anchor = fingerprints.first().ok_or("anchor exists")?;
        let trees = HashMap::from([(tree.file_id, &tree)]);
        let (sources, languages) = inputs(tree.file_id);
        let mut measurer = ContentMeasurer::default();
        for ordinal in 0..CONTENT_FRONTIER_MEMO_MAX {
            let mut missing = anchor.clone();
            missing.hash[..8].copy_from_slice(&ordinal.to_le_bytes());
            let _ = measurer.member(&missing, &trees, &sources, &languages);
        }
        let first = measurer
            .member(anchor, &trees, &sources, &languages)
            .ok_or("first resolves")?;
        let second = measurer
            .member(anchor, &trees, &sources, &languages)
            .ok_or("repeat resolves")?;
        assert!(
            Arc::ptr_eq(&first, &second),
            "later endpoint must be cached"
        );
        Ok(())
    }

    // [FUSED-CONTENT-GATE-MEMO] Caching changes neither the whole-content
    // evidence nor the aligned-core verdict for the same exact endpoint.
    #[test]
    fn cached_pair_matches_uncached_whole_and_core_evidence() -> Result<(), String> {
        let (tree, fingerprints) = fixture()?;
        let endpoint = fingerprints.first().ok_or("endpoint exists")?;
        let trees = HashMap::from([(tree.file_id, &tree)]);
        let (sources, languages) = inputs(tree.file_id);
        let mut measurer = ContentMeasurer::default();
        let pair = measurer.pair((endpoint, endpoint), &trees, &sources, &languages);
        let whole_scope = PairScope {
            same_file: true,
            interior: false,
            core: false,
        };
        let core_scope = PairScope {
            core: true,
            ..whole_scope
        };
        let core = [(endpoint.clone(), endpoint.clone())];
        let uncached_whole =
            measure_pair_content_indexed(endpoint, endpoint, &trees, &sources, &languages, false);
        let uncached_core = measure_aligned_core(
            (endpoint, endpoint),
            &core,
            &trees,
            &sources,
            &languages,
            core_scope,
        );
        assert_evidence_eq(pair.whole(&sources, whole_scope), uncached_whole);
        assert_evidence_eq(pair.core(&core, &sources, core_scope), uncached_core);
        Ok(())
    }

    fn assert_evidence_eq(actual: ContentEvidence, expected: ContentEvidence) {
        assert_eq!(actual.agreement.to_bits(), expected.agreement.to_bits());
        assert_eq!(
            actual.rename_consistency.to_bits(),
            expected.rename_consistency.to_bits()
        );
        assert_eq!(
            actual.literal_fraction.to_bits(),
            expected.literal_fraction.to_bits()
        );
        assert_eq!(actual.consistent_rename, expected.consistent_rename);
        assert_eq!(actual.measured, expected.measured);
        assert_eq!(actual.contradiction, expected.contradiction);
    }
}
