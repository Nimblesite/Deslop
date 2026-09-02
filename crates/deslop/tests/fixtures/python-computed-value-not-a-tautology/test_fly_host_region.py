def test_fly_host(monkeypatch, host_prefix):
    monkeypatch.setenv("FLY_API_TOKEN", "xyz")
    monkeypatch.setenv("FLY_APP_NAME", "iad")
    explicit_host_id = host_prefix + "2"
    assert explicit_host_id == "fly-2"
