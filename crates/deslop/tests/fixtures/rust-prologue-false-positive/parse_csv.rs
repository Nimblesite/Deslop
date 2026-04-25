use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::fs;
use std::io;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

pub fn parse_csv(input: &str) -> Result<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let cells = trimmed.split(',').map(str::to_string).collect();
        rows.push(cells);
    }
    Ok(rows)
}
