"""StrEnum declarations modelling agent types.

Each `class X(StrEnum)` has a different alphabet of members but the AST
shape (docstring + assignments only) is identical to every other
`StrEnum`. After identifier normalisation the bodies collapse and the
classes cluster as duplicates, when in reality each enum is a closed
discriminator the application code depends on by name.
"""

from enum import StrEnum


class AgentType(StrEnum):
    """Top-level agent dispatcher kinds."""

    CLAUDE = "claude"
    GPT = "gpt"
    GEMINI = "gemini"
    LOCAL = "local"


class WorkspaceHostKind(StrEnum):
    """Workspace host runtime kinds."""

    K8S = "k8s"
    DOCKER = "docker"
    LOCAL_FS = "local_fs"
    REMOTE_SSH = "remote_ssh"
