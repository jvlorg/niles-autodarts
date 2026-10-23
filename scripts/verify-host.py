"""Exercise a built component in a real Niles dev host with synthetic HTTP only."""
import os
import argparse
import http.server
import pathlib
import shutil
import subprocess
import tempfile
import threading

root = pathlib.Path(__file__).resolve().parents[1]
requests = []


class Board(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        requests.append(self.path)
        bodies = {
            "/api/version": b"1.0.0-fixture",
            "/api/state": b'{"connected":true,"running":true,"numThrows":2}',
        }
        body = bodies.get(self.path)
        self.send_response(200 if body else 404)
        self.end_headers()
        self.wfile.write(body or b"")

    def log_message(self, format, *args):
        pass


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("cli", type=pathlib.Path)
    parser.add_argument("--package-dir", type=pathlib.Path)
    args = parser.parse_args()
    cli = args.cli.resolve()
    if not cli.is_file() or not cli.stat().st_mode & 0o111:
        raise SystemExit(f"CLI is not executable: {cli}")

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Board)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    temporary = None
    try:
        if args.package_dir is None:
            temporary = tempfile.TemporaryDirectory(prefix="autodarts-host-")
            stage = pathlib.Path(temporary.name)
            shutil.copy(root / "component/niles-integration.toml", stage)
            wasm_target = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", root / "target"))
            shutil.copy(
                wasm_target / "wasm32-wasip2/release/niles_autodarts_component.wasm",
                stage / "integration.wasm",
            )
        else:
            stage = args.package_dir.resolve()
            if not stage.is_dir():
                raise SystemExit(f"package directory does not exist: {stage}")
            for name in ("niles-integration.toml", "integration.wasm"):
                if not (stage / name).is_file():
                    raise SystemExit(f"package is missing {name}: {stage}")

        subprocess.run([str(cli), "plugin", "check", str(stage)], check=True, timeout=30)
        result = subprocess.run(
            [
                str(cli),
                "plugin",
                "dev",
                str(stage),
                "--dry-run",
                "--allow-loopback",
                "--settings",
                f"address=127.0.0.1:{server.server_port}",
                "--settings",
                "room=living_room",
                "--render",
                "board",
            ],
            capture_output=True,
            text=True,
            timeout=30,
        )
        print(result.stdout)
        print(result.stderr)
        result.check_returncode()
        assert requests == ["/api/version", "/api/state"], requests
        for text in ["Board Manager: reachable", "Running: true", "Observed throws: 2"]:
            assert text in result.stdout, f"Missing rendered evidence: {text}"
        print("PASS: exact package → real Wasm host → synthetic GETs → rendered state")
        return 0
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
        if temporary is not None:
            temporary.cleanup()


if __name__ == "__main__":
    raise SystemExit(main())
