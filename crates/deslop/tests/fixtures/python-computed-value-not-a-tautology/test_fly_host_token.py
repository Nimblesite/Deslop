def test_fly_host(monkeypatch, host_prefix):
    monkeypatch.setenv("FLY_API_TOKEN", "abc")
    monkeypatch.setenv("FLY_APP_NAME", "app")
    explicit_host_id = host_prefix + "1"
    assert explicit_host_id == "fly-1"
