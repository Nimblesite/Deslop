from typing import Any
from host_base import AgentWorkspaceHost


class DockerAgentWorkspaceHost(AgentWorkspaceHost):
    async def tool_call(
        self,
        *,
        instance: object,
        name: str,
        arguments: dict[str, Any],
        trace_id: str | None = None,
    ) -> object:
        container = self.containers[instance]
        return await container.invoke(name, arguments)
