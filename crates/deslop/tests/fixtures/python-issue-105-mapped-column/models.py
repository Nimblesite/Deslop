"""SQLAlchemy ORM model declarations.

Three model classes, each declaring a different set of mapped columns
with distinct primary-key attribute names so byte-identical lines do
not falsely look like extractable duplication. Token Jaccard still hits
1.00 because every column shares the alphabet
{Mapped, mapped_column, ForeignKey, UUID, datetime}. Structural
similarity is near zero — each block names different attributes on
different tables — so the cluster filter must drop these clusters.
"""

from datetime import datetime
from uuid import UUID, uuid4

from sqlalchemy import ForeignKey
from sqlalchemy.orm import Mapped, mapped_column


class Tenant:
    tenant_id: Mapped[UUID] = mapped_column(primary_key=True, default=uuid4)
    display_name: Mapped[str] = mapped_column(nullable=False)
    activated_at: Mapped[datetime] = mapped_column(nullable=False)
    refreshed_at: Mapped[datetime] = mapped_column(nullable=True)


class Conversation:
    conversation_id: Mapped[UUID] = mapped_column(primary_key=True, default=uuid4)
    owner_tenant: Mapped[UUID] = mapped_column(ForeignKey("tenant.id"))
    subject_line: Mapped[str] = mapped_column(nullable=False)
    is_archived: Mapped[bool] = mapped_column(default=False)


class Message:
    message_id: Mapped[UUID] = mapped_column(primary_key=True, default=uuid4)
    parent_conversation: Mapped[UUID] = mapped_column(ForeignKey("conversation.id"))
    sender_role: Mapped[str] = mapped_column(nullable=False)
    body_text: Mapped[str] = mapped_column(nullable=False)
