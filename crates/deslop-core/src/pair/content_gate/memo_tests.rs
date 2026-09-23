//! [FUSED-CONTENT-GATE-MEMO] One gate pass resolves repeated endpoints once.

use std::{collections::HashMap, path::PathBuf};

use super::*;
use crate::{
    fingerprint::collect_fingerprints,
    lang::{rust_lang::RustParser, LanguageParser},
    state::FileRegistry,
};

const SOURCE: &str = "fn calculate(limit: u32) -> u32 { let mut sum = 0; for step in 0..limit { sum += step; } sum }";
const CHANGED_OPERATOR: &str = "fn calculate(limit: u32) -> u32 { let mut sum = 0; for step in 0..limit { sum -= step; } sum }";
const PATHS: [&str; 3] = ["first.rs", "second.rs", "third.rs"];
const EVERY_SUBTREE: usize = 1;
const RUST: &str = "rust";
const FIRST: usize = 0;
const SECOND: usize = 1;
const THIRD: usize = 2;
const EXACT: f64 = 1.0;
const NO_EMBEDDING: f64 = 0.0;
const EXPECTED_PAIRS: usize = 1;
const EXPECTED_REUSE: usize = 3;
const EXACT_SCORE: super::super::PairScore = super::super::PairScore {
    structural: EXACT,
    token_jaccard: EXACT,
    embedding_cos: NO_EMBEDDING,
};

type GateCorpus = (
    Vec<NormalizedNode>,
    Vec<Fingerprint>,
    HashMap<FileId, Vec<u8>>,
    HashMap<FileId, &'static str>,
);

fn parsed_root(file_id: FileId, source: &str) -> Result<(NormalizedNode, Fingerprint), String> {
    let tree = RustParser
        .parse_and_normalize(source.as_bytes(), file_id)
        .map_err(|error| error.to_string())?;
    let found = collect_fingerprints(&tree, EVERY_SUBTREE)
        .into_iter()
        .max_by_key(|item| item.node_count)
        .ok_or("Rust gate fixture must have a root fingerprint")?;
    Ok((tree, found))
}

fn corpus() -> Result<GateCorpus, String> {
    let mut registry = FileRegistry::new();
    let mut trees = Vec::new();
    let mut fingerprints = Vec::new();
    let mut sources = HashMap::new();
    let mut languages = HashMap::new();
    for (index, path) in PATHS.into_iter().enumerate() {
        let source = if index == THIRD {
            CHANGED_OPERATOR
        } else {
            SOURCE
        };
        let file_id = registry.register(PathBuf::from(path));
        let (tree, found) = parsed_root(file_id, source)?;
        trees.push(tree);
        fingerprints.push(found);
        let _previous = sources.insert(file_id, source.as_bytes().to_vec());
        let _previous = languages.insert(file_id, RUST);
    }
    Ok((trees, fingerprints, sources, languages))
}

fn candidate(
    left: usize,
    right: usize,
    fingerprints: &[Fingerprint],
) -> Result<CandidatePair, String> {
    let left_nodes = fingerprints
        .get(left)
        .ok_or("first gate fingerprint is missing")?
        .node_count;
    let right_nodes = fingerprints
        .get(right)
        .ok_or("second gate fingerprint is missing")?
        .node_count;
    Ok(CandidatePair {
        left,
        right,
        endpoint_node_counts: (left_nodes, right_nodes),
        lsh_only_node_floor: SHARED_SUBTREE_MIN_NODE_COUNT,
        lsh_only_min_jaccard: LSH_ONLY_MIN_JACCARD,
        fused_min_score: FUSED_THRESHOLD,
        shared_subtree_overlap: EXACT,
        verified_async_core: false,
        score: EXACT_SCORE,
    })
}

#[test]
fn gate_reuses_frontiers_across_exact_content_edges() -> Result<(), String> {
    let (trees, fingerprints, sources, languages) = corpus()?;
    let mut pairs = vec![
        candidate(FIRST, SECOND, &fingerprints)?,
        candidate(FIRST, THIRD, &fingerprints)?,
        candidate(SECOND, THIRD, &fingerprints)?,
    ];
    let mut content = ContentMeasurer::default();
    apply_pair_content_gate_with_content(
        &mut pairs,
        &fingerprints,
        &trees,
        &sources,
        &languages,
        &ParseCache::new(),
        &mut content,
    );
    assert_eq!(pairs.len(), EXPECTED_PAIRS);
    assert_eq!(content.hits(), EXPECTED_REUSE);
    assert_eq!(
        pairs
            .iter()
            .map(|pair| (pair.left, pair.right))
            .collect::<Vec<_>>(),
        [(FIRST, SECOND)]
    );
    Ok(())
}
