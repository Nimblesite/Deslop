PY_HEADER = '''"""Generated from contracts/chat-protocol.td. DO NOT HAND-EDIT.

Regenerate with make regen-contracts. Source pipeline is documented in
scripts/gen_contracts.py and docs/specs/shared-dtos.md.
Implements shared wire contracts for the API layer.
"""'''


def render_contracts() -> str:
    return PY_HEADER + "\n\nSCHEMA_VERSION = \"v1\"\n"
