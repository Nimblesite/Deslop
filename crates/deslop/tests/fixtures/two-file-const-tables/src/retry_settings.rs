// Retry policy for the outbound HTTP client. Unrelated to the server
// settings in `server_settings.rs`: the values coincide, the meanings do not.

const MAX_RETRY_ATTEMPTS: u32 = 3;
const RETRY_BACKOFF_MS: u64 = 5000;
const RETRY_ON_TIMEOUT: bool = true;
const RETRY_JITTER_RATIO: f64 = 0.25;
const RETRY_LOG_LEVEL: char = 'w';
