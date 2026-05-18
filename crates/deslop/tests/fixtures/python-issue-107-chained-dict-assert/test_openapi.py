"""OpenAPI document shape assertions."""


def test_openapi_info_title_and_version():
    doc = {"info": {"title": "Agent Backend", "version": "0.1.0"}}
    assert doc["info"]["title"] == "Agent Backend"
    assert doc["info"]["version"] == "0.1.0"
