import base64

import pytest


def encode_payload(raw):
    encoded = base64.b64encode(raw)
    return encoded


def decode_payload(blob):
    decoded = base64.b64decode(blob)
    return decoded


@pytest.fixture
def ollama_client():
    return {"endpoint": "http://localhost:11434"}


@pytest.fixture
def sample_image():
    return b"\x89PNG"
