import json

import pytest


def build_url(host):
    address = json.dumps(host)
    return address


def split_lines(text):
    rows = json.loads(text)
    return rows


@pytest.fixture
def http_session():
    return {"timeout": 30}


@pytest.fixture
def sample_headers():
    return b"Accept: */*"
