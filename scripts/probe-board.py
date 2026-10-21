#!/usr/bin/env python3
# /// script
# dependencies = ["websockets==15.0.1"]
# ///
"""Bounded read-only Board Manager diagnostic probe."""
import argparse
import json
import socket
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

MAX_BODY = 64 * 1024
ALLOWED_PATHS = ("/api/version", "/api/state")


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


OPENER = urllib.request.build_opener(NoRedirect)


def read_bounded(response, deadline):
    """Enforce an absolute response deadline as well as a per-read socket timeout."""
    body = bytearray()
    while len(body) <= MAX_BODY:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError
        raw = getattr(getattr(response, "fp", None), "raw", None)
        sock = getattr(raw, "_sock", None)
        if sock is not None:
            sock.settimeout(remaining)
        read_chunk = getattr(response, "read1", response.read)
        chunk = read_chunk(min(8192, MAX_BODY + 1 - len(body)))
        if not chunk:
            break
        body.extend(chunk)
    return bytes(body)


def sanitized_version(value):
    return value if isinstance(value, str) and len(value) <= 80 and value.isprintable() else None


def sanitized_state(value):
    if not isinstance(value, dict):
        raise ValueError("invalid state shape")
    status, event, throws = value.get("status"), value.get("event"), value.get("numThrows")
    return {
        "connected": value.get("connected") if isinstance(value.get("connected"), bool) else None,
        "running": value.get("running") if isinstance(value.get("running"), bool) else None,
        "status": status if isinstance(status, str) and len(status) <= 40 and status.isprintable() else None,
        "event": event if isinstance(event, str) and len(event) <= 40 and event.isprintable() else None,
        "numThrows": throws if isinstance(throws, int) and not isinstance(throws, bool) and 0 <= throws <= 10_000_000 else None,
    }


def fetch(base, path, timeout):
    if path not in ALLOWED_PATHS:
        raise ValueError("endpoint not allowlisted")
    request = urllib.request.Request(base + path, method="GET", headers={"Accept": "application/json, text/plain"})
    deadline = time.monotonic() + timeout
    try:
        with OPENER.open(request, timeout=timeout) as response:
            body = read_bounded(response, deadline)
    except urllib.error.HTTPError as exc:
        if 300 <= exc.code < 400:
            return None, "redirect_rejected"
        return None, "http_error"
    except (urllib.error.URLError, TimeoutError, socket.timeout, OSError):
        return None, "request_failed"
    if len(body) > MAX_BODY:
        return None, "response_too_large"
    try:
        decoded = body.decode("utf-8")
        parsed = json.loads(decoded) if path == "/api/state" else decoded
        return (sanitized_state(parsed) if path == "/api/state" else sanitized_version(parsed)), None
    except (UnicodeError, json.JSONDecodeError, ValueError):
        return None, "invalid_response"


def observe_events(base, seconds, timeout, message_limit):
    """Observe using the RFC-compliant websockets client; never send app commands."""
    from websockets.sync.client import connect
    from websockets.exceptions import ConnectionClosed, InvalidHandshake

    uri = base.replace("https://", "wss://", 1).replace("http://", "ws://", 1) + "/api/events"
    count, topics = 0, set()
    deadline = time.monotonic() + seconds
    try:
        with connect(uri, open_timeout=min(timeout, seconds), close_timeout=0.2,
                     max_size=MAX_BODY, proxy=None) as connection:
            while count < message_limit:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    break
                try:
                    payload = connection.recv(timeout=remaining)
                except TimeoutError:
                    break
                count += 1
                try:
                    message = json.loads(payload)
                except (UnicodeError, json.JSONDecodeError):
                    continue
                topic = message.get("type") if isinstance(message, dict) else None
                if isinstance(topic, str):
                    topics.add("state" if topic == "state" else "other")
        return {"available": True, "reason": None, "messages": count, "topics": sorted(topics)}
    except InvalidHandshake:
        reason = "handshake_failed"
    except ConnectionClosed as exc:
        close = exc.rcvd if exc.rcvd is not None else exc.sent
        reason = "message_too_large" if close is not None and close.code == 1009 else "connection_closed"
    except (OSError, TimeoutError, ValueError):
        reason = "connection_failed"
    return {"available": False, "reason": reason, "messages": count, "topics": sorted(topics)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("address", help="Board Manager origin/address, e.g. http://autodartspi.local:3180")
    parser.add_argument("--timeout", type=float, default=2.0)
    parser.add_argument("--observe-seconds", type=float, default=0)
    parser.add_argument("--max-messages", type=int, default=20)
    args = parser.parse_args()
    try:
        parsed = urllib.parse.urlparse(args.address)
        port = parsed.port
        if parsed.scheme not in ("http", "https") or not parsed.hostname or parsed.username or parsed.password or parsed.path not in ("", "/") or parsed.query or parsed.fragment:
            raise ValueError("address must be an http(s) origin without credentials, path, query, or fragment")
        if port is not None and not 1 <= port <= 65535:
            raise ValueError("invalid port")
        if not (0.1 <= args.timeout <= 10) or not (0 <= args.observe_seconds <= 10) or not (1 <= args.max_messages <= 100):
            raise ValueError("bounds: timeout 0.1..10s, observation 0..10s, messages 1..100")
        base = args.address.rstrip("/")
    except ValueError:
        print(json.dumps({"error": "invalid address or bounds"}))
        return 2
    output = {"read_only": True, "endpoints": {}}
    for path in ALLOWED_PATHS:
        value, error = fetch(base, path, args.timeout)
        output["endpoints"][path] = {"ok": error is None, "data": value, "error": error}
    if args.observe_seconds:
        output["events"] = observe_events(base, args.observe_seconds, args.timeout, args.max_messages)
    state = output["endpoints"]["/api/state"]["data"]
    output["conclusion"] = "idle" if state and state["running"] is False else "not_established"
    print(json.dumps(output, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
