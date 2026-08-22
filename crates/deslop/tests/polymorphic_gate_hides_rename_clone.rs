//! E2E regression for GH #373 [CLONE-NOISE-POLYMORPHIC-SIGNATURE].
//!
//! The polymorphic gate exists to hide different *implementations* of
//! one inherited signature (gh #69). It decided "different
//! implementation" by comparing the enclosing bodies in raw source
//! bytes, so a consistent rename — the definition of a Type-2 clone —
//! read as polymorphism, and the most ordinary duplication shape there
//! is (one helper pasted into a second module, locals renamed) was
//! silently dropped: `clusters: []`, `duplication_percent: 0.0`,
//! exit 0, while every measured axis said duplicate (`structural=1.0`,
//! `token_jaccard=1.0`, `content_rename_consistency=0.91`). Sharing
//! the function name is *stronger* evidence of copy-paste; it must
//! never be the thing that deletes the finding.
//!
//! Both directions live in ONE test at ONE threshold so a fix for
//! either can never trade away the other: the renamed helper must
//! surface, and the genuine abstract-method implementations of gh #69
//! must stay suppressed in the same run.


use crate::common::*;

#[test]
fn same_named_rename_clone_surfaces_while_real_polymorphism_stays_hidden() -> Result<()> {
    let scan_root = fixture("same-name-rename-clone");
    let report = run_report(&scan_root, 8)?;
    let visible = visible_cluster_lines(&report);
    assert_eq!(
        cluster_count(&report),
        1,
        "the renamed `summarise_ledger` pair is the only duplication in \
         this fixture and it must be reported — a same-named consistent \
         rename is a Type-2 clone, not polymorphism: {visible:#?}"
    );
    assert_eq!(
        clusters_hidden(&report),
        0,
        "nothing in this fixture is noise — a hidden cluster means a \
         consistent rename was classified as a different implementation: \
         {report:#}"
    );
    let clone = expect_cluster_spanning(&report, &["alpha.py", "beta.py"])?;
    assert_eq!(
        cluster_bucket(clone),
        "nearly_identical",
        "a total consistent rename is the definition of nearly-identical: \
         {report:#}"
    );
    assert_eq!(
        cluster_size(clone),
        2,
        "one occurrence per file: {report:#}"
    );
    assert!(
        approx(signal(clone, "structural"), 1.0),
        "identifier renames are invisible to the normalised tree: {report:#}"
    );
    assert!(
        approx(signal(clone, "token_jaccard"), 1.0),
        "the token layer is rename-invariant by design: {report:#}"
    );
    assert!(
        signal(clone, "fused") >= 0.6,
        "a certified consistent rename must at least reach the \
         read-the-canonical-occurrence band: {report:#}"
    );
    for occurrence in occurrences(clone) {
        let start = field(occurrence, "start_line").as_u64();
        let end = field(occurrence, "end_line").as_u64();
        assert_eq!(
            start,
            Some(1),
            "the clone begins at `def summarise_ledger` in both files: \
             {visible:#?}"
        );
        assert!(
            end >= Some(16),
            "the clone covers the whole 16-line function in both files: \
             {visible:#?}"
        );
    }
    let duplication = metric_field(&report, "duplication_percent").as_f64();
    assert!(
        duplication.unwrap_or(0.0) > 0.0,
        "two rename-identical files are not 0% duplicated: {report:#}"
    );

    let contract_root = fixture("python-issue-69-abstract-method");
    let contract_report = run_report(&contract_root, 8)?;
    let contract_visible = visible_cluster_lines(&contract_report);
    assert_eq!(
        cluster_count(&contract_report),
        0,
        "four backends implementing one forced `tool_call` signature \
         share bytes by contract, not by copy-paste — the polymorphic \
         gate must keep suppressing them in the same run that surfaces \
         the rename clone: {contract_visible:#?}"
    );
    let contract_duplication = metric_field(&contract_report, "duplication_percent").as_f64();
    assert!(
        approx(contract_duplication.unwrap_or(-1.0), 0.0),
        "nothing extractable exists across the four backends, so the \
         metric must stay at zero: {contract_report:#}"
    );
    Ok(())
}

/// GH #373's secondary defect: every hidden cluster — including ones
/// suppressed by Deslop's own built-in noise filters — was attributed
/// to "your .deslop.toml config", in scans whose root contains no such
/// file. That sends anyone hunting a missing cluster to a config that
/// does not exist. The summary must name what the renderer actually
/// knows: the count, hidden by built-in filters or report policy.
#[test]
fn hidden_group_summary_names_the_hider_not_the_users_config() -> Result<()> {
    let scan_root = fixture("python-issue-69-abstract-method");
    let tmp = tempfile::tempdir()?;
    let mut cmd = deslop_cmd(&scan_root, &tmp.path().join("report"))?;
    let assertion = cmd
        .args(["--min-nodes", "8", "--embeddings", "off"])
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).into_owned();
    assert!(
        stderr.contains("(2 more groups hidden by built-in noise filters or report policy)"),
        "the two contract clusters this fixture hides are suppressed by \
         Deslop's own polymorphic and signature-only filters, and the \
         summary must say so — no .deslop.toml exists in this scan root \
         to blame. stderr:\n{stderr}"
    );
    Ok(())
}
