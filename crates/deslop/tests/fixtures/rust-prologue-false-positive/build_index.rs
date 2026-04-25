use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::fs;
use std::io;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

pub fn build_index(paths: &[PathBuf]) -> Result<HashMap<String, usize>> {
    let mut counts = HashMap::new();
    for path in paths {
        let key = path.display().to_string();
        let entry = counts.entry(key).or_insert(0);
        *entry += 1;
    }
    info!(total = counts.len(), "indexed");
    Ok(counts)
}
