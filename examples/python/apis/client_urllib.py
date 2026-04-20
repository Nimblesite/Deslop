"""HTTP client built on the stdlib ``urllib``.

Twin of ``client_requests.py``. Same surface, stdlib-only
implementation — far more plumbing, identical semantics.
"""

from __future__ import annotations

import json
import urllib.parse
import urllib.request


class Response:
    def __init__(self, status, body):
        self.status = status
        self.body = body


class UserClient:
    def __init__(self, base_url):
        self.base_url = base_url.rstrip("/")

    def get_user(self, user_id):
        url = f"{self.base_url}/users/{user_id}"
        request = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(request) as handle:
            payload = json.loads(handle.read().decode("utf-8"))
            return Response(handle.status, payload)

    def list_users(self, limit):
        query = urllib.parse.urlencode({"limit": limit})
        url = f"{self.base_url}/users?{query}"
        request = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(request) as handle:
            payload = json.loads(handle.read().decode("utf-8"))
            return Response(handle.status, payload)

    def create_user(self, payload):
        url = f"{self.base_url}/users"
        body = json.dumps(payload).encode("utf-8")
        request = urllib.request.Request(
            url, data=body, method="POST", headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(request) as handle:
            response_body = json.loads(handle.read().decode("utf-8"))
            return Response(handle.status, response_body)

    def delete_user(self, user_id):
        url = f"{self.base_url}/users/{user_id}"
        request = urllib.request.Request(url, method="DELETE")
        with urllib.request.urlopen(request) as handle:
            return Response(handle.status, None)
