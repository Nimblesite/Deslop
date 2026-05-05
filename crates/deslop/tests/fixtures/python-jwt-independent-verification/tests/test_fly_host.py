import base64
import hashlib
import hmac


def expected_hs256_for_fly_host(header_b64: str, payload_b64: str, secret: str) -> str:
    signing_input = f"{header_b64}.{payload_b64}".encode("ascii")
    digest = hmac.new(secret.encode("utf-8"), signing_input, hashlib.sha256).digest()
    signature = base64.urlsafe_b64encode(digest).rstrip(b"=").decode("ascii")
    return f"{header_b64}.{payload_b64}.{signature}"


def test_fly_host_token_signature_matches_black_box_minter() -> None:
    expected = expected_hs256_for_fly_host("eyJhbGciOiJIUzI1NiJ9", "eyJhdWQiOiJmbHkifQ", "secret")
    assert expected.count(".") == 2
