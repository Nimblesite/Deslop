//! E2E coverage for noisy default-analysis classes.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use assert_cmd::Command;
use serde_json::Value;

#[test]
fn default_run_hides_generated_only_clusters_from_metrics() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let root = tmp.path().join("repo");
    let generated = root.join("contracts/generated/csharp");
    fs::create_dir_all(&generated)?;
    fs::write(generated.join("Alpha.g.cs"), GENERATED_ALPHA)?;
    fs::write(generated.join("Beta.g.cs"), GENERATED_BETA)?;

    let report = run_report(&root, tmp.path(), 8)?;

    assert_eq!(
        clusters(&report)?.len(),
        0,
        "generated-only clusters must be hidden: {report}"
    );
    assert!(
        field(&report, "clusters_hidden")
            .as_u64()
            .is_some_and(|count| count > 0),
        "hidden generated clusters must be counted: {report}",
    );
    assert_eq!(
        metrics_field(&report, "duplicated_loc").as_u64(),
        Some(0),
        "generated-only duplication must not inflate headline duplicated LOC: {report}",
    );
    assert_eq!(
        metrics_field(&report, "duplication_percent").as_f64(),
        Some(0.0),
        "generated-only duplication must not inflate headline percent: {report}",
    );
    Ok(())
}

#[test]
fn default_run_does_not_cluster_distinct_fastapi_route_decorators() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let root = tmp.path().join("repo");
    fs::create_dir_all(root.join("backend/src"))?;
    fs::write(root.join("backend/src/routes.py"), FASTAPI_ROUTES)?;

    let report = run_report(&root, tmp.path(), 20)?;

    assert_eq!(
        clusters(&report)?.len(),
        0,
        "distinct FastAPI route declarations must not rank as duplicate code: {report}",
    );
    assert_eq!(metrics_field(&report, "duplicated_loc").as_u64(), Some(0));
    Ok(())
}

fn run_report(root: &Path, tmp: &Path, min_nodes: u32) -> Result<Value> {
    let output = tmp.join("report");
    let _assertion = Command::cargo_bin("deslop")?
        .arg(root)
        .arg("--min-nodes")
        .arg(min_nodes.to_string())
        .arg("--embeddings")
        .arg("off")
        .arg("--output")
        .arg(&output)
        .assert()
        .success();
    let json = fs::read_to_string(with_ext(&output, "json"))?;
    serde_json::from_str(&json).map_err(Into::into)
}

fn clusters(report: &Value) -> Result<&Vec<Value>> {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("clusters missing"))
}

fn metrics_field<'a>(report: &'a Value, key: &str) -> &'a Value {
    field(field(report, "metrics"), key)
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or(&Value::Null)
}

fn with_ext(base: &Path, ext: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    let mut name = path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".");
    name.push(ext);
    path.set_file_name(name);
    path
}

const GENERATED_ALPHA: &str = r"
namespace Contracts.Generated;

public sealed record AlphaDto(
    string Id,
    string TenantId,
    string DisplayName,
    string Description,
    string CreatedAt,
    string UpdatedAt
);
";

const GENERATED_BETA: &str = r"
namespace Contracts.Generated;

public sealed record BetaDto(
    string Id,
    string TenantId,
    string DisplayName,
    string Description,
    string CreatedAt,
    string UpdatedAt
);
";

const FASTAPI_ROUTES: &str = r#"
from fastapi import APIRouter

router = APIRouter()

@router.get(
    "/tenants/me",
    tags=["tenants"],
    summary="Read the current tenant",
    description="Returns tenant metadata for the authenticated user.",
    responses={401: {"description": "Unauthorized"}, 503: {"description": "Unavailable"}},
)
async def read_current_tenant(user_id: str):
    return {"tenant": user_id, "kind": "tenant"}

@router.post(
    "/sessions/{session_id}/chat",
    tags=["sessions"],
    summary="Send a chat turn",
    description="Runs one AI chat turn for a stateful session.",
    responses={
        400: {"description": "Bad request"},
        401: {"description": "Unauthorized"},
        404: {"description": "Session not found"},
        410: {"description": "Session closed"},
        501: {"description": "Workspace unavailable"},
    },
)
async def chat_with_session(session_id: str, body: dict):
    return {"session": session_id, "tokens": len(body)}
"#;
