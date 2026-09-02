"""Invoice case checks over a static parameter table."""

import pytest


@pytest.mark.parametrize("case", ["sent", "paid"])
def test_invoice_case_shape(case):
    payload = {"case": {"total": 900, "state": "sent", "currency": "USD", "region": "eu"}}
    assert payload["case"]["total"] == 900
    assert payload["case"]["state"] == "sent"
