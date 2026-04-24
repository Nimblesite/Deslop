"""Synchronous builtin-registry test (bug #34)."""

from __future__ import annotations

import pytest

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from httpx import AsyncClient
    from sqlalchemy.ext.asyncio import AsyncSession


def test_register_builtin_tools() -> None:
    """List + for-loop assertion — structurally unlike the others."""
    register_builtin_tools()
    expected_tools = [
        "get_current_time",
        "echo",
        "search_emails",
        "draft_reply",
        "analyze_todos",
        "schedule_tasks",
    ]
    for tool_name in expected_tools:
        assert registry.has(tool_name), f"Tool {tool_name} should be registered"
