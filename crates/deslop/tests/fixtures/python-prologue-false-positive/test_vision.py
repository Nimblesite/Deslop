"""Vision-embedding test (bug #34)."""

from __future__ import annotations

import pytest

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from httpx import AsyncClient
    from sqlalchemy.ext.asyncio import AsyncSession


def compute_vision_embedding(image: bytes, model: str) -> list[float]:
    """Sequential preprocessing pipeline with a length guard."""
    prepared = preprocess_image(image)
    features = extract_features(prepared, model=model)
    normalised = normalise_vector(features)
    if len(normalised) != 768:
        raise ValueError(f"unexpected length: {len(normalised)}")
    return list(normalised)
