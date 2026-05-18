"""Companion module declaring a fourth ORM table on a different file.

Mirrors the same `Mapped[T] = mapped_column(...)` shape so cross-file
clusters can form. Each column is a different attribute on a different
table; no two attribute names match the other module so the filter has
a distinct name set on every member.
"""

from datetime import datetime
from uuid import UUID, uuid4

from sqlalchemy import ForeignKey
from sqlalchemy.orm import Mapped, mapped_column


class AgentLog:
    log_id: Mapped[UUID] = mapped_column(primary_key=True, default=uuid4)
    log_conversation: Mapped[UUID] = mapped_column(ForeignKey("conversation.id"))
    severity_label: Mapped[str] = mapped_column(nullable=False)
    log_body: Mapped[str] = mapped_column(nullable=False)
    logged_at: Mapped[datetime] = mapped_column(nullable=False)
