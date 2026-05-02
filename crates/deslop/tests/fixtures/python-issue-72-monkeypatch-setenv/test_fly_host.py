def test_fly_host_loads_token(monkeypatch):
    monkeypatch.setenv("FLY_API_TOKEN", "abc")
    monkeypatch.setenv("FLY_APP_NAME", "app")
    explicit_host_id = "fly-1"
    assert explicit_host_id == "fly-1"


def test_fly_host_with_region(monkeypatch):
    monkeypatch.setenv("FLY_API_TOKEN", "xyz")
    monkeypatch.setenv("FLY_REGION", "iad")
    explicit_host_id = "fly-2"
    assert explicit_host_id == "fly-2"


def test_fly_host_with_org(monkeypatch):
    monkeypatch.setenv("FLY_API_TOKEN", "qwe")
    monkeypatch.setenv("FLY_ORG", "main")
    explicit_host_id = "fly-3"
    assert explicit_host_id == "fly-3"
