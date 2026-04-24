"""HTTP client built on ``requests`` (pretend).

Paired with ``client_urllib.py`` (stdlib urllib) and
``client_httpx.py`` (modern ``httpx``) — three implementations of the
same client surface. Every call site behaves identically; the AST
differs dramatically. Same behavior, different code [Type-4] family.
"""

from __future__ import annotations


class Response:
    def __init__(self, status, body):
        self.status = status
        self.body = body


class UserClient:
    def __init__(self, base_url, session):
        self.base_url = base_url.rstrip("/")
        self.session = session

    def get_user(self, user_id):
        url = f"{self.base_url}/users/{user_id}"
        response = self.session.get(url)
        return Response(response.status_code, response.json())

    def list_users(self, limit):
        url = f"{self.base_url}/users"
        response = self.session.get(url, params={"limit": limit})
        return Response(response.status_code, response.json())

    def create_user(self, payload):
        url = f"{self.base_url}/users"
        response = self.session.post(url, json=payload)
        return Response(response.status_code, response.json())

    def delete_user(self, user_id):
        url = f"{self.base_url}/users/{user_id}"
        response = self.session.delete(url)
        return Response(response.status_code, None)
