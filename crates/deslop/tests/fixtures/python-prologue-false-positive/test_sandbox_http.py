"""Sandbox HTTP roundtrip test (bug #34)."""

from __future__ import annotations

import pytest

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from httpx import AsyncClient
    from sqlalchemy.ext.asyncio import AsyncSession


async def test_sandbox_http_roundtrip(async_client: AsyncClient) -> None:
    """Fire a request, assert the JSON body matches an expected payload."""
    response = await async_client.post(
        "/sandbox",
        json={"input": "hello", "kind": "echo"},
        headers={"x-tenant": "abc"},
    )
    assert response.status_code == 200
    body = response.json()
    assert body["output"] == "hello"
    assert body["kind"] == "echo"
    assert "trace_id" in body
