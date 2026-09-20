//! [SEVERITY-TESTING] Category defaults hold independently of duplicated mass.

use deslop_core::{
    buckets::ClusterKind,
    report_fixtures::{fixture_cluster, fixture_occurrence},
};

use super::*;
use crate::diagnostic_settings::DiagnosticLevel;

const PRIMARY: &str = "primary.rs";
const CANONICAL: &str = "canonical.rs";
const WORKSPACE: &str = "/workspace";
const DEFAULTS: [(ClusterKind, Option<DiagnosticSeverity>); 5] = [
    (ClusterKind::Identical, Some(DiagnosticSeverity::WARNING)),
    (
        ClusterKind::NearlyIdentical,
        Some(DiagnosticSeverity::WARNING),
    ),
    (
        ClusterKind::SameBehavior,
        Some(DiagnosticSeverity::INFORMATION),
    ),
    (
        ClusterKind::LooselySimilar,
        Some(DiagnosticSeverity::INFORMATION),
    ),
    (ClusterKind::StructuralOnly, None),
];
const MASSES: [u64; 2] = [1, 10_000];
const LEVELS: [DiagnosticLevel; 5] = [
    DiagnosticLevel::None,
    DiagnosticLevel::Hint,
    DiagnosticLevel::Information,
    DiagnosticLevel::Warning,
    DiagnosticLevel::Error,
];

fn enabled() -> DiagnosticSettings {
    DiagnosticSettings {
        enabled: true,
        ..DiagnosticSettings::default()
    }
}

fn report_for(kind: ClusterKind, mass: u64) -> FileReport {
    let occurrences = vec![
        fixture_occurrence(PRIMARY, 0, 1),
        fixture_occurrence(CANONICAL, 0, 1),
    ];
    let mut cluster = fixture_cluster(kind.wire_label(), occurrences);
    cluster.kind = kind;
    cluster.mass = mass;
    FileReport {
        path: PRIMARY.into(),
        clusters: vec![cluster],
        total_occurrences: 2,
    }
}

#[test]
fn category_defaults_do_not_depend_on_mass_or_rank() {
    for (kind, expected) in DEFAULTS {
        for mass in MASSES {
            let report = report_for(kind, mass);
            let diagnostics = build_for_file(&report, Path::new(WORKSPACE), &enabled());
            assert_eq!(
                diagnostics.len(),
                usize::from(expected.is_some()),
                "{kind:?}"
            );
            assert_eq!(
                diagnostics.first().and_then(|item| item.severity),
                expected,
                "{kind:?}"
            );
        }
    }
}

#[test]
fn every_kind_accepts_every_override_without_changing_the_report() -> anyhow::Result<()> {
    for (kind, _) in DEFAULTS {
        let report = report_for(kind, MASSES[0]);
        for level in LEVELS {
            let mut settings = enabled();
            let _previous = settings.severity_by_kind.insert(kind, level);
            let diagnostics = build_for_file(&report, Path::new(WORKSPACE), &settings);
            assert_eq!(diagnostics.len(), usize::from(level.severity().is_some()));
            assert_eq!(
                diagnostics.first().and_then(|item| item.severity),
                level.severity()
            );
            let cluster = report
                .clusters
                .first()
                .ok_or_else(|| anyhow::anyhow!("fixture finding"))?;
            assert_eq!(cluster.kind, kind);
            assert_eq!(cluster.mass, MASSES[0]);
        }
    }
    Ok(())
}

#[test]
fn disabled_diagnostics_suppress_even_explicit_error_overrides() {
    let mut settings = DiagnosticSettings::default();
    for (kind, _) in DEFAULTS {
        let _previous = settings
            .severity_by_kind
            .insert(kind, DiagnosticLevel::Error);
        let report = report_for(kind, MASSES[0]);
        assert!(build_for_file(&report, Path::new(WORKSPACE), &settings).is_empty());
    }
}

#[test]
fn opted_in_shape_only_diagnostic_describes_information_without_duplicate_quantities(
) -> anyhow::Result<()> {
    let report = report_for(ClusterKind::StructuralOnly, MASSES[0]);
    let mut settings = enabled();
    let _previous = settings
        .severity_by_kind
        .insert(ClusterKind::StructuralOnly, DiagnosticLevel::Hint);
    let diagnostics = build_for_file(&report, Path::new(WORKSPACE), &settings);
    assert_eq!(diagnostics.len(), 1);
    let diagnostic = diagnostics
        .first()
        .ok_or_else(|| anyhow::anyhow!("opted-in information"))?;
    assert_eq!(
        diagnostic.message,
        "Same shape, different content — informational, not a clone"
    );
    assert_eq!(
        diagnostic.data,
        Some(serde_json::json!({"cluster_id": "structural_only", "kind": "structural_only"}))
    );
    assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::HINT));
    Ok(())
}

#[test]
fn settings_validate_kinds_levels_scope_and_missing_entries() -> anyhow::Result<()> {
    let valid = serde_json::json!({"diagnostics": {"enabled": true,
        "severityByKind": {"structural_only": "error"}, "scope": "workspace"}});
    let settings = DiagnosticSettings::from_settings(&valid)?;
    assert!(settings.enabled);
    assert_eq!(settings.severity_by_kind.len(), 1);
    for invalid in [
        serde_json::json!({"severityByKind": {"unknown": "warning"}}),
        serde_json::json!({"severityByKind": {"identical": "worst"}}),
        serde_json::json!({"scope": "all"}),
        serde_json::json!({"enabled": "true"}),
    ] {
        assert!(
            DiagnosticSettings::from_settings(&serde_json::json!({"diagnostics": invalid}))
                .is_err()
        );
    }
    Ok(())
}
