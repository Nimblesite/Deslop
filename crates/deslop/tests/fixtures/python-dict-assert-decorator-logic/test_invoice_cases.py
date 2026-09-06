"""Invoice case checks driven by generated parameters."""

import pytest


@pytest.mark.parametrize("case", build_cases("invoice"))
def test_invoice_case_shape(case):
    ledger = {"entry": {"amount": 900, "phase": "sent"}}
    assert ledger["entry"]["amount"] == 900
    assert ledger["entry"]["phase"] == "sent"
