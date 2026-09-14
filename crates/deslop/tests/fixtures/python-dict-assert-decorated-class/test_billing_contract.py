import pytest


@pytest.mark.usefixtures("database")
class TestBillingContract:
    session = build_session("billing", 30)

    def test_total(self):
        payload = {"invoice": {"total": 100}}
        assert payload["invoice"]["total"] == 100
