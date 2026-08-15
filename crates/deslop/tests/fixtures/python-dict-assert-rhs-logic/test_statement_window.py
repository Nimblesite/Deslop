"""Statement window reconciliation — the expected value is COMPUTED."""


def test_statement_window_reconciles_gross_and_net():
    statement = {"window": {"gross": 4400, "net": 4000}}
    assert statement["window"]["gross"] == reconcile_amount(
        [1000, 1000, 1000, 1000], 0.10, 2, "AUD", True, "gross"
    )
    assert statement["window"]["net"] == reconcile_amount(
        [1000, 1000, 1000, 1000], 0.10, 2, "AUD", True, "net"
    )
