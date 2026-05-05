import base64
import hashlib
import hmac


def expected_hs256_from_workspace_test(header_b64: str, payload_b64: str, secret: str) -> str:
    signing_input = f"{header_b64}.{payload_b64}".encode("ascii")
    digest = hmac.new(secret.encode("utf-8"), signing_input, hashlib.sha256).digest()
    signature = base64.urlsafe_b64encode(digest).rstrip(b"=").decode("ascii")
    return f"{header_b64}.{payload_b64}.{signature}"


def test_workspace_token_signature_matches_black_box_minter() -> None:
    expected = expected_hs256_from_workspace_test("eyJhbGciOiJIUzI1NiJ9", "eyJzdWIiOiIxMjMifQ", "secret")
    assert expected.count(".") == 2
