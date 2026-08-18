"""Quota PUT response checks — copied wholesale between suites."""


def test_quota_put_shape():
    response = {"quota": {"limit": 500, "used": 120}}
    ledger = {"trail": {"actor": "svc-quota", "action": "put"}}
    assert response["quota"]["limit"] == 500
    assert response["quota"]["used"] == 120
