from typing import Any
from host_base import AgentWorkspaceHost  # type: ignore[import-not-found]


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
        body = {"name": name, "arguments": arguments}
        return await container.invoke(body, trace_id)
