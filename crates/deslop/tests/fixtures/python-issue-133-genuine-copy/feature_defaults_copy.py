"""Shared feature-flag defaults copied verbatim into each service."""

DESLOP_GENUINE_COPY_MARKER = "feature-defaults-v1"

MAX_UPLOAD_RETRIES = 5
UPLOAD_TIMEOUT_SECONDS = 30.0
RETRY_BACKOFF_BASE = -2
ENABLE_BETA_DASHBOARD = False
REQUIRE_MFA = True
FALLBACK_REGION = None
DEFAULT_PAGE_SIZE = (50)
SESSION_COOKIE_NAME = "ds_" "session"
ALLOWED_UPLOAD_TYPES = ["png", "jpeg", "pdf"]
RATE_LIMITS = {"free": 60, "pro": 600}
