//! [PAIR-COMPARE-CLI] `--compare`: the engine's verdict on exactly two
//! occurrences.
//!
//! An endpoint names one exact occurrence: `<path>:<start_byte>:<end_byte>`,
//! the same triple `pair/compare` and `compare_pair` take on the LSP and
//! MCP surfaces. The byte offsets are the ones a rendered report already
//! carries on every occurrence, so a caller pastes them straight out of
//! the JSON it just read.
//!
//! Admission evidence is pair-scoped and recomputed on demand — never
//! stored on a cluster, never carried in a rendered report — so answering
//! needs a resolved corpus, which is why the scan runs first.

use std::{io::Write as _, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use deslop_core::{
    wire_generated::{PairComparisonParams, PairEndpoint},
    EmbeddingMode, EmbeddingProvider, EmbeddingSettings, PipelineSession,
};

use crate::diff_input::pipeline_error;

/// Separator between an endpoint's path and its two byte offsets.
const ENDPOINT_SEPARATOR: char = ':';

/// Endpoints one comparison needs — exactly two, never a cluster id.
pub(crate) const ENDPOINTS_PER_COMPARISON: usize = 2;

/// The scan that resolves the endpoints: the same root, node floor, cache
/// policy and config file an ordinary `deslop <path>` run reads.
pub(crate) struct ScanRequest {
    /// The scan root.
    pub(crate) root: PathBuf,
    /// `--min-nodes`.
    pub(crate) min_nodes: u32,
    /// Whether the on-disk parse store may be consulted and filled.
    pub(crate) incremental: bool,
    /// `--config`, when given.
    pub(crate) config: Option<PathBuf>,
}

/// [PAIR-COMPARE-CLI] Recomputes admission evidence for exactly the two
/// named occurrences and writes the verdict to stdout as one JSON
/// `PairComparison`.
///
/// # Errors
///
/// Returns an error when the endpoints do not parse, when the scan fails,
/// when either endpoint names a range the scan did not fingerprint, or
/// when the verdict cannot be written.
pub(crate) fn run(
    scan: ScanRequest,
    mode: EmbeddingMode,
    provider: Option<&dyn EmbeddingProvider>,
    endpoints: &[String],
) -> Result<()> {
    let (left, right) = parse_comparison(endpoints)?;
    let settings = EmbeddingSettings {
        mode,
        provider,
        batch_yield: None,
        progress: None,
    };
    let (session, _initial) = PipelineSession::initialise_with_diff(
        scan.root,
        scan.min_nodes,
        scan.incremental,
        scan.config,
        settings,
        None,
    )
    .map_err(pipeline_error)?;
    let comparison = session
        .compare_pair(&PairComparisonParams { left, right }, provider)
        .map_err(pipeline_error)?;
    let rendered = serde_json::to_string_pretty(&comparison).context("render pair verdict")?;
    let mut handle = std::io::stdout().lock();
    handle
        .write_all(rendered.as_bytes())
        .context("write pair verdict to stdout")?;
    handle.write_all(b"\n").context("terminate pair verdict")
}

/// Parses the two endpoints one `--compare` invocation names.
///
/// # Errors
///
/// Returns an error unless exactly [`ENDPOINTS_PER_COMPARISON`] endpoints
/// are given, or when either fails to parse.
pub(crate) fn parse_comparison(endpoints: &[String]) -> Result<(PairEndpoint, PairEndpoint)> {
    if endpoints.len() != ENDPOINTS_PER_COMPARISON {
        return Err(anyhow!(
            "`--compare` names exactly {ENDPOINTS_PER_COMPARISON} endpoints, one per occurrence \
             — got {}. A cluster id is not valid input: the engine never chooses comparison \
             endpoints from a component.",
            endpoints.len()
        ));
    }
    let mut parsed = endpoints.iter().map(|text| parse_endpoint(text));
    let left = parsed.next().ok_or_else(|| anyhow!("no left endpoint"))??;
    let right = parsed
        .next()
        .ok_or_else(|| anyhow!("no right endpoint"))??;
    Ok((left, right))
}

/// Parses `<path>:<start_byte>:<end_byte>`.
///
/// Split from the right, so a Windows drive letter or any other colon in
/// the path survives: only the final two fields are offsets.
///
/// # Errors
///
/// Returns an error when the triple is malformed, either offset is not a
/// number, or the range is empty or inverted.
pub(crate) fn parse_endpoint(text: &str) -> Result<PairEndpoint> {
    let (head, end) = text
        .rsplit_once(ENDPOINT_SEPARATOR)
        .ok_or_else(|| malformed(text))?;
    let (path, start) = head
        .rsplit_once(ENDPOINT_SEPARATOR)
        .ok_or_else(|| malformed(text))?;
    if path.is_empty() {
        return Err(malformed(text));
    }
    let start_byte: usize = start
        .parse()
        .with_context(|| format!("endpoint `{text}` has a non-numeric start byte `{start}`"))?;
    let end_byte: usize = end
        .parse()
        .with_context(|| format!("endpoint `{text}` has a non-numeric end byte `{end}`"))?;
    if end_byte <= start_byte {
        return Err(anyhow!(
            "endpoint `{text}` covers no bytes: end {end_byte} is not past start {start_byte}"
        ));
    }
    Ok(PairEndpoint {
        path: path.into(),
        start_byte,
        end_byte,
    })
}

/// The error for an endpoint that is not a `<path>:<start_byte>:<end_byte>`
/// triple.
fn malformed(text: &str) -> anyhow::Error {
    anyhow!("endpoint `{text}` is not `<path>:<start_byte>:<end_byte>`")
}

#[cfg(test)]
mod tests;
