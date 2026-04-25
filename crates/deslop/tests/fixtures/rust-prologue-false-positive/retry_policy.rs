use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::fs;
use std::io;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

pub struct RetryPolicy {
    max_attempts: u32,
    base_delay_ms: u64,
}

impl RetryPolicy {
    pub fn new(max_attempts: u32, base_delay_ms: u64) -> Self {
        Self { max_attempts, base_delay_ms }
    }

    pub fn delay_for(&self, attempt: u32) -> std::time::Duration {
        let exponent = attempt.min(self.max_attempts);
        std::time::Duration::from_millis(self.base_delay_ms << exponent)
    }
}
