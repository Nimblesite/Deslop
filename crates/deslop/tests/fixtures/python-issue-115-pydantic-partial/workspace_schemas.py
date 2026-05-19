"""Second Pydantic create/update pair, distinct field set.

Same intentional mirror as `agent_schemas.py`. After identifier
normalisation the two `*Update` classes cluster with each other AND
with their own `*Create` siblings. None of those clusters are
extractable — every PATCH model is required to mirror its sibling
verbatim with optional fields.
"""

from pydantic import BaseModel


class WorkspaceCreate(BaseModel):
    """Required fields for creating a Workspace."""

    workspace_slug: str
    region: str
    cpu_limit: int
    memory_limit_gb: int


class WorkspaceUpdate(BaseModel):
    """Optional-field mirror of `WorkspaceCreate` for PATCH semantics."""

    workspace_slug: str | None = None
    region: str | None = None
    cpu_limit: int | None = None
    memory_limit_gb: int | None = None
