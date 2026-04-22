//! Report projection for suppressed import/prologue boilerplate.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    boilerplate::BoilerplateRange,
    config::ExclusionConfig,
    state::{FileId, FileRegistry},
};

/// Low-severity hygiene hint for import/prologue boilerplate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportBoilerplateHint {
    /// Hint category. Currently `"imports"`.
    pub kind: String,
    /// Language id the hint applies to.
    pub language: String,
    /// Always `"info"` for boilerplate hygiene hints.
    pub severity: String,
    /// Gentle remediation guidance.
    pub recommendation: String,
    /// Suppressed byte ranges that justify the hint.
    pub occurrences: Vec<ReportBoilerplateOccurrence>,
}

/// One suppressed import/prologue occurrence in the report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportBoilerplateOccurrence {
    /// Source path, relative to the scan root when possible.
    pub path: PathBuf,
    /// Inclusive start byte of the suppressed boilerplate range.
    pub start_byte: usize,
    /// Exclusive end byte of the suppressed boilerplate range.
    pub end_byte: usize,
}

/// Builds report hints from suppressed boilerplate ranges.
#[must_use]
pub fn build_boilerplate_hints(
    ranges: &[BoilerplateRange],
    registry: &FileRegistry,
    scan_root: &Path,
    exclusion: &ExclusionConfig,
) -> Vec<ReportBoilerplateHint> {
    let mut by_language: BTreeMap<&'static str, Vec<ReportBoilerplateOccurrence>> = BTreeMap::new();
    for range in ranges {
        if !exclusion
            .boilerplate_imports_mode(range.language)
            .reports_hints()
        {
            continue;
        }
        by_language
            .entry(range.language)
            .or_default()
            .push(occurrence(range, registry, scan_root));
    }
    by_language
        .into_iter()
        .filter_map(|(language, occurrences)| hint(language, occurrences))
        .collect()
}

/// Builds one hint when there are repeated occurrences for `language`.
fn hint(
    language: &'static str,
    occurrences: Vec<ReportBoilerplateOccurrence>,
) -> Option<ReportBoilerplateHint> {
    if occurrences.len() < 2 {
        return None;
    }
    Some(ReportBoilerplateHint {
        kind: "imports".to_owned(),
        language: language.to_owned(),
        severity: "info".to_owned(),
        recommendation: recommendation(language),
        occurrences,
    })
}

/// Projects one internal range into its public report occurrence.
fn occurrence(
    range: &BoilerplateRange,
    registry: &FileRegistry,
    scan_root: &Path,
) -> ReportBoilerplateOccurrence {
    ReportBoilerplateOccurrence {
        path: display_path(range.file_id, registry, scan_root),
        start_byte: range.byte_range.start,
        end_byte: range.byte_range.end,
    }
}

/// Resolves the report path for `file_id` relative to `scan_root`.
fn display_path(file_id: FileId, registry: &FileRegistry, scan_root: &Path) -> PathBuf {
    registry.path(file_id).map_or_else(PathBuf::new, |abs| {
        abs.strip_prefix(scan_root)
            .map_or_else(|_| abs.to_path_buf(), Path::to_path_buf)
    })
}

/// Returns the gentle remediation copy for a language.
fn recommendation(language: &str) -> String {
    match language {
        "csharp" => csharp_recommendation(),
        "rust" => "Repeated use declarations are import hygiene, not duplicate logic. Consider a small prelude only when it improves local readability.".to_owned(),
        "python" => "Repeated imports are import hygiene, not duplicate logic. Keep them local unless a shared module makes the dependency story clearer.".to_owned(),
        _ => "Repeated imports are import hygiene, not duplicate logic. Review only if they make files harder to read.".to_owned(),
    }
}

/// Returns the C# global-using remediation copy.
fn csharp_recommendation() -> String {
    "Repeated using directives are import hygiene, not duplicate logic. Consider moving stable \
     common usings to GlobalUsings.cs or project-file <Using Include=\"...\" />."
        .to_owned()
}
