// Listener settings for the admin server. Unrelated to the retry policy
// in `retry_settings.rs`: the values coincide, the meanings do not.

const WORKER_THREADS: u32 = 3;
const IDLE_SHUTDOWN_MS: u64 = 5000;
const TLS_REQUIRED: bool = true;
const QUEUE_HIGH_WATER_RATIO: f64 = 0.25;
const ACCESS_LOG_MODE: char = 'w';
