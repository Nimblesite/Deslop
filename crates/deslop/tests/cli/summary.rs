//! [UX-CLUSTER-IDENTITY] Summary handles survive changes to another group's rank.
use super::support::*;
use crate::common::{clusters, deslop_cmd};
use anyhow::Context as _;

const FIXTURE: &str = "rank-mass";
const SLUG_LENGTH: usize = 7;
const GROUPS: usize = 2;
const FIRST_RANK: u64 = 1;
const SECOND_RANK: u64 = 2;
const UNCHANGED_FILE: &str = "mid_0.ts";
const ORIGINAL_COPY: &str = "big_a.ts";
const ADDED_COPY: &str = "big_c.ts";
const FIXTURE_MIN_NODES: &str = "15";
const SERIAL_WORKERS: &str = "1";

#[test]
fn summaries_keep_cluster_handles_when_another_group_changes_the_rank() -> Result<()> {
    let (temp, root) = seeded_fixture_root(FIXTURE)?;
    let base = temp.path().join(REPORT_OUTPUT_STEM);
    let initial = assert_modes(&root, &base, FIRST_RANK)?;
    add_copy(&root)?;
    let reordered = assert_modes(&root, &base, SECOND_RANK)?;
    assert_eq!(
        initial, reordered,
        "the unchanged group retains its quoted handle"
    );
    Ok(())
}

fn assert_modes(root: &Path, base: &Path, expected_rank: u64) -> Result<String> {
    let mut identity = None;
    for technical in [false, true] {
        let (report, summary) = scan(root, base, technical)?;
        assert_rows(&report, &summary)?;
        let id = unchanged_identity(&report, expected_rank)?;
        if let Some(previous) = &identity {
            assert_eq!(previous, id, "verbosity preserves identity");
        }
        identity = Some(id.to_owned());
    }
    identity.context("both CLI modes exercised")
}

fn unchanged_identity(report: &Value, expected_rank: u64) -> Result<&str> {
    assert_eq!(
        clusters(report).len(),
        GROUPS,
        "both fixture families survive"
    );
    let unchanged = unchanged_cluster(report)?;
    assert_eq!(
        field(unchanged, "rank"),
        expected_rank,
        "the mutation changes ordering"
    );
    field(unchanged, "id").as_str().context("canonical id")
}

fn add_copy(root: &Path) -> Result<()> {
    let original = root.join(ORIGINAL_COPY);
    let copied = fs::copy(&original, root.join(ADDED_COPY))?;
    assert_eq!(
        copied,
        fs::metadata(original)?.len(),
        "copy the complete source"
    );
    Ok(())
}

fn scan(root: &Path, base: &Path, technical: bool) -> Result<(Value, String)> {
    let mut command = deslop_cmd(root, base)?;
    let assertion = command
        .args([
            MIN_NODES_FLAG,
            FIXTURE_MIN_NODES,
            NO_COLOR_FLAG,
            "--embeddings",
            "off",
            "--jobs",
            SERIAL_WORKERS,
        ])
        .args(technical.then_some("--technical"))
        .assert()
        .success();
    let summary = String::from_utf8(assertion.get_output().stderr.clone())?;
    Ok((read_json_report(&base.with_extension("json"))?, summary))
}

fn assert_rows(report: &Value, summary: &str) -> Result<()> {
    for cluster in clusters(report) {
        let id = field(cluster, "id").as_str().context("cluster id")?;
        let slug = id.get(..SLUG_LENGTH).context("seven-character slug")?;
        let prefix = format!("{slug} ");
        let row = summary
            .lines()
            .map(str::trim)
            .find(|line| line.starts_with(&prefix));
        let row = row.with_context(|| format!("summary must lead with {slug}: {summary}"))?;
        let rank = field(cluster, "rank").as_u64().context("rank")?;
        assert!(
            row.contains(&format!("rank #{rank}")),
            "rank is explicitly labelled: {row}"
        );
    }
    Ok(())
}

fn unchanged_cluster(report: &Value) -> Result<&Value> {
    clusters(report)
        .iter()
        .find(|cluster| {
            field(cluster, "occurrences")
                .as_array()
                .is_some_and(|occurrences| {
                    occurrences.iter().any(|occurrence| {
                        field(occurrence, "path")
                            .as_str()
                            .is_some_and(|path| path.ends_with(UNCHANGED_FILE))
                    })
                })
        })
        .context("unchanged five-file fixture family")
}
