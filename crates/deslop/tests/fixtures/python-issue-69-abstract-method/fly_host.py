from typing import Any
from host_base import AgentWorkspaceHost


class FlyAgentWorkspaceHost(AgentWorkspaceHost):
    async def tool_call(
        self,
        *,
        instance: object,
        name: str,
        arguments: dict[str, Any],
        trace_id: str | None = None,
    ) -> object:
        machine = self.machines[instance]
        response = await machine.execute(name, arguments)
        return self.fly_result(response, trace_id)
