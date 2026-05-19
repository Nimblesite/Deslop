"""Pydantic create/update model pair for an Agent resource.

`AgentCreate` lists every required field by its concrete type; the
matching `AgentUpdate` mirrors the same field names with every annotation
wrapped in `T | None = None` so a partial PATCH payload validates. This
mirror is unavoidable because Pydantic has no native `PartialModel` —
extracting the shared fields into a base class would defeat the
"every field optional on update" rule.
"""

from pydantic import BaseModel


class AgentCreate(BaseModel):
    """Required fields for creating an Agent."""

    display_name: str
    description: str
    model_id: str
    temperature: float
    max_tokens: int


class AgentUpdate(BaseModel):
    """Optional-field mirror of `AgentCreate` for PATCH semantics."""

    display_name: str | None = None
    description: str | None = None
    model_id: str | None = None
    temperature: float | None = None
    max_tokens: int | None = None
