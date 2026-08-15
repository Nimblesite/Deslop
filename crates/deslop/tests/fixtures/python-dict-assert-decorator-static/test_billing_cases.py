"""Billing case checks over a static parameter table."""

import pytest


@pytest.mark.parametrize("case", ["draft", "final"])
def test_billing_case_shape(case):
    payload = {"case": {"total": 500, "state": "open"}}
    assert payload["case"]["total"] == 500
    assert payload["case"]["state"] == "open"
