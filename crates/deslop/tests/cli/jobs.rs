//! [CLI-ARG-JOBS] `--jobs <N>` caps every parallel stage of a scan at `N`
//! worker threads, so the CLI can run beside an IDE or a build without
//! taking every core, and the report is the same for every budget.
//!
//! Black box: the budget is read back from the scan's own debug log, where
//! every parallel stage names the workers it started, and the reports are
//! compared field for field (gh #578, gh #604).

use super::support::*;
use crate::common::deslop_cmd;

const JOBS_FLAG: &str = "--jobs";
const LOG_TO_CONSOLE_FLAG: &str = "--log-to-console";
const LOG_LEVEL_FLAG: &str = "--log-level";
const DEBUG_LEVEL: &str = "debug";
const EMBEDDINGS_FLAG: &str = "--embeddings";
const EMBEDDINGS_OFF: &str = "off";
/// The log field every parallel stage reports its thread count in.
const WORKERS_FIELD: &str = "workers=";
/// The stage that fingerprints the corpus, one file per task.
const FINGERPRINT_STAGE: &str = "fingerprint_workers";
/// The budgets exercised: one thread, and fewer than the files.
const SERIAL: usize = 1;
const SMALL_BUDGET: usize = 2;
/// The workload: enough files that every budget is the binding limit.
const COPIED_FILES: usize = 24;
const DISTINCT_FILES: usize = 24;
const SCAN_DIR: &str = "workload";
/// A budget of no workers, and the usage-error status clap refuses it with.
const NO_WORKERS: &str = "0";
const USAGE_EXIT: i32 = 2;

/// A function copied into `COPIED_FILES` files, beside `DISTINCT_FILES`
/// unrelated ones: one cluster, and plenty of files to fingerprint.
fn write_workload(root: &Path) -> Result<()> {
    for index in 0..COPIED_FILES {
        fs::write(
            root.join(format!("copy_{index:02}.py")),
            "def summarise(rows, label):\n    total = 0\n    seen = []\n    for row in rows:\n        if row.get(\"label\") == label:\n            total += row.get(\"amount\", 0) * 3\n            seen.append(row.get(\"id\"))\n    return {\"label\": label, \"total\": total, \"seen\": seen}\n",
        )?;
    }
    for index in 0..DISTINCT_FILES {
        fs::write(
            root.join(format!("distinct_{index:02}.py")),
            format!("def only_{index}(value):\n    return value * {index} + {index}{index}\n"),
        )?;
    }
    Ok(())
}

/// One scan of `root` with `extra` arguments, returning its JSON report and stderr.
fn scan(root: &Path, extra: &[&str]) -> Result<(Value, String)> {
    let output = tempfile::tempdir()?;
    let mut command = deslop_cmd(root, &output.path().join(REPORT_OUTPUT_STEM))?;
    let assertion = command
        .args([
            MIN_NODES_FLAG,
            MIN_NODES_VALUE,
            EMBEDDINGS_FLAG,
            EMBEDDINGS_OFF,
            NO_COLOR_FLAG,
        ])
        .args(extra)
        .assert()
        .success();
    let stderr = String::from_utf8(assertion.get_output().stderr.clone())?;
    let report = read_json_report(&with_ext(&output.path().join(REPORT_OUTPUT_STEM), "json"))?;
    Ok((report, stderr))
}

/// Every worker count a stage logged, with the line it came from.
fn logged_workers(stderr: &str) -> Result<Vec<(usize, String)>> {
    stderr
        .lines()
        .filter_map(|line| line.split_once(WORKERS_FIELD).map(|(_, rest)| (rest, line)))
        .map(|(rest, line)| {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            Ok((digits.parse()?, line.to_owned()))
        })
        .collect()
}

/// The parts of a report that are its findings and figures.
fn findings(report: &Value) -> Value {
    serde_json::json!({
        "clusters": report.get("clusters"),
        "metrics": report.get("metrics"),
        "files_analysed": report.get("files_analysed"),
    })
}

#[test]
fn every_parallel_stage_stays_within_the_jobs_budget() -> Result<()> {
    let (_tmp, root) = temp_scan_dir(SCAN_DIR)?;
    write_workload(&root)?;
    for budget in [SERIAL, SMALL_BUDGET] {
        let jobs = budget.to_string();
        let (_report, stderr) = scan(
            &root,
            &[
                JOBS_FLAG,
                &jobs,
                LOG_LEVEL_FLAG,
                DEBUG_LEVEL,
                LOG_TO_CONSOLE_FLAG,
            ],
        )?;
        let workers = logged_workers(&stderr)?;
        assert!(
            workers
                .iter()
                .any(|(_, line)| line.contains(FINGERPRINT_STAGE)),
            "--jobs {budget}: the fingerprint stage must report its workers; stderr: {stderr}"
        );
        for (count, line) in &workers {
            assert!(
                *count <= budget,
                "--jobs {budget}: a stage started {count} workers: {line}"
            );
        }
        let fingerprint = workers
            .iter()
            .find(|(_, line)| line.contains(FINGERPRINT_STAGE))
            .map(|(count, _)| *count);
        assert_eq!(
            fingerprint,
            Some(budget),
            "--jobs {budget}: {} files fill every one of the {budget} fingerprint workers",
            COPIED_FILES + DISTINCT_FILES
        );
    }
    Ok(())
}

#[test]
fn the_report_is_identical_for_every_jobs_budget() -> Result<()> {
    let (_tmp, root) = temp_scan_dir(SCAN_DIR)?;
    write_workload(&root)?;
    let (unbounded, _) = scan(&root, &[])?;
    let clusters = unbounded
        .get("clusters")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    assert!(
        clusters > 0,
        "control: the workload reports its copied function: {unbounded}"
    );
    for budget in [SERIAL, SMALL_BUDGET] {
        let jobs = budget.to_string();
        let (bounded, _) = scan(&root, &[JOBS_FLAG, &jobs])?;
        assert_eq!(
            findings(&bounded),
            findings(&unbounded),
            "--jobs {budget} must not change a single finding or figure"
        );
    }
    Ok(())
}

#[test]
fn a_zero_jobs_budget_is_refused() -> Result<()> {
    let (_tmp, root) = temp_scan_dir(SCAN_DIR)?;
    write_workload(&root)?;
    let output = tempfile::tempdir()?;
    let mut command = deslop_cmd(&root, &output.path().join(REPORT_OUTPUT_STEM))?;
    let _assertion = command
        .args([JOBS_FLAG, NO_WORKERS])
        .assert()
        .failure()
        .code(USAGE_EXIT)
        .stderr(contains(JOBS_FLAG));
    Ok(())
}
