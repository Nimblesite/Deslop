"""Config builder test (bug #34)."""

from __future__ import annotations

import pytest

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from httpx import AsyncClient
    from sqlalchemy.ext.asyncio import AsyncSession


class ConfigBuilder:
    """Fluent builder — unlike a function or for-loop shape."""

    def __init__(self, tenant_id: uuid.UUID) -> None:
        self.tenant_id = tenant_id
        self.name = "default"
        self.model = "gpt-4"

    def with_name(self, name: str) -> "ConfigBuilder":
        self.name = name
        return self

    def build(self) -> AgentConfig:
        return AgentConfig(
            tenant_id=self.tenant_id,
            name=self.name,
            model=self.model,
        )
