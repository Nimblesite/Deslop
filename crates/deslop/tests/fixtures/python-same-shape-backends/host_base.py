from abc import ABC, abstractmethod
from typing import Any


class AgentWorkspaceHost(ABC):
    @abstractmethod
    async def tool_call(
        self,
        *,
        instance: object,
        name: str,
        arguments: dict[str, Any],
        trace_id: str | None = None,
    ) -> object:
        ...
