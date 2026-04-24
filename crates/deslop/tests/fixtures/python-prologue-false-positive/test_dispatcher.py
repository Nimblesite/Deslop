"""Dispatcher concurrency test (bug #34)."""

from __future__ import annotations

import pytest

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from httpx import AsyncClient
    from sqlalchemy.ext.asyncio import AsyncSession


async def test_concurrent_dispatcher_invocations() -> None:
    """gather three distinct dispatches and check every result."""
    dispatcher = Dispatcher()
    coroutines = [
        dispatcher.dispatch({"kind": "sandbox"}),
        dispatcher.dispatch({"kind": "sandbox"}),
        dispatcher.dispatch({"kind": "vision"}),
    ]
    results = await asyncio.gather(*coroutines)
    assert len(results) == 3
    for outcome in results:
        assert outcome.ok is True
