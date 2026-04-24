"""Mirror of the real test_usage_api.py that triggers issue #34."""

from __future__ import annotations

import datetime as _dt
import uuid
from typing import TYPE_CHECKING

import pytest

from agent_backend.db.models import AgentConfig, Tenant, UsageEvent

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
    """Async helper: insert one UsageEvent row and flush the session."""
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


@pytest.fixture
async def second_tenant(db_session: AsyncSession) -> Tenant:
    """Async pytest fixture: adds a second tenant and commits."""
    t = Tenant(
        id=uuid.uuid4(),
        name="Other Tenant",
    )
    db_session.add(t)
    await db_session.commit()
    await db_session.refresh(t)
    return t


@pytest.fixture
async def second_agent_config(db_session: AsyncSession, second_tenant: Tenant) -> AgentConfig:
    """Async pytest fixture: adds an AgentConfig owned by the second tenant."""
    cfg = AgentConfig(
        id=uuid.uuid4(),
        tenant_id=second_tenant.id,
        name="Other Agent",
        system_prompt="",
    )
    db_session.add(cfg)
    await db_session.commit()
    await db_session.refresh(cfg)
    return cfg
