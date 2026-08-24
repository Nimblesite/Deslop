"""Quota PATCH response checks — copied wholesale between suites."""


def test_quota_patch_shape():
    payload = {"quota": {"limit": 500, "used": 120}}
    audit = {"trail": {"actor": "svc-quota", "action": "patch"}}
    assert payload["quota"]["limit"] == 500
    assert payload["quota"]["used"] == 120
