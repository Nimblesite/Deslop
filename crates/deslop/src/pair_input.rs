//! [PAIR-COMPARE-CLI] Parsing `--compare` endpoints.
//!
//! An endpoint names one exact occurrence: `<path>:<start_byte>:<end_byte>`,
//! the same triple `pair/compare` and `compare_pair` take on the LSP and
//! MCP surfaces. The byte offsets are the ones a rendered report already
//! carries on every occurrence, so a caller pastes them straight out of
//! the JSON it just read.

use anyhow::{anyhow, Context, Result};
use deslop_core::wire_generated::PairEndpoint;

/// Separator between an endpoint's path and its two byte offsets.
const ENDPOINT_SEPARATOR: char = ':';

/// Endpoints one comparison needs — exactly two, never a cluster id.
pub const ENDPOINTS_PER_COMPARISON: usize = 2;

/// Parses `<path>:<start_byte>:<end_byte>`.
///
/// Split from the right, so a Windows drive letter or any other colon in
/// the path survives: only the final two fields are offsets.
///
/// # Errors
///
/// Returns an error when the triple is malformed, either offset is not a
/// number, or the range is empty or inverted.
pub fn parse_endpoint(text: &str) -> Result<PairEndpoint> {
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

/// Parses the two endpoints one `--compare` invocation names.
///
/// # Errors
///
/// Returns an error unless exactly [`ENDPOINTS_PER_COMPARISON`] endpoints
/// are given, or when either fails to parse.
pub fn parse_comparison(endpoints: &[String]) -> Result<(PairEndpoint, PairEndpoint)> {
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

/// The error a malformed endpoint earns, naming the shape expected.
fn malformed(text: &str) -> anyhow::Error {
    anyhow!("endpoint `{text}` is not `<path>{ENDPOINT_SEPARATOR}<start_byte>{ENDPOINT_SEPARATOR}<end_byte>`")
}

#[cfg(test)]
mod tests;
