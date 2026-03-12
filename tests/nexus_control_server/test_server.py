import http.client
import json
import os
import sys
import threading
import time
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest import TestCase, mock

ROOT = Path(__file__).resolve().parents[2]
SERVER_DIR = ROOT / "src" / "nexus_control_server"
sys.path.insert(0, str(SERVER_DIR))

import server  # noqa: E402


def _json_body(payload):
    raw = json.dumps(payload).encode("utf-8")
    return raw, {
        "Content-Type": "application/json",
        "Content-Length": str(len(raw)),
    }


class ControlServerTests(TestCase):
    def setUp(self):
        server.SESSIONS.clear()
        server.RATE_LIMITS.clear()
        server.SESSION_BIND_CONTEXT = True

        self.tmp = TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        tmp_path = Path(self.tmp.name)

        server.LOG_DIR = tmp_path / "logs"
        server.LOG_DIR.mkdir(parents=True, exist_ok=True)

        self.config_path = tmp_path / "services.json"
        self.state_path = tmp_path / "state.json"
        config = {
            "services": {
                "dummy": {
                    "command": ["{python}", "-c", "import time; time.sleep(60)"],
                    "cwd": ".",
                    "interactive": False,
                }
            }
        }
        self.config_path.write_text(json.dumps(config), encoding="utf-8")
        self.state_path.write_text(json.dumps({"services": {}}, indent=2), encoding="utf-8")

        self.manager = server.ServiceManager(tmp_path, self.config_path, self.state_path)
        handler = server.make_handler(self.manager)
        self.httpd = server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
        self.port = self.httpd.server_address[1]
        self.thread = threading.Thread(target=self.httpd.serve_forever, daemon=True)
        self.thread.start()
        time.sleep(0.05)

    def tearDown(self):
        try:
            self.manager.stop_service("dummy")
        except Exception:
            pass
        self.httpd.shutdown()
        self.httpd.server_close()

    def _request(self, method, path, body=None, headers=None):
        conn = http.client.HTTPConnection("127.0.0.1", self.port)
        conn.request(method, path, body=body, headers=headers or {})
        resp = conn.getresponse()
        data = resp.read()
        conn.close()
        return resp, data

    def _auth_headers(self, token):
        return {
            "Authorization": f"Bearer {token}",
            "User-Agent": "test-agent",
        }

    def test_protected_route_requires_auth(self):
        # Scenario: access protected API without a session.
        # Expectation: 401 response with WWW-Authenticate header.
        with mock.patch.dict(os.environ, {"NEXUS_GOOGLE_CLIENT_ID": "test"}, clear=False):
            resp, _ = self._request("GET", "/api/services", headers={"User-Agent": "test-agent"})

        self.assertEqual(resp.status, 401)
        self.assertIn("WWW-Authenticate", resp.headers)

    def test_oauth_flow_is_mocked(self):
        # Scenario: Google OAuth token is verified via mock.
        # Expectation: session is created and allows access to protected endpoints.
        with mock.patch.dict(os.environ, {"NEXUS_GOOGLE_CLIENT_ID": "test"}, clear=False):
            with mock.patch.object(server, "verify_google_id_token", return_value="user@example.com"):
                body, headers = _json_body({"id_token": "good"})
                headers["User-Agent"] = "test-agent"
                resp, data = self._request("POST", "/api/auth/google", body=body, headers=headers)

        self.assertEqual(resp.status, 200)
        payload = json.loads(data.decode("utf-8"))
        token = payload.get("token")
        self.assertTrue(token)

        resp, _ = self._request("GET", "/api/services", headers=self._auth_headers(token))
        self.assertEqual(resp.status, 200)

    def test_rate_limiting_after_threshold(self):
        # Scenario: exceed auth rate limit within the window.
        # Expectation: endpoint returns 429 with Retry-After.
        with mock.patch.dict(os.environ, {"NEXUS_GOOGLE_CLIENT_ID": "test"}, clear=False):
            with mock.patch.object(server, "verify_google_id_token", return_value="user@example.com"):
                body, headers = _json_body({"id_token": "good"})
                headers["User-Agent"] = "test-agent"
                for _ in range(server.AUTH_RATE_LIMIT_MAX):
                    resp, _ = self._request("POST", "/api/auth/google", body=body, headers=headers)
                    self.assertIn(resp.status, {200, 403, 401})

                resp, _ = self._request("POST", "/api/auth/google", body=body, headers=headers)

        self.assertEqual(resp.status, 429)
        self.assertIn("Retry-After", resp.headers)

    def test_security_headers_present(self):
        # Scenario: fetch health endpoint in production mode.
        # Expectation: security headers (CSP, COOP, XFO, HSTS) are present.
        with mock.patch.dict(os.environ, {"NEXUS_ENV": "production"}, clear=False):
            resp, _ = self._request("GET", "/api/health")

        self.assertIn("Content-Security-Policy", resp.headers)
        self.assertIn("Cross-Origin-Opener-Policy", resp.headers)
        self.assertIn("X-Frame-Options", resp.headers)
        self.assertIn("Strict-Transport-Security", resp.headers)

    def test_session_hardening_and_expiry(self):
        # Scenario: validate session binding, invalidation, and expiry.
        # Expectation: invalid tokens fail, context mismatch fails, expired sessions are rejected.
        context = {"ip": "127.0.0.1", "ua_hash": "abc"}
        session = server.create_session("user@example.com", context)
        token = session["token"]

        self.assertFalse(server.validate_session("invalid-token", context))
        self.assertTrue(server.validate_session(token, context))
        self.assertFalse(server.validate_session(token, {"ip": "10.0.0.1", "ua_hash": "abc"}))

        server.SESSIONS[token]["exp"] = int(time.time()) - 10
        self.assertFalse(server.validate_session(token, context))

        server.revoke_session(token)
        self.assertFalse(server.validate_session(token, context))

    def test_start_stop_idempotent(self):
        # Scenario: start/stop service multiple times.
        # Expectation: start reports already_running and stop is idempotent.
        with mock.patch.dict(os.environ, {"NEXUS_GOOGLE_CLIENT_ID": "test"}, clear=False):
            with mock.patch.object(server, "verify_google_id_token", return_value="user@example.com"):
                body, headers = _json_body({"id_token": "good"})
                headers["User-Agent"] = "test-agent"
                resp, data = self._request("POST", "/api/auth/google", body=body, headers=headers)
                token = json.loads(data.decode("utf-8"))["token"]

        resp, data = self._request(
            "POST",
            "/api/services/dummy/start",
            headers=self._auth_headers(token),
        )
        payload = json.loads(data.decode("utf-8"))
        self.assertFalse(payload.get("already_running"))

        resp, data = self._request(
            "POST",
            "/api/services/dummy/start",
            headers=self._auth_headers(token),
        )
        payload = json.loads(data.decode("utf-8"))
        self.assertTrue(payload.get("already_running"))

        resp, data = self._request(
            "POST",
            "/api/services/dummy/stop",
            headers=self._auth_headers(token),
        )
        payload = json.loads(data.decode("utf-8"))
        self.assertTrue(payload.get("stopped"))

        resp, data = self._request(
            "POST",
            "/api/services/dummy/stop",
            headers=self._auth_headers(token),
        )
        payload = json.loads(data.decode("utf-8"))
        self.assertFalse(payload.get("stopped"))

