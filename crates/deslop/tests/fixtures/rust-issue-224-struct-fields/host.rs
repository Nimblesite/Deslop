//! Distinct serde data-model structs — different fields, not duplication.
use serde::{Deserialize, Serialize};

/// Host installer channels.
#[derive(Serialize, Deserialize)]
pub struct HostInstaller {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brew: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scoop: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winget: Option<String>,
}

/// Source reference.
#[derive(Serialize, Deserialize)]
pub struct SourceRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}
