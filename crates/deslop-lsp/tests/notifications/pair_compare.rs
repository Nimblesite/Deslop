//! [VSIX-PAIR-COMPARE] Completed discovery is reused by explicit comparisons.

use super::*;

const FIXTURE: &str = "csharp-small";
const REPORT_GET: &str = "deslop/reportGet";
const PAIR_COMPARE: &str = "deslop/pairCompare";
const REPORT_FINISHED: &str = "deslop/reportGet handler complete";
const DISCOVERY_FINISHED: &str = "pairs: post-lsh";
const RESULT_CLUSTERS: &str = "/result/clusters";
const MINIMUM_OCCURRENCES: usize = 2;

#[test]
fn completed_discovery_is_reused_by_repeated_pair_comparisons() -> Result<()> {
    let workspace = copy_fixture(FIXTURE)?;
    let mut child = session::spawn_logging_lsp(workspace.path(), &[])?;
    let (mut stdin, mut stdout, stderr) = take_io(&mut child)?;
    let result = compare_reported_pair(&mut stdin, &mut stdout);
    let logs = shutdown_and_read_stderr(child, stdin, stderr)?;
    result?;
    assert_discovery_reused(&logs)
}

fn compare_reported_pair(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
) -> Result<()> {
    let _initialized = handshake(stdin, stdout)?;
    let report = call(stdin, stdout, REPORT_GET, &serde_json::json!({}))?;
    let (left, right) = reported_pair(&report)?;
    for (left, right) in [(&left, &right), (&right, &left), (&left, &right)] {
        let params = serde_json::json!({ "left": left, "right": right });
        let compared = call(stdin, stdout, PAIR_COMPARE, &params)?;
        assert_comparison(&compared, left, right);
    }
    let _shutdown = session::shutdown(stdin, stdout)?;
    Ok(())
}

fn assert_comparison(
    compared: &serde_json::Value,
    left: &serde_json::Value,
    right: &serde_json::Value,
) {
    assert_eq!(compared.pointer("/result/left"), Some(left));
    assert_eq!(compared.pointer("/result/right"), Some(right));
    assert_eq!(
        compared.pointer("/result/evidence/admitted"),
        Some(&serde_json::json!(true))
    );
}

fn reported_pair(report: &serde_json::Value) -> Result<(serde_json::Value, serde_json::Value)> {
    let [left, right, ..] = pair_occurrences(report)?.as_slice() else {
        return Err(anyhow!("a reported pair must contain both occurrences"));
    };
    Ok((endpoint(left)?, endpoint(right)?))
}

fn pair_occurrences(report: &serde_json::Value) -> Result<&Vec<serde_json::Value>> {
    let clusters = report
        .pointer(RESULT_CLUSTERS)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("completed analysis must return clusters: {report}"))?;
    clusters
        .iter()
        .filter_map(|cluster| cluster.get("occurrences"))
        .filter_map(serde_json::Value::as_array)
        .find(|occurrences| occurrences.len() >= MINIMUM_OCCURRENCES)
        .ok_or_else(|| {
            anyhow!("completed analysis must include an actual duplicate pair: {report}")
        })
}

fn endpoint(occurrence: &serde_json::Value) -> Result<serde_json::Value> {
    let endpoint: deslop_core::report::PairEndpoint = serde_json::from_value(occurrence.clone())?;
    Ok(serde_json::to_value(endpoint)?)
}

fn assert_discovery_reused(logs: &str) -> Result<()> {
    let (startup, _) = logs
        .split_once(REPORT_FINISHED)
        .ok_or_else(|| anyhow!("server must finish reportGet before comparisons: {logs}"))?;
    let completed_passes = startup.matches(DISCOVERY_FINISHED).count();
    ensure!(
        completed_passes > 0,
        "the real server must finish discovery first: {logs}"
    );
    assert_eq!(
        logs.matches(DISCOVERY_FINISHED).count(),
        completed_passes,
        "repeated pair comparisons must reuse completed discovery, including reversed endpoints"
    );
    Ok(())
}
