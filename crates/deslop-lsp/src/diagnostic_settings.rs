//! [SEVERITY-CONFIG] Validated editor diagnostic settings.

use std::collections::HashMap;

use deslop_core::buckets::ClusterKind;
use serde::Deserialize;
use tower_lsp::lsp_types::DiagnosticSeverity;

/// A configured diagnostic level; `None` suppresses publication.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel {
    /// Publish nothing.
    None,
    /// Editor hint.
    Hint,
    /// Editor information.
    Information,
    /// Editor warning.
    Warning,
    /// Editor error.
    Error,
}

impl DiagnosticLevel {
    /// Converts a validated setting into its protocol value.
    #[must_use]
    pub const fn severity(self) -> Option<DiagnosticSeverity> {
        match self {
            Self::None => None,
            Self::Hint => Some(DiagnosticSeverity::HINT),
            Self::Information => Some(DiagnosticSeverity::INFORMATION),
            Self::Warning => Some(DiagnosticSeverity::WARNING),
            Self::Error => Some(DiagnosticSeverity::ERROR),
        }
    }
}

/// Files to publish diagnostics for ([LSP-DIAGNOSTICS-SCOPE]).
#[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticScope {
    /// Answer requests for files the client opens.
    #[default]
    OpenFiles,
    /// Also publish every affected workspace file.
    Workspace,
}

/// [SEVERITY-DIAGNOSTICS-GATE] Diagnostics are disabled unless opted in.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticSettings {
    /// Master switch, including explicit kind overrides.
    pub enabled: bool,
    /// Entries override the shared category defaults individually.
    pub severity_by_kind: HashMap<ClusterKind, DiagnosticLevel>,
    /// Publication scope; independent of classification and metrics.
    pub scope: DiagnosticScope,
}

impl DiagnosticSettings {
    /// Reads the diagnostic section from LSP initialization or settings.
    ///
    /// # Errors
    /// Rejects unknown kind keys, levels, fields and invalid value types.
    pub fn from_settings(value: &serde_json::Value) -> Result<Self, serde_json::Error> {
        let section = value.get("deslop").unwrap_or(value);
        serde_json::from_value(
            section
                .get("diagnostics")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
        )
    }
}
