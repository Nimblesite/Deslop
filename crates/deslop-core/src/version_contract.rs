//! Deployment Toolkit binary version output helpers.

use serde::Serialize;

/// Deployment Toolkit version-output schema version.
const MANIFEST_VERSION: u32 = 1;
/// Product id shared by every Deslop executable component.
const PRODUCT_ID: &str = "deslop";
/// Language label required by the version-output schema.
const RUST_LANGUAGE: &str = "rust";

/// Executable component kind used in Deployment Toolkit version JSON.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentKind {
    /// Cold-cache command-line interface.
    Cli,
    /// Language Server Protocol server.
    Lsp,
    /// Model Context Protocol server.
    Mcp,
}

impl ComponentKind {
    /// Returns the schema string for this component kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Lsp => "lsp",
            Self::Mcp => "mcp",
        }
    }
}

/// JSON payload emitted by `--version --json`.
#[derive(Serialize)]
struct VersionManifest<'a> {
    /// Version-output schema revision.
    #[serde(rename = "manifestVersion")]
    manifest_version: u32,
    /// Deployment component id.
    name: &'a str,
    /// Semantic component version.
    version: &'a str,
    /// Deployment component kind.
    kind: &'a str,
    /// Implementation language.
    language: &'a str,
    /// Product id that owns the component.
    product: &'a str,
}

/// Builds the exact plain-text version contract line.
#[must_use]
pub fn plain_version_line(component_id: &str) -> String {
    format!("{component_id} {}\n", crate::version())
}

/// Builds JSON output matching Deployment Toolkit's version schema.
///
/// See <https://github.com/MelbourneDeveloper/deployment_toolkit/blob/main/schemas/version-manifest.schema.json>.
///
/// # Errors
///
/// Returns a serialization error if the schema payload cannot be encoded.
pub fn json_version_line(
    component_id: &str,
    kind: ComponentKind,
) -> Result<String, serde_json::Error> {
    let mut output = serde_json::to_string(&version_manifest(component_id, kind))?;
    output.push('\n');
    Ok(output)
}

/// Returns version output when CLI args request the contract.
///
/// Supports the issue-required `--version --json` form and the private-doc
/// `--version --format json` spelling.
///
/// # Errors
///
/// Returns a serialization error when JSON output is requested and fails.
pub fn version_contract_output(
    args: &[String],
    component_id: &str,
    kind: ComponentKind,
) -> Result<Option<String>, serde_json::Error> {
    if !requests_version(args) {
        return Ok(None);
    }
    if requests_json(args) {
        return json_version_line(component_id, kind).map(Some);
    }
    Ok(Some(plain_version_line(component_id)))
}

/// Returns whether args request any version output.
fn requests_version(args: &[String]) -> bool {
    args.iter()
        .skip(1)
        .any(|arg| matches!(arg.as_str(), "--version" | "-V"))
}

/// Returns whether args request JSON version output.
fn requests_json(args: &[String]) -> bool {
    args.iter().skip(1).any(|arg| arg == "--json")
        || args.windows(2).any(|pair| {
            pair.first().is_some_and(|flag| flag == "--format")
                && pair.get(1).is_some_and(|value| value == "json")
        })
}

/// Builds the structured JSON payload before serialization.
fn version_manifest(component_id: &str, kind: ComponentKind) -> VersionManifest<'_> {
    VersionManifest {
        manifest_version: MANIFEST_VERSION,
        name: component_id,
        version: crate::version(),
        kind: kind.as_str(),
        language: RUST_LANGUAGE,
        product: PRODUCT_ID,
    }
}
