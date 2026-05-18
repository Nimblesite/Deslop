from typing import Any
from host_base import AgentWorkspaceHost  # type: ignore[import-not-found]


class MockAgentWorkspaceHost(AgentWorkspaceHost):
    async def tool_call(
        self,
        *,
        instance: object,
        name: str,
        arguments: dict[str, Any],
        trace_id: str | None = None,
    ) -> object:
        self.calls.append((instance, name, arguments, trace_id))
        return {"status": "mocked"}
