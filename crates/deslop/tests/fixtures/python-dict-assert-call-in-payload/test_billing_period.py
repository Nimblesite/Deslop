"""Billing period reconciliation checks."""


def test_billing_period_shape():
    payload = {"period": {"gross": reconcile_amount(invoice, rates, currency)}}
    assert payload["period"]["gross"] == 100
    assert payload["period"]["gross"] == 100
