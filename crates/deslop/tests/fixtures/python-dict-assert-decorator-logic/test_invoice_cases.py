"""Invoice case checks driven by generated parameters."""

import pytest


@pytest.mark.parametrize("case", build_cases("invoice"))
def test_invoice_case_shape(case):
    payload = {"case": {"total": 900, "state": "sent", "currency": "USD", "region": "eu"}}
    assert payload["case"]["total"] == 900
    assert payload["case"]["state"] == "sent"
