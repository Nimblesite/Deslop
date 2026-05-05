"""Gamma module exports."""

from __future__ import annotations

__all__ = [
    "GammaRenderer",
    "GammaTheme",
    "GammaError",
    "render_gamma_card",
    "load_gamma_theme",
]


def gamma_heading(title: str, suffix: str | None) -> str:
    clean = title.strip().title()
    if suffix is None:
        return clean
    return f"{clean} - {suffix.strip()}"
