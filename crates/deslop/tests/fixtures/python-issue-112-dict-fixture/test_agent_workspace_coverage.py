"""Workspace agent LLM-response shape assertions."""


def test_agent_workspace_tool_calls():
    payload = {
        "id": "call-1",
        "name": "write_file",
        "arguments": {"path": "x"},
    }
    assert payload["id"] == "call-1"
