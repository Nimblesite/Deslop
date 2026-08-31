//! Public report and explicit-pair wire-contract coverage.

use std::{fs, path::PathBuf};

use deslop_core::{
    ast::ByteRange,
    boilerplate::BoilerplateRange,
    config::ExclusionConfig,
    report::{PairClassification, PairComparison, PairEndpoint, PairEvidence, ReportOccurrence},
    report_boilerplate::build_boilerplate_hints,
    report_fixtures::{fixture_cluster, fixture_report},
    state::FileRegistry,
};

/// Requested live-wire occurrence budget.
const WIRE_OCCURRENCE_CAP: usize = 2;
/// Occurrences carried by the untruncated sample cluster.
const FULL_OCCURRENCE_COUNT: usize = 3;
/// Pair and presentation fields forbidden on a cluster.
const FORBIDDEN_CLUSTER_FIELDS: [&str; 11] = [
    "signals",
    "signal_source",
    "content",
    "evidence_verdict",
    "bucket",
    "category",
    "classification",
    "weight",
    "summary",
    "interpretation",
    "language",
];

#[test]
fn truncate_for_wire_caps_occurrences_without_inventing_a_pair() -> anyhow::Result<()> {
    let original = sample_report();
    let original_mass = original
        .clusters
        .first()
        .ok_or_else(|| anyhow::anyhow!("sample report must contain a cluster"))?
        .mass;
    let report = sample_report()
        .truncate_for_wire(WIRE_OCCURRENCE_CAP)
        .truncate_for_wire(WIRE_OCCURRENCE_CAP);
    let cluster = report
        .clusters
        .first()
        .ok_or_else(|| anyhow::anyhow!("truncated report must retain its cluster"))?;
    assert!(report.schema_doc.is_empty());
    assert_eq!(cluster.occurrences.len(), WIRE_OCCURRENCE_CAP);
    assert_eq!(cluster.occurrences_total, FULL_OCCURRENCE_COUNT);
    assert_eq!(cluster.occurrence_count, FULL_OCCURRENCE_COUNT);
    assert!(cluster.occurrences_truncated);
    assert_eq!(cluster.mass, original_mass);
    assert_eq!(cluster.rank, 1);
    Ok(())
}

#[test]
fn report_cluster_serialises_only_mass_membership_and_diff_state() -> anyhow::Result<()> {
    let value = serde_json::to_value(sample_report())?;
    let cluster = value
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .and_then(|clusters| clusters.first())
        .ok_or_else(|| anyhow::anyhow!("serialized report must contain its cluster"))?;
    assert_eq!(
        cluster.get("mass").and_then(serde_json::Value::as_u64),
        Some(8)
    );
    assert_eq!(
        cluster
            .get("canonical_node_count")
            .and_then(serde_json::Value::as_u64),
        Some(4)
    );
    assert_eq!(
        cluster
            .get("occurrence_count")
            .and_then(serde_json::Value::as_u64),
        Some(u64::try_from(FULL_OCCURRENCE_COUNT).unwrap_or(u64::MAX))
    );
    for field in FORBIDDEN_CLUSTER_FIELDS {
        assert!(
            cluster.get(field).is_none(),
            "cluster leaked pair/presentation field {field}: {cluster:#}"
        );
    }
    Ok(())
}

#[test]
fn explicit_pair_comparison_round_trips_both_exact_endpoints_and_evidence() -> anyhow::Result<()> {
    let comparison = PairComparison {
        left: endpoint("left.rs", 10, 20),
        right: endpoint("right.rs", 30, 40),
        evidence: PairEvidence {
            structural: 0.8,
            token_jaccard: 0.9,
            embedding_cos: 0.7,
            agreement: 0.75,
            rename_consistency: 0.6,
            literal_fraction: 0.1,
            fused_score: 0.9,
            content_required: true,
            content_ok: true,
            admitted: true,
            classification: Some(PairClassification::NearlyIdentical),
            explanation: "pair clears admission".to_owned(),
        },
    };
    let encoded = serde_json::to_vec(&comparison)?;
    let decoded: PairComparison = serde_json::from_slice(&encoded)?;
    assert_eq!(decoded, comparison);
    assert_ne!(decoded.left, decoded.right);
    assert!(decoded.evidence.admitted);
    assert!(decoded.evidence.content_ok);
    Ok(())
}

#[test]
fn boilerplate_hints_use_default_recommendation_for_future_languages() -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let config_path = tmp.path().join(".deslop.toml");
    fs::write(
        &config_path,
        "[defaults.boilerplate]\nimports = \"report\"\n",
    )?;
    let config = ExclusionConfig::load(&config_path)?;
    assert_eq!(config.source_path(), config_path.as_path());
    let mut registry = FileRegistry::new();
    let file_id = registry.register(tmp.path().join("main.go"));
    let ranges = vec![range(file_id, "go", 0, 10), range(file_id, "go", 11, 20)];
    let hints = build_boilerplate_hints(&ranges, &registry, tmp.path(), &config);
    assert_eq!(hints.len(), 1);
    let hint = hints
        .first()
        .ok_or_else(|| anyhow::anyhow!("the fixture must produce one hint"))?;
    assert_eq!(hint.language, "go");
    assert!(hint.recommendation.contains("harder to read"));
    Ok(())
}

fn endpoint(path: &str, start_byte: usize, end_byte: usize) -> PairEndpoint {
    PairEndpoint {
        path: PathBuf::from(path),
        start_byte,
        end_byte,
    }
}

fn range(
    file_id: deslop_core::state::FileId,
    language: &'static str,
    start: usize,
    end: usize,
) -> BoilerplateRange {
    BoilerplateRange {
        file_id,
        language,
        byte_range: ByteRange { start, end },
    }
}

fn sample_report() -> deslop_core::Report {
    let occurrences = (0_usize..FULL_OCCURRENCE_COUNT)
        .map(|index| {
            let start_byte = index.saturating_mul(10);
            ReportOccurrence {
                path: PathBuf::from(format!("file-{index}.cs")),
                start_byte,
                end_byte: start_byte.saturating_add(5),
                start_line: i64::try_from(index.saturating_add(1)).unwrap_or(i64::MAX),
                end_line: i64::try_from(index.saturating_add(1)).unwrap_or(i64::MAX),
                hidden: false,
                in_diff: None,
            }
        })
        .collect();
    let mut report = fixture_report(vec![fixture_cluster("abcdef", occurrences)]);
    "schema".clone_into(&mut report.schema_doc);
    report
}
