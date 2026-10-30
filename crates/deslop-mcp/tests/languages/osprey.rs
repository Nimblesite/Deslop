//! [LANG-CAND-OSPREY] Real LSP/MCP binaries publish native Osprey findings.

#![cfg(unix)]

use crate::common::language_label::assert_language_label_over_mcp;
use anyhow::Result;

const FIXTURE: &str = "osprey-identical";
const EXTENSION: &str = "osp";
const LANGUAGE: &str = "osprey";
const CONTRACT: &str = "[LANG-CAND-OSPREY]";
const ML_FIXTURE: &str = "osprey-ml";
const ML_EXTENSION: &str = "ospml";
const ML_LANGUAGE: &str = "osprey-ml";
const PYTHON_MARKER_FIXTURE: &str = "osprey-python-marker";
const PYTHON_PROJECT_FIXTURE: &str = "osprey-python-project";
const PYTHON_EXTENSION: &str = "py";
const PYTHON_LANGUAGE: &str = "osprey-python";

#[test]
fn osprey_clusters_keep_their_language_over_mcp() -> Result<()> {
    assert_language_label_over_mcp(FIXTURE, EXTENSION, LANGUAGE, CONTRACT)
}

#[test]
fn osprey_ml_clusters_keep_their_language_over_mcp() -> Result<()> {
    assert_language_label_over_mcp(ML_FIXTURE, ML_EXTENSION, ML_LANGUAGE, CONTRACT)
}

#[test]
fn osprey_python_marker_keeps_its_language_over_mcp() -> Result<()> {
    assert_language_label_over_mcp(
        PYTHON_MARKER_FIXTURE,
        PYTHON_EXTENSION,
        PYTHON_LANGUAGE,
        CONTRACT,
    )
}

#[test]
fn osprey_python_project_keeps_its_language_over_mcp() -> Result<()> {
    assert_language_label_over_mcp(
        PYTHON_PROJECT_FIXTURE,
        PYTHON_EXTENSION,
        PYTHON_LANGUAGE,
        CONTRACT,
    )
}
