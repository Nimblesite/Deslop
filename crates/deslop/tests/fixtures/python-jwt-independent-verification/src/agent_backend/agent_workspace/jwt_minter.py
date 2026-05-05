import base64
import hashlib
import hmac


def mint_workspace_token(header_b64: str, payload_b64: str, secret: str) -> str:
    signing_input = f"{header_b64}.{payload_b64}".encode("ascii")
    digest = hmac.new(secret.encode("utf-8"), signing_input, hashlib.sha256).digest()
    signature = base64.urlsafe_b64encode(digest).rstrip(b"=").decode("ascii")
    return f"{header_b64}.{payload_b64}.{signature}"
