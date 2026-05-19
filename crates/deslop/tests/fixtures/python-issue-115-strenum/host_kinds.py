"""More StrEnum declarations on a separate module.

Cross-file cluster forms when these enum classes share the same
docstring + assignment-only shape with the ones in `agent_types.py`.
The filter must drop these so each enum stays as an independent contract.
"""

from enum import StrEnum


class AgentWorkspaceHostKind(StrEnum):
    """Agent-side workspace host discriminator."""

    K8S = "k8s"
    DOCKER = "docker"
    PODMAN = "podman"


class TaskState(StrEnum):
    """Lifecycle states for a scheduled task."""

    PENDING = "pending"
    RUNNING = "running"
    SUCCEEDED = "succeeded"
    FAILED = "failed"
