"""Deterministic tests for the read-only Board Manager probe."""
import importlib.util
import json
import pathlib

import threading
import time
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from unittest import mock

from websockets.sync.server import serve

SCRIPT = pathlib.Path(__file__).with_name("probe-board.py")
spec = importlib.util.spec_from_file_location("probe_board", SCRIPT)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load probe module")
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class ProbeTests(unittest.TestCase):
    def test_state_allowlist_drops_private_and_unrecognized_fields(self):
        state = probe.sanitized_state({
            "connected": True, "running": False, "status": "Stopped", "event": "Stopped",
            "numThrows": 0, "players": ["private"], "coordinates": [1, 2], "accountId": "secret",
        })
        self.assertEqual(set(state), {"connected", "running", "status", "event", "numThrows"})
        self.assertEqual(state["numThrows"], 0)

    def test_invalid_schema_yields_safe_error(self):
        self.assertEqual(probe.fetch("http://example.invalid", "/api/state", 1), (None, "request_failed"))
        with mock.patch.object(probe.OPENER, "open") as open_url:
            response = mock.MagicMock()
            response.__enter__.return_value.read1.side_effect = [b'{"accountId":"secret","running":false}', b""]
            open_url.return_value = response
            result, error = probe.fetch("http://127.0.0.1", "/api/state", 1)
        self.assertIsNone(error)
        self.assertEqual(result["running"], False)
        self.assertNotIn("accountId", json.dumps(result))

    def test_size_limit_and_endpoint_allowlist(self):
        with mock.patch.object(probe.OPENER, "open") as open_url:
            response = mock.MagicMock()
            response.__enter__.return_value.read1.side_effect = [b"x" * (probe.MAX_BODY + 1)]
            open_url.return_value = response
            self.assertEqual(probe.fetch("http://127.0.0.1", "/api/version", 1), (None, "response_too_large"))
        with self.assertRaises(ValueError):
            probe.fetch("http://127.0.0.1", "/api/config", 1)

    def test_redirect_is_rejected(self):
        with mock.patch.object(probe.OPENER, "open", side_effect=probe.urllib.error.HTTPError(
                "http://127.0.0.1/api/state", 302, "redirect", {"Location": "http://elsewhere/"}, None)):
            self.assertEqual(probe.fetch("http://127.0.0.1", "/api/state", 1), (None, "redirect_rejected"))

    def test_total_body_deadline_defeats_slow_trickle(self):
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                self.send_response(200)
                self.send_header("Content-Length", "100")
                self.end_headers()
                try:
                    for _ in range(100):
                        self.wfile.write(b"x")
                        self.wfile.flush()
                        time.sleep(0.08)
                except (BrokenPipeError, ConnectionResetError):
                    pass
            def log_message(self, *_args):
                pass

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        server.daemon_threads = True
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        started = time.monotonic()
        try:
            value = probe.fetch(f"http://127.0.0.1:{server.server_port}", "/api/state", 0.3)
            self.assertLess(time.monotonic() - started, 0.8)
            self.assertEqual(value, (None, "request_failed"))
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

    def test_invalid_ports_fail_address_validation(self):
        for address in ("http://127.0.0.1:99999", "http://127.0.0.1:notaport"):
            with mock.patch("sys.argv", ["probe-board.py", address]):
                self.assertEqual(probe.main(), 2)

    def test_loopback_websocket_cases_and_no_application_writes(self):
        def run_case(kind, seconds=0.15):
            attempted_writes = []
            def handler(connection):
                try:
                    if kind == "success":
                        connection.send(json.dumps({"type": "state", "data": {"running": True, "private": "secret"}}))
                        connection.send(json.dumps({"type": "household-account-identifier", "data": {"token": "secret"}}))
                        try:
                            attempted_writes.append(connection.recv(timeout=0.35))
                        except TimeoutError:
                            pass
                    elif kind == "idle":
                        try:
                            attempted_writes.append(connection.recv(timeout=0.35))
                        except TimeoutError:
                            pass
                    elif kind == "oversize":
                        connection.send("x" * (probe.MAX_BODY + 1))
                    elif kind == "close":
                        connection.close()
                except Exception:
                    pass

            def reject(_connection, request):
                from websockets.http11 import Response
                from websockets.datastructures import Headers
                return Response(403, "Forbidden", Headers(), b"")

            options = {"process_request": reject} if kind == "reject" else {}
            with serve(handler, "127.0.0.1", 0, **options) as server:
                port = server.socket.getsockname()[1]
                server_thread = threading.Thread(target=server.serve_forever, daemon=True)
                server_thread.start()
                try:
                    result = probe.observe_events(f"http://127.0.0.1:{port}", seconds, 1, 2)
                finally:
                    server.shutdown()
                    server_thread.join()
            return result, attempted_writes

        result, writes = run_case("success")
        self.assertTrue(result["available"], result)
        self.assertEqual(result["messages"], 2)
        self.assertEqual(result["topics"], ["other", "state"])
        self.assertEqual(writes, [])
        self.assertNotIn("private", json.dumps(result))
        for kind, reason in (("idle", None), ("close", "connection_closed"),
                             ("oversize", "message_too_large"), ("reject", "handshake_failed")):
            result, writes = run_case(kind)
            if kind == "idle":
                self.assertTrue(result["available"])
                self.assertEqual(result["messages"], 0)
            else:
                self.assertFalse(result["available"])
                self.assertEqual(result["reason"], reason)
            self.assertEqual(writes, [])

    def test_safe_state_field_bounds_and_types(self):
        state = probe.sanitized_state({"connected": 1, "running": False, "status": "x" * 50, "numThrows": True})
        self.assertIsNone(state["connected"])
        self.assertIsNone(state["status"])
        self.assertIsNone(state["numThrows"])
        with self.assertRaises(ValueError):
            probe.sanitized_state([])


if __name__ == "__main__":
    unittest.main()
