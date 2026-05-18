"""Cross-file AgentLog row instance built via kwargs constructor.

Identical AST shape to Message in `conftest.py` (4 kwargs, datetime.now
on the last field) but distinct keyword names (`level`/`message` vs
`role`/`content`). Without the filter this clusters as a duplicate.
"""

from datetime import UTC, datetime


class AgentLog:
    pass


CONV_TWO = "conv-2"


agent_log_one = AgentLog(
    conversation_id=CONV_TWO,
    level="info",
    message="agent started",
    created_at=datetime.now(tz=UTC),
)
