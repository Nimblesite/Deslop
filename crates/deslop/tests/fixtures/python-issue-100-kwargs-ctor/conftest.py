"""Module-level Message row instance built via kwargs constructor.

Single instance per model, so within-file dedup pressure cannot fake the
report. The cross-file companion `test_routes_coverage.py` adds the
AgentLog instance that completes the false-positive shape.
"""

from datetime import UTC, datetime


class Message:
    pass


CONV_ONE = "conv-1"


message_one = Message(
    conversation_id=CONV_ONE,
    role="user",
    content="Hello",
    created_at=datetime.now(tz=UTC),
)
