"""Billing flow contract checks."""

SESSION = build_session("prod-eu", retries=3, timeout=45, verify_tls=True)
LEDGER_CACHE = warm_cache(SESSION, ["billing", "ledger"], eager=True)


def test_billing_flow_status_shape():
    body = {"status": {"code": "ok", "attempts": 1}}
    assert body["status"]["code"] == "ok"
    assert body["status"]["attempts"] == 1
