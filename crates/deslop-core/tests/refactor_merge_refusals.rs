//! The refusal half of the mechanical call-site merge
//! ([AUTOFIX-MERGE-GATE], [AUTOFIX-MERGE-SAFETY], [AUTOFIX-ZERO-RISK]).
//!
//! Every fixture here is one a merge must decline. A refusal routes to
//! `ai_or_human` carrying a reason and no `WorkspaceEdit` — never a
//! partial plan, because a partial merge silently changes behaviour
//! while looking like a successful refactor.

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::wire_generated::MergeVerdict;

use crate::common::{
    fixture,
    merge::{assert_all_refused_with, merge_plans, synthetic_merge_plan},
};

/// Control-flow drift routes to `ai_or_human` with a structural reason
/// ([AUTOFIX-MERGE-GATE] step 2) — never a partial plan.
#[test]
fn structural_drift_routes_to_ai_or_human() -> Result<()> {
    let plans = merge_plans("csharp-merge-drift", "DriftLimits.cs")?;
    ensure!(!plans.is_empty(), "drift fixture produces clusters");
    for plan in plans {
        let MergeVerdict::AiOrHuman { reason } = plan.verdict else {
            return Err(anyhow!(
                "drifted cluster {} must not merge",
                plan.cluster_id
            ));
        };
        ensure!(!reason.is_empty(), "refusal carries a reason");
        ensure!(
            plan.workspace_edit.is_none(),
            "refusals carry no workspace edit"
        );
    }
    Ok(())
}

/// A slot whose literals disagree on type routes to `ai_or_human`
/// ([AUTOFIX-MERGE-SAFETY] D) — no `object` guessing.
#[test]
fn literal_type_conflict_routes_to_ai_or_human() -> Result<()> {
    let plans = merge_plans("csharp-merge-typeconflict", "MixedDefaults.cs")?;
    ensure!(!plans.is_empty(), "type-conflict fixture produces clusters");
    for plan in plans {
        ensure!(
            matches!(plan.verdict, MergeVerdict::AiOrHuman { .. }),
            "type-conflicting cluster {} must not merge",
            plan.cluster_id
        );
    }
    Ok(())
}

/// [AUTOFIX-ZERO-RISK]: Python merges always refuse in v1 — strict
/// type checking cannot be detected yet, and without it there is no
/// compiler backstop.
#[test]
fn python_merge_always_refuses() -> Result<()> {
    let plans = merge_plans("python-extract-type1", "metrics.py")?;
    for plan in plans {
        let MergeVerdict::AiOrHuman { reason } = plan.verdict else {
            return Err(anyhow!("python cluster {} must refuse", plan.cluster_id));
        };
        ensure!(
            reason.contains("python"),
            "the reason names the language gate, got {reason}"
        );
    }
    Ok(())
}

/// [AUTOFIX-MERGE-SAFETY] B: a `return` crossing the boundary refuses.
#[test]
fn boundary_crossing_return_refuses() -> Result<()> {
    assert_all_refused_with("csharp-merge-return", "EarlyExit.cs", "transfers control")
}

/// [AUTOFIX-MERGE-SAFETY] B: a local declared inside the span and read
/// after it refuses.
#[test]
fn declared_inside_read_after_refuses() -> Result<()> {
    assert_all_refused_with("csharp-merge-readafter", "Prefix.cs", "read after")
}

/// [AUTOFIX-MERGE-SAFETY] D: a hole identifier written inside the span
/// refuses — call-time evaluation would change behaviour.
#[test]
fn written_hole_identifier_refuses() -> Result<()> {
    assert_all_refused_with("csharp-merge-writtenhole", "Mutator.cs", "written inside")
}

/// [AUTOFIX-MERGE-SAFETY] / [AUTOFIX-EXTRACT-PRECONDITIONS] rule 7: a
/// *context* free variable (identical at every site, so never a hole)
/// written inside the span refuses — the helper's by-value parameter
/// copy would absorb the mutation and every caller's variable would
/// keep its old value.
#[test]
fn written_context_variable_refuses() -> Result<()> {
    assert_all_refused_with(
        "csharp-merge-writtencontext",
        "Accumulator.cs",
        "written inside",
    )
}

/// Same context-write refusal through the Dart tables — the only
/// coverage of Dart's `write_kinds` (Dart has no Tier-1 emitter, so an
/// extract-path Dart test would be vacuous).
#[test]
fn dart_written_context_variable_refuses() -> Result<()> {
    assert_all_refused_with(
        "dart-merge-writtencontext",
        "accumulator.dart",
        "written inside",
    )
}

/// The residual byte proof: operator drift outside the holes refuses
/// even though the normalised skeletons match.
#[test]
fn operator_drift_refuses_via_residual_proof() -> Result<()> {
    assert_all_refused_with(
        "csharp-merge-operatordrift",
        "Drift.cs",
        "not byte-equivalent",
    )
}

/// [AUTOFIX-MERGE-GATE] 4b/4c: too many differing leaves refuse.
#[test]
fn too_many_holes_refuse() -> Result<()> {
    let root = fixture("csharp-merge-manyholes");
    let plan = synthetic_merge_plan(&root, "Sprawl.cs", &["\"a1\"", "\"a2\""], sprawl_body_span)?;
    let MergeVerdict::AiOrHuman { reason } = plan.verdict else {
        return Err(anyhow!("twelve distinct substitutions must refuse"));
    };
    ensure!(
        reason.contains("exceed the budget"),
        "the substitution budget names itself, got {reason}"
    );
    Ok(())
}

/// The statement span of one `Sprawl.cs` method body.
fn sprawl_body_span(text: &str, anchor: &str) -> Result<(usize, usize)> {
    let position = text.find(anchor).context("anchor present")?;
    let start = text
        .get(..position)
        .and_then(|head| head.rfind("policy.Set("))
        .context("body start")?;
    let end = text
        .get(position..)
        .and_then(|tail| tail.find("policy.Commit();"))
        .map(|offset| {
            position
                .saturating_add(offset)
                .saturating_add("policy.Commit();".len())
        })
        .context("body end")?;
    Ok((start, end))
}
