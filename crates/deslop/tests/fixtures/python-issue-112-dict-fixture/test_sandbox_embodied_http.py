"""Tool-schema fixtures used by the embodied HTTP sandbox tests."""


def test_tool_schema_payload():
    schema = {
        "name": "write_file",
        "description": "Writes a file",
        "parameters_schema": {"type": "object"},
    }
    assert schema["name"] == "write_file"


def test_tool_config_payload():
    schema = {
        "name": "read_file",
        "description": "Reads a file",
        "parameters_schema": {"type": "object"},
    }
    assert schema["name"] == "read_file"
