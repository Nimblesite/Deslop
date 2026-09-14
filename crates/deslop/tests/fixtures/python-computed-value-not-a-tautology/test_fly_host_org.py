def test_fly_host(monkeypatch, host_prefix):
    monkeypatch.setenv("FLY_API_TOKEN", "qwe")
    monkeypatch.setenv("FLY_APP_NAME", "main")
    explicit_host_id = host_prefix + "3"
    assert explicit_host_id == "fly-3"
