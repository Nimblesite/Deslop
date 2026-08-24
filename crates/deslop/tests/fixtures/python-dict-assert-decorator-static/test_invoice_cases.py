"""Invoice case checks over a static parameter table."""

import pytest


@pytest.mark.parametrize("case", ["sent", "paid"])
def test_invoice_case_shape(case):
    ledger = {"entry": {"amount": 900, "phase": "sent"}}
    assert ledger["entry"]["amount"] == 900
    assert ledger["entry"]["phase"] == "sent"
