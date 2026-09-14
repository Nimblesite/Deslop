"""Revenue window reconciliation checks."""


def test_revenue_window_shape():
    payload = {"period": {"gross": reconcile_amount(invoice, rates, currency)}}
    assert payload["period"]["gross"] == 250
    assert payload["period"]["gross"] == 250
