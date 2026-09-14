"""Every test here starts a turn through the already-extracted helper
`_post_turn`. The extraction has already happened: the call sites are
the deduplicated form, and wrapping the wrapper is over-abstraction."""


async def test_turn_returns_an_assistant_message(client, test_api_key, config_id):
    body = await _post_turn(
        client,
        test_api_key,
        config_id=config_id,
        message="what is the weather",
        conversation_id=None,
    )
    assert body["role"] == "assistant"


async def test_turn_continues_an_existing_conversation(client, test_api_key, config_id):
    body = await _post_turn(
        client,
        test_api_key,
        config_id=config_id,
        message="and tomorrow",
        conversation_id="conv-77",
    )
    assert body["conversation_id"] == "conv-77"


async def test_turn_rejects_an_empty_message(client, test_api_key, config_id):
    body = await _post_turn(
        client,
        test_api_key,
        config_id=config_id,
        message="",
        conversation_id=None,
    )
    assert body["error"] == "empty_message"


async def test_turn_records_the_tool_call(client, test_api_key, config_id):
    body = await _post_turn(
        client,
        test_api_key,
        config_id=config_id,
        message="run the report",
        conversation_id="conv-91",
    )
    assert body["tool_calls"][0]["name"] == "report"
