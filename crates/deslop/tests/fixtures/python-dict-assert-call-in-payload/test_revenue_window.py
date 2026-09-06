"""Revenue window reconciliation checks."""


def test_revenue_window_shape():
    summary = {"window": {"net": reconcile_amount(ledger, tariffs, region)}}
    assert summary["window"]["net"] == 250
    assert summary["window"]["net"] == 250
