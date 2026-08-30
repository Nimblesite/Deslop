//! Real-LSP regression for [PIPELINE-DETERMINISM]. Returning to identical
//! source and config bytes must return the identical ordered report.

use crate::common;

use std::{collections::BTreeMap, fs, path::Path, time::Duration};

use anyhow::{anyhow, Result};
use common::{
    at, fixture, handshake, path as json_path, reports::assert_initialize_contract,
    spawn_lsp_guarded, wait_for_report_matching, watched_file_changed, write_frame,
};
use serde_json::Value;

const REPORT_TIMEOUT: Duration = Duration::from_secs(20);

/// The elected pair is the first pair in corpus order when every
/// candidate ties on `max(S, J, E)` ([FUSED-CLUSTER-SIGNALS]).
const ELECTED_PAIR: (u64, u64) = (0, 1);
const FULL_EVIDENCE: f64 = 1.0;
const ORIGINAL_CONFIG: &[u8] = b"[defaults]\nexclude = []\n";
const SOURCE_LAYOUT: [(&str, &str, &str); 4] = [
    ("move/tax_alpha.ts", "ts-type2-loop", "tax_alpha.ts"),
    ("move/tax_beta.ts", "ts-type1-identical", "tax_beta.ts"),
    (
        "stay/inventory_gamma.ts",
        "ts-type2-loop",
        "inventory_gamma.ts",
    ),
    ("stay/tax_alpha.ts", "ts-type1-identical", "tax_alpha.ts"),
];

/// [PIPELINE-DETERMINISM] Excluding and re-including either half of an
/// unchanged corpus must not let append-only file-registration history alter
/// cluster identity, ranges, ranking, or repository metrics.
#[test]
fn config_exclusion_cycle_preserves_the_complete_report() -> Result<()> {
    run_exclusion_cycle("move/**")?;
    run_exclusion_cycle("stay/**")?;
    Ok(())
}

fn run_exclusion_cycle(excluded: &str) -> Result<()> {
    let canonical_temp = fs::canonicalize(std::env::temp_dir())?;
    let workspace = tempfile::tempdir_in(canonical_temp)?;
    seed_workspace(workspace.path())?;
    let before_bytes = workspace_bytes(workspace.path())?;

    let (_guard, mut stdin, mut stdout) = spawn_lsp_guarded(workspace.path())?;
    assert_initialize_contract(&handshake(&mut stdin, &mut stdout)?);

    let clean = wait_for_files(&mut stdin, &mut stdout, 4)?;
    assert_clean_control(&clean)?;

    let config_path = workspace.path().join(".deslop.toml");
    fs::write(
        &config_path,
        format!("[defaults]\nexclude = [\"{excluded}\"]\n"),
    )?;
    write_frame(&mut stdin, &watched_file_changed(&config_path)?)?;
    let reduced = wait_for_files(&mut stdin, &mut stdout, 2)?;
    assert_reduced_control(&reduced, excluded);

    fs::write(&config_path, ORIGINAL_CONFIG)?;
    write_frame(&mut stdin, &watched_file_changed(&config_path)?)?;
    let restored = wait_for_files(&mut stdin, &mut stdout, 4)?;

    assert_eq!(workspace_bytes(workspace.path())?, before_bytes);
    assert_report_identity(&clean, &restored, excluded);
    Ok(())
}

fn seed_workspace(root: &Path) -> Result<()> {
    for (destination, fixture_name, source_name) in SOURCE_LAYOUT {
        let destination = root.join(destination);
        fs::create_dir_all(
            destination
                .parent()
                .ok_or_else(|| anyhow!("destination has no parent"))?,
        )?;
        let _bytes = fs::copy(fixture(fixture_name).join(source_name), destination)?;
    }
    fs::write(root.join(".deslop.toml"), ORIGINAL_CONFIG)?;
    Ok(())
}

fn workspace_bytes(root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut bytes = BTreeMap::new();
    let _previous = bytes.insert(
        ".deslop.toml".to_owned(),
        fs::read(root.join(".deslop.toml"))?,
    );
    for (relative, _, _) in SOURCE_LAYOUT {
        let _previous = bytes.insert(relative.to_owned(), fs::read(root.join(relative))?);
    }
    Ok(bytes)
}

fn wait_for_files(
    stdin: &mut std::process::ChildStdin,
    stdout: &mut std::io::BufReader<std::process::ChildStdout>,
    expected: u64,
) -> Result<Value> {
    wait_for_report_matching(stdin, stdout, REPORT_TIMEOUT, |report| {
        at(report, "files_analysed").as_u64() == Some(expected)
    })
}

fn assert_clean_control(report: &Value) -> Result<()> {
    assert_eq!(at(report, "files_analysed"), 4, "{report:#}");
    assert_eq!(at(report, "clusters_hidden"), 0, "{report:#}");
    assert!(at(report, "embedding_provenance").is_null(), "{report:#}");
    let clusters = at(report, "clusters")
        .as_array()
        .ok_or_else(|| anyhow!("clusters is not an array: {report}"))?;
    assert_eq!(clusters.len(), 1, "control corpus must form one cluster");
    let cluster = clusters
        .first()
        .ok_or_else(|| anyhow!("cluster list is empty: {report}"))?;
    assert_eq!(at(cluster, "size"), 4, "{cluster:#}");
    assert_eq!(at(cluster, "occurrences_total"), 4, "{cluster:#}");
    assert_eq!(at(cluster, "occurrences_truncated"), false, "{cluster:#}");
    assert_eq!(
        json_path(cluster, &["signals", "structural"]),
        1.0,
        "{cluster:#}"
    );
    assert_eq!(
        json_path(cluster, &["signals", "token_jaccard"]),
        1.0,
        "{cluster:#}"
    );
    assert!(
        json_path(cluster, &["signals", "fused"]).is_null(),
        "cluster fused confidence was deleted from the report contract: {cluster:#}"
    );
    assert_eq!(
        (
            json_path(cluster, &["signals", "pair_agreement"]).as_f64(),
            json_path(cluster, &["signals", "pair_rename_consistency"]).as_f64(),
        ),
        (Some(FULL_EVIDENCE), Some(FULL_EVIDENCE)),
        "the elected pair's measured content evidence must survive the history cycle: {cluster:#}"
    );
    assert_eq!(
        (
            json_path(cluster, &["signal_source", "left"]).as_u64(),
            json_path(cluster, &["signal_source", "right"]).as_u64(),
        ),
        (Some(ELECTED_PAIR.0), Some(ELECTED_PAIR.1)),
        "the rendered axes must name the one elected pair: {cluster:#}"
    );
    assert_eq!(
        json_path(report, &["metrics", "clusters_total"]),
        1,
        "{report:#}"
    );
    assert_eq!(
        json_path(report, &["metrics", "duplicated_files"]),
        4,
        "{report:#}"
    );
    assert!(
        json_path(report, &["metrics", "duplicated_loc"])
            .as_u64()
            .unwrap_or_default()
            > 0
    );
    Ok(())
}

fn assert_reduced_control(report: &Value, excluded: &str) {
    assert_eq!(
        at(report, "files_analysed"),
        2,
        "exclude {excluded}: {report:#}"
    );
    assert!(
        json_path(report, &["metrics", "analysed_loc"])
            .as_u64()
            .unwrap_or_default()
            > 0
    );
}

fn assert_report_identity(clean: &Value, restored: &Value, excluded: &str) {
    for field in [
        "files_analysed",
        "clusters_hidden",
        "embedding_provenance",
        "metrics",
        "clusters",
        "action_hints",
        "boilerplate_hints",
    ] {
        assert_eq!(
            restored[field], clean[field],
            "report field `{field}` retained history after cycling {excluded}"
        );
    }
}
