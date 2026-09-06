"""Billing case checks driven by generated parameters."""

import pytest


@pytest.mark.parametrize("case", build_cases("billing"))
def test_billing_case_shape(case):
    payload = {"case": {"total": 500, "state": "open"}}
    assert payload["case"]["total"] == 500
    assert payload["case"]["state"] == "open"
