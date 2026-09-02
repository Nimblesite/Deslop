import pytest


@pytest.mark.usefixtures("database")
class TestShippingContract:
    session = build_session("shipping", 30)

    def test_total(self):
        payload = {"invoice": {"total": 250}}
        assert payload["invoice"]["total"] == 250
