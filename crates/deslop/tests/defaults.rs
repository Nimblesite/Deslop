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
fn default_run_hides_alembic_migration_only_clusters() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let root = tmp.path().join("repo");
    let versions = root.join("alembic").join("versions");
    fs::create_dir_all(&versions)?;
    fs::write(
        versions.join("001_initial_schema.py"),
        ALEMBIC_INITIAL_SCHEMA,
    )?;

    let report = run_report(&root, tmp.path(), 30)?;

    assert_eq!(
        clusters(&report)?.len(),
        0,
        "Alembic migration-only clusters must be hidden: {report}"
    );
    assert!(
        field(&report, "clusters_hidden")
            .as_u64()
            .is_some_and(|count| count > 0),
        "hidden Alembic clusters must be counted: {report}",
    );
    assert_eq!(
        metrics_field(&report, "duplicated_loc").as_u64(),
        Some(0),
        "Alembic migration-only duplication must not inflate headline duplicated LOC: {report}",
    );
    Ok(())
}

#[test]
fn default_run_hides_python_generated_suffix_clusters() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let root = tmp.path().join("repo");
    fs::create_dir_all(root.join("src/agent_backend/api"))?;
    fs::write(
        root.join("src/agent_backend/api/schemas_generated.py"),
        PYTHON_SCHEMAS_GENERATED,
    )?;
    fs::write(
        root.join("src/agent_backend/api/handlers.py"),
        PYTHON_HAND_WRITTEN_HANDLER,
    )?;

    let report = run_report(&root, tmp.path(), 8)?;
    let visible_clusters = clusters(&report)?;

    assert!(
        field(&report, "clusters_hidden")
            .as_u64()
            .is_some_and(|count| count > 0),
        "generated-only Python clusters must be counted as hidden: {report}",
    );
    assert!(
        visible_clusters
            .iter()
            .any(has_hidden_and_visible_occurrences),
        "mixed generated/hand-written Python clusters must remain visible: {report}",
    );
    assert!(
        visible_clusters
            .iter()
            .all(|cluster| !all_occurrences_hidden(cluster)),
        "visible clusters must not be generated-only: {report}",
    );
    assert!(
        visible_clusters
            .iter()
            .flat_map(cluster_occurrences)
            .any(is_hidden_generated_python_occurrence),
        "mixed cluster must flag the generated Python occurrence as hidden: {report}",
    );
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(2),
        "both generated and hand-written Python files must still be analysed: {report}",
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

fn cluster_occurrences(cluster: &Value) -> impl Iterator<Item = &Value> {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn has_hidden_and_visible_occurrences(cluster: &Value) -> bool {
    let mut hidden = false;
    let mut visible = false;
    for occurrence in cluster_occurrences(cluster) {
        match field(occurrence, "hidden").as_bool() {
            Some(true) => hidden = true,
            Some(false) => visible = true,
            None => {}
        }
    }
    hidden && visible
}

fn all_occurrences_hidden(cluster: &Value) -> bool {
    let mut seen = false;
    let mut all_hidden = true;
    for occurrence in cluster_occurrences(cluster) {
        seen = true;
        all_hidden &= field(occurrence, "hidden").as_bool() == Some(true);
    }
    seen && all_hidden
}

fn is_hidden_generated_python_occurrence(occurrence: &Value) -> bool {
    let path = field(occurrence, "path").as_str().unwrap_or_default();
    path.ends_with("schemas_generated.py") && field(occurrence, "hidden").as_bool() == Some(true)
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

const ALEMBIC_INITIAL_SCHEMA: &str = r#"
"""Initial schema."""

from __future__ import annotations

from typing import TYPE_CHECKING

import sqlalchemy as sa
from alembic import op
from sqlalchemy.dialects import postgresql

if TYPE_CHECKING:
    from collections.abc import Sequence

revision: str = "001"
down_revision: str | None = None
branch_labels: str | Sequence[str] | None = None
depends_on: str | Sequence[str] | None = None


def upgrade() -> None:
    op.create_table(
        "tenants",
        sa.Column("id", postgresql.UUID(as_uuid=True), primary_key=True),
        sa.Column("name", sa.String(), nullable=False),
        sa.Column("api_key_hash", sa.String(), nullable=False, unique=True),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.func.now()),
    )
    op.create_index("ix_tenants_api_key_hash", "tenants", ["api_key_hash"])

    op.create_table(
        "agent_configs",
        sa.Column("id", postgresql.UUID(as_uuid=True), primary_key=True),
        sa.Column(
            "tenant_id",
            postgresql.UUID(as_uuid=True),
            sa.ForeignKey("tenants.id"),
            nullable=False,
        ),
        sa.Column("name", sa.String(), nullable=False),
        sa.Column("system_prompt", sa.Text(), nullable=False, server_default=""),
        sa.Column("model_config", postgresql.JSON(), nullable=False),
        sa.Column("tools_config", postgresql.JSON(), nullable=False, server_default="[]"),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.func.now()),
        sa.Column("updated_at", sa.DateTime(timezone=True), server_default=sa.func.now()),
    )
    op.create_index("ix_agent_configs_tenant_id", "agent_configs", ["tenant_id"])

    op.create_table(
        "conversations",
        sa.Column("id", postgresql.UUID(as_uuid=True), primary_key=True),
        sa.Column(
            "tenant_id",
            postgresql.UUID(as_uuid=True),
            sa.ForeignKey("tenants.id"),
            nullable=False,
        ),
        sa.Column(
            "config_id",
            postgresql.UUID(as_uuid=True),
            sa.ForeignKey("agent_configs.id"),
            nullable=False,
        ),
        sa.Column("session_id", sa.String(), nullable=False, unique=True),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.func.now()),
        sa.Column("updated_at", sa.DateTime(timezone=True), server_default=sa.func.now()),
    )
    op.create_index("ix_conversations_tenant_id", "conversations", ["tenant_id"])
    op.create_index("ix_conversations_config_id", "conversations", ["config_id"])
    op.create_index("ix_conversations_session_id", "conversations", ["session_id"])

    op.create_table(
        "messages",
        sa.Column("id", postgresql.UUID(as_uuid=True), primary_key=True),
        sa.Column(
            "conversation_id",
            postgresql.UUID(as_uuid=True),
            sa.ForeignKey("conversations.id"),
            nullable=False,
        ),
        sa.Column("role", sa.String(), nullable=False),
        sa.Column("content", sa.Text(), nullable=False),
        sa.Column("tool_calls", postgresql.JSON(), nullable=True),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.func.now()),
    )
    op.create_index("ix_messages_conversation_id", "messages", ["conversation_id"])

    op.create_table(
        "agent_logs",
        sa.Column("id", postgresql.UUID(as_uuid=True), primary_key=True),
        sa.Column(
            "conversation_id",
            postgresql.UUID(as_uuid=True),
            sa.ForeignKey("conversations.id"),
            nullable=False,
        ),
        sa.Column("level", sa.String(), nullable=False, server_default="info"),
        sa.Column("message", sa.Text(), nullable=False),
        sa.Column("metadata", postgresql.JSON(), nullable=True),
        sa.Column("created_at", sa.DateTime(timezone=True), server_default=sa.func.now()),
    )
    op.create_index("ix_agent_logs_conversation_id", "agent_logs", ["conversation_id"])


def downgrade() -> None:
    op.drop_table("agent_logs")
    op.drop_table("messages")
    op.drop_table("conversations")
    op.drop_table("agent_configs")
    op.drop_table("tenants")
"#;

const PYTHON_SCHEMAS_GENERATED: &str = r#"
"""Generated API schema. DO NOT HAND-EDIT.

Regenerate with scripts/generate_api_schema.py.
"""

from __future__ import annotations


def generated_alpha(payload: dict[str, str]) -> dict[str, str]:
    result: dict[str, str] = {}
    result["id"] = payload.get("id", "")
    result["tenant_id"] = payload.get("tenant_id", "")
    result["display_name"] = payload.get("display_name", "")
    result["description"] = payload.get("description", "")
    result["created_at"] = payload.get("created_at", "")
    result["updated_at"] = payload.get("updated_at", "")
    return result


def generated_beta(payload: dict[str, str]) -> dict[str, str]:
    result: dict[str, str] = {}
    result["id"] = payload.get("id", "")
    result["tenant_id"] = payload.get("tenant_id", "")
    result["display_name"] = payload.get("display_name", "")
    result["description"] = payload.get("description", "")
    result["created_at"] = payload.get("created_at", "")
    result["updated_at"] = payload.get("updated_at", "")
    return result


def generated_lookup(row: dict[str, str]) -> str:
    name = row.get("name", "")
    tenant = row.get("tenant", "")
    if tenant:
        return f"{tenant}:{name}"
    return name
"#;

const PYTHON_HAND_WRITTEN_HANDLER: &str = r#"
from __future__ import annotations


def manual_lookup(row: dict[str, str]) -> str:
    name = row.get("name", "")
    tenant = row.get("tenant", "")
    if tenant:
        return f"{tenant}:{name}"
    return name
"#;

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
