"""Mirror of the real test_builtins.py that triggers issue #34."""

from __future__ import annotations

import pytest

from agent_backend.core.tool_registry import registry
from agent_backend.tools.builtins import register_builtin_tools


def test_register_builtin_tools() -> None:
    """Synchronous test: list literal + for-loop asserting registry state."""
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


def test_get_current_time_tool() -> None:
    """Synchronous test: fetch a tool, call it, assert shape of result."""
    register_builtin_tools()
    get_time = registry.get("get_current_time")
    result = get_time()
    assert isinstance(result, str)
    assert "T" in result
    assert result.endswith("+00:00") or "Z" in result


def test_echo_tool() -> None:
    """Synchronous test: round-trip a literal through the echo tool."""
    register_builtin_tools()
    echo = registry.get("echo")
    test_text = "Hello, World!"
    result = echo(test_text)
    assert result == test_text
