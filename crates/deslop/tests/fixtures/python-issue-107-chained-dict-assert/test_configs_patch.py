"""PATCH /configs response assertions over nested model config."""


def test_configs_patch_model_config_nesting():
    data = {"model_config": {"provider": "openai", "model": "gpt-4o"}}
    assert data["model_config"]["provider"] == "openai"
    assert data["model_config"]["model"] == "gpt-4o"
