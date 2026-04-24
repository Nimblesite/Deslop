//! End-to-end regression coverage for issue #3: rendered fused scores
//! must stay in the public confidence range `[0, 1]`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use assert_cmd::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn report_path(tmp: &Path) -> PathBuf {
    let mut path = tmp.join("report");
    let _replaced = path.set_extension("json");
    path
}

fn run_report(tmp: &Path) -> Result<serde_json::Value> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(fixture("csharp-small"))
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.join("report"))
        .assert()
        .success();
    let body = fs::read_to_string(report_path(tmp))?;
    Ok(serde_json::from_str(&body)?)
}

fn first_unbounded_fused(report: &serde_json::Value) -> Option<String> {
    let clusters = report.get("clusters")?.as_array()?;
    for cluster in clusters {
        let id = cluster
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("?");
        let fused = cluster
            .get("signals")?
            .get("fused")?
            .as_f64()
            .unwrap_or(f64::NAN);
        if !(0.0..=1.0).contains(&fused) {
            return Some(format!("cluster {id} reports fused={fused}"));
        }
    }
    None
}

// Implements [FUSION-STRATEGY-MAX-SUM]: component scores are public
// confidence signals in [0, 1], and the fused confidence reported to
// agents must be bounded to the same range.
#[test]
fn rendered_fused_scores_are_bounded_to_unit_interval() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let report = run_report(tmp.path())?;
    assert!(
        first_unbounded_fused(&report).is_none(),
        "fused scores must be bounded to [0, 1]: {}",
        first_unbounded_fused(&report).unwrap_or_default()
    );
    Ok(())
}
