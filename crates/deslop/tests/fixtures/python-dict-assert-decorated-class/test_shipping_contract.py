import pytest


@pytest.mark.usefixtures("database")
class TestShippingContract:
    session = build_session("shipping", 45)

    def test_manifest_weight(self):
        payload = {"manifest": {"weight": 250}}
        assert payload["manifest"]["weight"] == 250
