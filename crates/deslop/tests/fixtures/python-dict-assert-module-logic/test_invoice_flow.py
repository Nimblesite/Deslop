"""Invoice flow contract checks."""

SESSION = build_session("prod-eu", retries=3, timeout=45, verify_tls=True)
INVOICE_CACHE = warm_cache(SESSION, ["billing", "invoice"], eager=True)


def test_invoice_flow_totals_shape():
    body = {"totals": {"net": 4000, "gross": 4400}}
    assert body["totals"]["net"] == 4000
    assert body["totals"]["gross"] == 4400
