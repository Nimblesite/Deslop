use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::fs;
use std::io;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

pub async fn fetch_remote(url: &str) -> Result<Vec<u8>> {
    let response = reqwest::get(url).await?;
    response.error_for_status_ref()?;
    let bytes = response.bytes().await?.to_vec();
    info!(size = bytes.len(), "fetched");
    Ok(bytes)
}
