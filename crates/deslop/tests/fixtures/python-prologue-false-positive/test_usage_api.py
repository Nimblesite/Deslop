"""Async UsageEvent helper (bug #34)."""

from __future__ import annotations

import pytest

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from httpx import AsyncClient
    from sqlalchemy.ext.asyncio import AsyncSession


async def _insert_event(
    db: AsyncSession,
    *,
    tenant_id: uuid.UUID,
    agent_config_id: uuid.UUID,
    kind: str,
    quantity: int,
    unit: str,
    when: _dt.datetime,
) -> None:
    """Seven typed params then two body statements — unlike the others."""
    db.add(
        UsageEvent(
            tenant_id=tenant_id,
            agent_config_id=agent_config_id,
            conversation_id=None,
            kind=kind,
            quantity=quantity,
            unit=unit,
            created_at=when,
        )
    )
    await db.flush()
