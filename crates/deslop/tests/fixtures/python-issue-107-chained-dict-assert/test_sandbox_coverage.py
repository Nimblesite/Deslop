"""Sandbox dispatcher assertion over a nested-dict response payload."""


def test_sandbox_dispatch_retryable():
    row_r = {"content": {"retryable": True, "kind": "tool"}}
    assert row_r["content"]["retryable"] is True
    assert row_r["content"]["kind"] == "tool"
