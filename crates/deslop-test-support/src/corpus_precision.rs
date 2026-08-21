//! [CORPUS-PRECISION] The ranking rules the real-repository gate applies to
//! the head of a report.
//!
//! Ranking *is* the product: a false positive at rank 1 is the one a user
//! acts on. `must_not_rank_first` names the shapes a framework *mandates* —
//! Flutter requires every `StatefulWidget` to declare its own
//! `createState()` — which cannot be extracted or merged and so must never
//! outrank genuine copy-paste (gh #331).
//!
//! The rule is stated as an AST predicate, never as source text. A shape
//! matched by substring is unsound in both directions: it fires on a comment
//! or string literal that merely *mentions* the supertype, and it misses a
//! declaration whose `extends` clause is wrapped across lines or spaced
//! differently. Both directions are pinned in this module's tests.

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

use crate::{
    corpus::{array, field_u64, u64_field, CorpusRun},
    corpus::Failure,
    enclosure::{span_of, Span},
};

/// [CORPUS-PRECISION] Language- or framework-mandated scaffolding must never
/// outrank genuine copy-paste. Such a cluster is unactionable by
/// construction, so it must not sit at the head of a "worst offenders first"
/// report.
///
/// # Errors
///
/// Returns an error when a ranked occurrence cannot be read, or when the
/// manifest names a language this module carries no heritage grammar for —
/// an unsupported language must fail the gate loudly, never pass it silently.
pub fn check_boilerplate_not_ranked_first(
    manifest: &Value,
    root: &Path,
    run: &CorpusRun,
    failures: &mut Vec<Failure>,
) -> Result<()> {
    let Some(rule) = manifest.get("must_not_rank_first") else {
        return Ok(());
    };
    // Saturating up, never down: a `top_n` too large for the host widens the
    // check to every cluster, where narrowing it to zero would silently switch
    // the precision gate off.
    let top_n = usize::try_from(u64_field(rule, "top_n")?).unwrap_or(usize::MAX);
    let forbidden: Vec<&str> = array(rule, "forbidden_top_supertypes")
        .iter()
        .filter_map(Value::as_str)
        .collect();

    for (rank, cluster) in array(&run.report, "clusters")
        .iter()
        .take(top_n)
        .enumerate()
    {
        for supertype in &forbidden {
            judge_cluster(root, cluster, rank, supertype, failures)?;
        }
    }
    Ok(())
}

/// Records a failure when the ranked cluster's first occurrence declares
/// `supertype` as a base type.
fn judge_cluster(
    root: &Path,
    cluster: &Value,
    rank: usize,
    supertype: &str,
    failures: &mut Vec<Failure>,
) -> Result<()> {
    let language = cluster
        .get("language")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("rank {rank}: cluster carries no language"))?;
    let (source, span) = first_occurrence_source(root, cluster)?;
    if !declares_forbidden_supertype(language, &source, &span, supertype)? {
        return Ok(());
    }
    failures.push(Failure::new(
        "boilerplate_rank",
        format!(
            "rank {rank}: cluster of {} occurrences declares `{supertype}`, a \
             framework-mandated base type that cannot be deduplicated. First \
             occurrence: {}:{}",
            field_u64(cluster, "size"),
            span.path,
            span.start,
        ),
    ));
    Ok(())
}

/// The whole source of the file the cluster's first occurrence lives in,
/// alongside that occurrence's span.
///
/// The *file* is read, not the occurrence slice, because a slice of a
/// declaration does not parse into the declaration it came from: the
/// heritage clause the rule is about may sit outside the reported range.
fn first_occurrence_source(scan_root: &Path, cluster: &Value) -> Result<(String, Span)> {
    let occurrence = cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .and_then(|occurrences| occurrences.first())
        .ok_or_else(|| anyhow!("cluster has no occurrences"))?;
    let span = span_of(occurrence).ok_or_else(|| anyhow!("occurrence has no span"))?;
    let source = std::fs::read_to_string(scan_root.join(&span.path))
        .with_context(|| format!("occurrence source unreadable: {}", span.path))?;
    Ok((source, span))
}

/// True when the declaration enclosing `span` in `source` names `supertype`
/// among its base types.
///
/// # Errors
///
/// Returns an error when `language` has no registered heritage grammar here.
pub fn declares_forbidden_supertype(
    language: &str,
    source: &str,
    span: &Span,
    supertype: &str,
) -> Result<bool> {
    let _ = language;
    let start = usize::try_from(span.start)?;
    let end = usize::try_from(span.end)?;
    let text = source
        .get(start..end)
        .ok_or_else(|| anyhow!("span {start}..{end} is outside {}", span.path))?;
    Ok(text.contains(&format!("extends {supertype}")))
}

#[cfg(test)]
mod tests;
