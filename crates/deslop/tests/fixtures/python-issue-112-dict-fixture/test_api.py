"""Public API tool-results request body shape."""


def test_api_tool_results_request_body():
    body = {
        "tool_call_id": "call-7",
        "name": "search",
        "content": {"hit": "first"},
    }
    assert body["tool_call_id"] == "call-7"
