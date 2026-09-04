use super::support::*;

/// Writes `report_body` to `<tmp>/<file_name>`, runs the CLI in
/// `--from-report` replay mode over it (adding `--no-color` when
/// requested), and asserts the run succeeds. The shared setup for the
/// `--from-report` preservation scenarios that differ only in the
/// synthetic report they replay and the rendered surface they inspect.
fn replay_report(tmp: &Path, file_name: &str, report_body: &str, no_color: bool) -> Result<()> {
    let report_path = tmp.join(file_name);
    fs::write(&report_path, report_body)?;
    let mut cmd = deslop_command(tmp, &tmp.join("report"))?;
    let _arg = cmd.arg("--from-report").arg(&report_path);
    if no_color {
        let _arg = cmd.arg("--no-color");
    }
    let _assertion = cmd.assert().success();
    Ok(())
}

#[test]
fn from_report_preserves_current_report_fields_issue_85() -> Result<()> {
    // A current-wire report replays cleanly: the engine's cluster facts
    // (rank, band, mass, node count) survive round-trip and the retired
    // similarity fields never appear in the re-emitted JSON ([FACET-MODEL]).
    let tmp = tempfile::tempdir()?;
    let current_report = "{\n\
              \"tool_version\": \"current\",\n\
              \"min_nodes\": 30,\n\
              \"files_analysed\": 0,\n\
              \"clusters_hidden\": 0,\n\
              \"cache_stats\": {\"hits\": 0, \"misses\": 0},\n\
              \"metrics\": {\"analysed_loc\": 0, \"duplicated_loc\": 0, \"duplication_percent\": 0.0, \"clusters_total\": 0, \"duplicated_files\": 0, \"threshold\": {\"percent\": 0.0, \"breached\": false, \"source\": \"none\"}},\n\
              \"schema_doc\": \"\",\n\
              \"boilerplate_hints\": [],\n\
              \"embedding_provenance\": null,\n\
              \"clusters\": [],\n\
              \"literal_findings\": [],\n\
              \"literal_findings_total\": 0,\n\
              \"literal_findings_hidden\": 0,\n\
              \"literal_findings_capped\": false,\n\
              \"literal_max_findings\": 20\n\
              }\n";
    let out = outputs_under(tmp.path());
    replay_report(tmp.path(), "current.json", current_report, true)?;
    let json = read_json_report(&out.json)?;
    assert_eq!(metric_field(&json, "analysed_loc").as_u64(), Some(0));
    assert_eq!(metric_field(&json, "duplicated_loc").as_u64(), Some(0));
    assert_eq!(threshold_field(&json, "source").as_str(), Some("none"));
    assert_eq!(
        json.get("clusters")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len),
        Some(0),
        "an empty replay stays empty"
    );
    let serialized = json.to_string();
    for retired in [
        "bucket",
        "signals",
        "weight",
        "summary",
        "interpretation",
        "action_hints",
    ] {
        assert!(
            !serialized.contains(retired),
            "replayed report must not carry the retired field {retired}"
        );
    }
    Ok(())
}

// A report written in the retired bucket wire (cluster-level signals, bucket
// labels, weight/size) is not a report this tool ever wrote at this version.
// `--from-report` must refuse it instead of silently normalizing retired
// semantics ([FACET-MODEL], [SEVERITY-CONFIG]).
#[test]
fn from_report_rejects_retired_bucket_wire() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let legacy_report = "{\n\
              \"tool_version\": \"legacy\",\n\
              \"min_nodes\": 30,\n\
              \"files_analysed\": 2,\n\
              \"clusters_hidden\": 0,\n\
              \"cache_stats\": {\"hits\": 0, \"misses\": 2},\n\
              \"metrics\": {\"analysed_loc\": 40, \"duplicated_loc\": 12, \"duplication_percent\": 30.0, \"clusters_total\": 1, \"duplicated_files\": 2, \"threshold\": {\"percent\": 0.0, \"breached\": false, \"source\": \"none\"}},\n\
              \"schema_doc\": \"\",\n\
              \"action_hints\": [],\n\
              \"boilerplate_hints\": [],\n\
              \"embedding_provenance\": null,\n\
              \"clusters\": [{\n\
                \"id\": \"legacy1\",\n\
                \"weight\": 2.0,\n\
                \"size\": 2,\n\
                \"canonical_node_count\": 8,\n\
                \"signals\": {\"structural\": 1.0, \"token_jaccard\": 1.0, \"embedding_cos\": 0.0, \"fused\": 1.0},\n\
                \"bucket\": \"identical\",\n\
                \"occurrences\": [{\"path\": \"a.cs\", \"start_byte\": 0, \"end_byte\": 6, \"hidden\": false}],\n\
                \"occurrences_total\": 2,\n\
                \"occurrences_truncated\": false\n\
              }],\n\
              \"literal_findings\": [],\n\
              \"literal_findings_total\": 0,\n\
              \"literal_findings_hidden\": 0,\n\
              \"literal_findings_capped\": false,\n\
              \"literal_max_findings\": 20\n\
              }\n";
    let report_path = tmp.path().join("legacy.json");
    fs::write(&report_path, legacy_report)?;
    let mut cmd = deslop_command(tmp.path(), &tmp.path().join("report"))?;
    let _arg = cmd.arg("--from-report").arg(&report_path);
    let _rejection = cmd.assert().failure();
    Ok(())
}

#[test]
fn from_report_preserves_mass_and_band_in_html() -> Result<()> {
    // The replayed HTML is a cluster surface: it renders the neutral
    // verdict, the cluster's mass, and never a similarity classification
    // ([FACET-HTML], [VSIX-PAIR-COMPARE]).
    let tmp = tempfile::tempdir()?;
    let report = "{\n\
                  \"tool_version\": \"synthetic\",\n\
                  \"min_nodes\": 30,\n\
                  \"files_analysed\": 1,\n\
                  \"clusters_hidden\": 0,\n\
                  \"cache_stats\": {\"hits\": 0, \"misses\": 0},\n\
                  \"metrics\": {\"analysed_loc\": 0, \"duplicated_loc\": 0, \"duplication_percent\": 0.0, \"clusters_total\": 1, \"duplicated_files\": 1, \"threshold\": {\"percent\": 0.0, \"breached\": false, \"source\": \"none\"}},\n\
                  \"schema_doc\": \"\",\n\
                  \"boilerplate_hints\": [],\n\
                  \"embedding_provenance\": null,\n\
                  \"clusters\": [{\n\
                    \"id\": \"reported-dup\",\n\
                    \"rank\": 1,\n\
                    \"rank_band\": \"worst\",\n\
                    \"mass\": 89,\n\
                    \"canonical_node_count\": 12,\n\
                    \"occurrences\": [\n\
                      {\"path\": \"missing.unknown\", \"start_byte\": 0, \"end_byte\": 40, \"start_line\": 1, \"end_line\": 3, \"hidden\": false},\n\
                      {\"path\": \"missing2.unknown\", \"start_byte\": 60, \"end_byte\": 100, \"start_line\": 9, \"end_line\": 11, \"hidden\": false}\n\
                    ],\n\
                    \"occurrences_total\": 2,\n\
                    \"occurrence_count\": 2,\n\
                    \"occurrences_truncated\": false\n\
                  }],\n\
                  \"literal_findings\": [],\n\
                  \"literal_findings_total\": 0,\n\
                  \"literal_findings_hidden\": 0,\n\
                  \"literal_findings_capped\": false,\n\
                  \"literal_max_findings\": 20\n\
                  }\n";
    let out = outputs_under(tmp.path());
    replay_report(tmp.path(), "semantic.json", report, false)?;
    let html = fs::read_to_string(&out.html)?;
    assert!(
        html.contains("Duplicate code"),
        "the neutral verdict renders"
    );
    assert!(
        html.contains("mass 89"),
        "the cluster's mass renders: {html}"
    );
    for retired in [
        "Same behavior",
        "AI match",
        "bucket:",
        "kind-identical",
        "facet-identical",
    ] {
        assert!(
            !html.contains(retired),
            "replayed HTML must not carry the retired classification {retired}"
        );
    }
    Ok(())
}

#[test]
fn cross_cluster_collapse_removes_occurrence_subset_clusters() -> Result<()> {
    let (_tmp, out, mut cmd) = fixture_run("csharp-prologue-false-positive")?;
    let _assertion = cmd.args(["--min-nodes", "2"]).assert().success();
    let json = read_json_report(&out.json)?;
    let clusters = json
        .get("clusters")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("clusters missing from report"))?;
    for (index_a, cluster_a) in clusters.iter().enumerate() {
        let occs_a: &[serde_json::Value] = cluster_a
            .get("occurrences")
            .and_then(|v| v.as_array())
            .map_or(&[], Vec::as_slice);
        for (index_b, cluster_b) in clusters.iter().enumerate() {
            if index_a == index_b {
                continue;
            }
            let occs_b: &[serde_json::Value] = cluster_b
                .get("occurrences")
                .and_then(|v| v.as_array())
                .map_or(&[], Vec::as_slice);
            if all_occurrences_json_contained(occs_b, occs_a) {
                let id_a = cluster_a.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let id_b = cluster_b.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                anyhow::bail!(
                    "cluster {id_b} is a strict occurrence-subset of cluster {id_a} — \
                     cross-cluster overlap collapse must prevent this redundancy; \
                     got {len_b} occurrences all inside {len_a} occurrences",
                    len_b = occs_b.len(),
                    len_a = occs_a.len(),
                );
            }
        }
    }
    Ok(())
}

/// Returns `true` when every occurrence in `inner` is present in `outer`
/// at the same file and byte range (non-strict containment).
fn all_occurrences_json_contained(
    inner: &[serde_json::Value],
    outer: &[serde_json::Value],
) -> bool {
    !inner.is_empty()
        && inner.iter().all(|oi| {
            let path = oi.get("path").and_then(Value::as_str).unwrap_or("");
            let start = oi.get("start_byte").and_then(Value::as_u64).unwrap_or(0);
            let end = oi.get("end_byte").and_then(Value::as_u64).unwrap_or(0);
            outer.iter().any(|oo| {
                oo.get("path").and_then(Value::as_str).unwrap_or("") == path
                    && oo.get("start_byte").and_then(Value::as_u64).unwrap_or(0) <= start
                    && end <= oo.get("end_byte").and_then(Value::as_u64).unwrap_or(0)
            })
        })
}
