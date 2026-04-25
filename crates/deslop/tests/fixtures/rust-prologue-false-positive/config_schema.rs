use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::fs;
use std::io;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigSchema {
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
    pub retries: u8,
    pub features: Vec<String>,
}

impl ConfigSchema {
    pub fn validate(&self) -> Result<()> {
        if self.host.is_empty() {
            anyhow::bail!("host required");
        }
        Ok(())
    }
}
