def _tool_call_response(name, arguments, call_id):
    return {"name": name, "arguments": arguments, "id": call_id}


def test_homepage_write():
    response = _tool_call_response("write_file", {"path": "index.html", "content": "<h1>Hi</h1>"}, "call-1")
    assert response["id"] == "call-1"


def test_about_write():
    response = _tool_call_response("write_file", {"path": "about.md", "content": "About"}, "call-2")
    assert response["id"] == "call-2"


def test_hero_write():
    response = _tool_call_response("write_file", {"path": "hero.md", "content": "# New"}, "call-xyz")
    assert response["id"] == "call-xyz"


def test_footer_write():
    response = _tool_call_response("write_file", {"path": "footer.html", "content": "<footer/>"}, "call-foo")
    assert response["id"] == "call-foo"
