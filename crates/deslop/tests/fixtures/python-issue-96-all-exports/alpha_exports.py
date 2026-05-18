"""Alpha module exports."""

from __future__ import annotations

__all__ = [
    "AlphaClient",
    "AlphaConfig",
    "AlphaError",
    "build_alpha_client",
    "load_alpha_config",
]


def alpha_retry_window(attempts: int) -> int:
    limit = attempts * 3
    if limit > 24:
        return 24
    return limit + 2
