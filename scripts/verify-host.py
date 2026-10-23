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
    state_directory = None
    state_link = None
    try:
        if args.package_dir is None:
            temporary = tempfile.TemporaryDirectory(prefix="autodarts-host-")
            stage = pathlib.Path(temporary.name)
            shutil.copy(root / "component/niles-integration.toml", stage)
            wasm_target = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", root / "target"))
            if not wasm_target.is_absolute():
                wasm_target = root / wasm_target
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

        # The released dev command persists under the asset directory and reads
        # dev-settings.toml implicitly. Keep this diagnostic isolated and leave
        # the exact candidate untouched, including on timeout/failure.
        if (stage / "dev-settings.toml").exists():
            raise SystemExit("refusing non-synthetic dev-settings.toml in package")
        state_link = stage / ".niles-dev"
        if state_link.exists() or state_link.is_symlink():
            state_link = None
            raise SystemExit("refusing existing .niles-dev state in package")
        state_directory = tempfile.TemporaryDirectory(prefix="autodarts-dev-state-")
        state_link.symlink_to(state_directory.name, target_is_directory=True)

        # Directory lookup in Niles prefers a newer target/*.wasm when present.
        # An explicit file selects the exact candidate for both check and dev.
        wasm = stage / "integration.wasm"
        subprocess.run([str(cli), "plugin", "check", str(wasm)], check=True, timeout=30)
        result = subprocess.run(
            [
                str(cli),
                "plugin",
                "dev",
                str(wasm),
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
        for text in [
            "Board Manager: reachable", "Running: true", "Observed throws: 2",
            "Speech stream disabled",
        ]:
            assert text in result.stdout, f"Missing rendered evidence: {text}"
        print("PASS: exact package → real Wasm host → synthetic GETs → rendered state")
        return 0
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
        if state_link is not None and state_link.is_symlink():
            state_link.unlink()
        if state_directory is not None:
            state_directory.cleanup()
        if temporary is not None:
            temporary.cleanup()


if __name__ == "__main__":
    raise SystemExit(main())
