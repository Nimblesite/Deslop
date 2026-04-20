"""HTTP client built on ``httpx`` (pretend).

Twin of ``client_requests.py`` / ``client_urllib.py``. Same surface,
async-compatible library — we keep the sync methods here so the
clone relationship is the three client classes themselves, not the
async/sync split.
"""

from __future__ import annotations


class Response:
    def __init__(self, status, body):
        self.status = status
        self.body = body


class UserClient:
    def __init__(self, base_url, client):
        self.base_url = base_url.rstrip("/")
        self.client = client

    def get_user(self, user_id):
        response = self.client.get(f"{self.base_url}/users/{user_id}")
        return Response(response.status_code, response.json())

    def list_users(self, limit):
        response = self.client.get(
            f"{self.base_url}/users", params={"limit": limit}
        )
        return Response(response.status_code, response.json())

    def create_user(self, payload):
        response = self.client.post(f"{self.base_url}/users", json=payload)
        return Response(response.status_code, response.json())

    def delete_user(self, user_id):
        response = self.client.delete(f"{self.base_url}/users/{user_id}")
        return Response(response.status_code, None)
