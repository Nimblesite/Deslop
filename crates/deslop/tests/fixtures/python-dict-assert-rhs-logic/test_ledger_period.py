"""Ledger period reconciliation — the expected value is COMPUTED."""


def test_ledger_period_reconciles_gross_and_net():
    ledger = {"period": {"gross": 4400, "net": 4000}}
    assert ledger["period"]["gross"] == reconcile_amount(
        [1000, 1000, 1000, 1000], 0.10, 2, "AUD", True, "gross"
    )
    assert ledger["period"]["net"] == reconcile_amount(
        [1000, 1000, 1000, 1000], 0.10, 2, "AUD", True, "net"
    )
