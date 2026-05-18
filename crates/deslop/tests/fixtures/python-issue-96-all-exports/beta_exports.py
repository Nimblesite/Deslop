"""Beta module exports."""

from __future__ import annotations

__all__ = [
    "BetaPublisher",
    "BetaPayload",
    "BetaError",
    "publish_beta_event",
    "normalise_beta_payload",
]


def beta_payload_keys(payload: dict[str, object]) -> list[str]:
    keys = sorted(payload)
    if "trace" in keys:
        keys.remove("trace")
    return keys
