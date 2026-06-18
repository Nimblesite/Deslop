use crate::support::*;

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
fn from_report_preserves_current_empty_bucket_issue_85() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let current_report = "{\n\
              \"tool_version\": \"current\",\n\
              \"min_nodes\": 30,\n\
              \"files_analysed\": 0,\n\
              \"clusters_hidden\": 0,\n\
              \"cache_stats\": {\"hits\": 0, \"misses\": 0},\n\
              \"metrics\": {\"analysed_loc\": 0, \"duplicated_loc\": 0, \"duplication_percent\": 0.0, \"clusters_total\": 0, \"duplicated_files\": 0, \"threshold\": {\"percent\": 0.0, \"breached\": false, \"source\": \"none\"}},\n\
              \"schema_doc\": \"\",\n\
              \"action_hints\": [],\n\
              \"boilerplate_hints\": [],\n\
              \"embedding_provenance\": null,\n\
              \"clusters\": [{\n\
                \"id\": \"abc123\",\n\
                \"weight\": 1.0,\n\
                \"size\": 2,\n\
                \"canonical_node_count\": 8,\n\
                \"signals\": {\"structural\": 1.0, \"token_jaccard\": 1.0, \"embedding_cos\": 0.0, \"fused\": 1.0},\n\
                \"bucket\": \"\",\n\
                \"occurrences\": [],\n\
                \"occurrences_total\": 0,\n\
                \"occurrences_truncated\": false,\n\
                \"summary\": \"current\",\n\
                \"interpretation\": \"current\"\n\
              }]\n\
              }\n";
    let out = outputs_under(tmp.path());
    replay_report(tmp.path(), "current.json", current_report, true)?;
    let json = read_json_report(&out.json)?;
    assert_eq!(metric_field(&json, "analysed_loc").as_u64(), Some(0));
    assert_eq!(metric_field(&json, "duplicated_loc").as_u64(), Some(0));
    assert_eq!(threshold_field(&json, "source").as_str(), Some("none"));
    let cluster = json
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .and_then(|clusters| clusters.first())
        .context("current report cluster should survive --from-report")?;
    let bucket = cluster.get("bucket").and_then(serde_json::Value::as_str);
    assert_eq!(bucket, Some(""));
    assert_eq!(
        cluster.get("summary").and_then(serde_json::Value::as_str),
        Some("current")
    );
    assert_eq!(
        cluster
            .get("interpretation")
            .and_then(serde_json::Value::as_str),
        Some("current")
    );
    Ok(())
}

// Implements [CLONE-BUCKETS-DUAL-LABEL]: `--from-report` must preserve a
// schema-carried `same_behavior` bucket instead of re-routing from signals.
#[test]
fn from_report_preserves_same_behavior_bucket_in_html() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let report = "{\n\
                  \"tool_version\": \"synthetic\",\n\
                  \"min_nodes\": 30,\n\
                  \"files_analysed\": 1,\n\
                  \"clusters_hidden\": 0,\n\
                  \"cache_stats\": {\"hits\": 0, \"misses\": 0},\n\
                  \"metrics\": {\"analysed_loc\": 0, \"duplicated_loc\": 0, \"duplication_percent\": 0.0, \"clusters_total\": 0, \"duplicated_files\": 0, \"threshold\": {\"percent\": 0.0, \"breached\": false, \"source\": \"none\"}},\n\
                  \"schema_doc\": \"\",\n\
                  \"action_hints\": [],\n\
                  \"boilerplate_hints\": [],\n\
                  \"embedding_provenance\": null,\n\
                  \"clusters\": [{\n\
                    \"id\": \"same-behavior\",\n\
                    \"weight\": 4.0,\n\
                    \"size\": 2,\n\
                    \"canonical_node_count\": 12,\n\
                    \"signals\": {\"structural\": 0.0, \"token_jaccard\": 0.0, \"embedding_cos\": 0.9, \"fused\": 0.9},\n\
                    \"bucket\": \"same_behavior\",\n\
                    \"occurrences\": [{\"path\": \"missing.unknown\", \"start_byte\": 0, \"end_byte\": 0, \"hidden\": false}],\n\
                    \"occurrences_total\": 0,\n\
                    \"occurrences_truncated\": false,\n\
                    \"summary\": \"synthetic semantic clone\",\n\
                    \"interpretation\": \"semantic clone\"\n\
                  }]\n\
                  }\n";
    let out = outputs_under(tmp.path());
    replay_report(tmp.path(), "semantic.json", report, false)?;
    let html = fs::read_to_string(&out.html)?;
    assert!(html.contains("Same behavior, different code"));
    assert!(html.contains("AI match"));
    Ok(())
}

// Implements [PIPELINE-CLUSTER-EXACT] cross-cluster overlap collapse — issue #33.
//
// When the same physical code is fingerprinted at multiple AST depths, two
// separate clusters can form whose occurrence sets are in a containment
// relationship: every occurrence of cluster A appears (same file, same byte
// range) inside an occurrence of cluster B.  `collapse_cross_cluster_overlap`
// must remove A (the redundant inner cluster) so the report never carries two
// clusters representing the same duplicated region.
//
// The bug this guards: the collapse only checked whether the *lower-weight*
// cluster's occurrences were inside the *higher-weight* cluster's, never the
// reverse.  When the higher-weight cluster happened to be the logically-inner
// one (a strict occurrence-subset of the lower-weight cluster), both survived.
#[test]
fn cross_cluster_collapse_removes_occurrence_subset_clusters() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&fixture("csharp-prologue-false-positive"), &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "2"])
        .assert()
        .success();
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
