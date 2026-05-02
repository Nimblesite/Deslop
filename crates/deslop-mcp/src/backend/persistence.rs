//! Settings JSON persistence helpers for shared embedding configuration.

use std::{fs, path::Path};

use deslop_core::EmbeddingSpec;

use super::BackendError;

/// Writes shared embedding selection settings for sibling surfaces.
pub(super) fn persist_shared_embedding_settings(
    root: &Path,
    spec: &EmbeddingSpec,
    endpoint: Option<&str>,
) -> Result<(), BackendError> {
    let path = root.join(".vscode").join("settings.json");
    let mut settings = read_settings_object(&path)?;
    let _old_provider = settings.insert(
        "deslop.embedding.provider".to_owned(),
        serde_json::Value::String(spec.provider_id.clone()),
    );
    let _old_model = settings.insert(
        "deslop.embedding.model".to_owned(),
        serde_json::Value::String(spec.model_id.clone()),
    );
    let _old_mode = settings.insert(
        "deslop.embedding.mode".to_owned(),
        serde_json::Value::String("auto".to_owned()),
    );
    if let Some(ep) = endpoint {
        let _old_endpoint = settings.insert(
            "deslop.embedding.endpoint".to_owned(),
            serde_json::Value::String(ep.to_owned()),
        );
    }
    write_settings_object(&path, settings)
}

/// Reads `.vscode/settings.json` as a mutable JSON object.
pub(super) fn read_settings_object(
    path: &Path,
) -> Result<serde_json::Map<String, serde_json::Value>, BackendError> {
    if !path.exists() {
        return Ok(serde_json::Map::new());
    }
    let source = fs::read_to_string(path).map_err(|error| config_write_error(path, error))?;
    let value: serde_json::Value =
        serde_json::from_str(&source).map_err(|error| config_write_error(path, error))?;
    Ok(value.as_object().cloned().unwrap_or_default())
}

/// Writes a settings object back to disk, creating `.vscode` first.
pub(super) fn write_settings_object(
    path: &Path,
    settings: serde_json::Map<String, serde_json::Value>,
) -> Result<(), BackendError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| config_write_error(path, error))?;
    }
    let encoded = serde_json::to_vec_pretty(&serde_json::Value::Object(settings))
        .map_err(|error| config_write_error(path, error))?;
    fs::write(path, encoded).map_err(|error| config_write_error(path, error))
}

/// Converts any settings persistence error into a backend error.
pub(super) fn config_write_error(path: &Path, error: impl std::fmt::Display) -> BackendError {
    BackendError::ConfigWrite {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}
