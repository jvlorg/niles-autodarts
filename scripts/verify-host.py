"""Exercise the built component in a real Niles dev host, using synthetic HTTP only."""
import http.server
import pathlib
import shutil
import subprocess
import sys
import tempfile
import threading

root = pathlib.Path(__file__).resolve().parents[1]
requests = []

class Board(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        requests.append(self.path)
        bodies = {
            '/api/version': b'1.0.0-fixture',
            '/api/state': b'{"connected":true,"running":true,"numThrows":2}',
        }
        body = bodies.get(self.path)
        self.send_response(200 if body else 404)
        self.end_headers()
        self.wfile.write(body or b'')

    def log_message(self, format, *args):
        pass

server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Board)
thread = threading.Thread(target=server.serve_forever, daemon=True)
thread.start()
try:
    with tempfile.TemporaryDirectory(prefix='autodarts-host-') as folder:
        stage = pathlib.Path(folder)
        shutil.copy(root / 'component/niles-integration.toml', stage)
        shutil.copy(root / 'target/wasm32-wasip2/release/niles_autodarts_component.wasm', stage / 'integration.wasm')
        cli = str(pathlib.Path(sys.argv[1]).resolve())
        subprocess.run([cli, 'plugin', 'check', folder], check=True, timeout=30)
        result = subprocess.run([
            cli, 'plugin', 'dev', folder, '--dry-run', '--allow-loopback',
            '--settings', f'address=127.0.0.1:{server.server_port}',
            '--settings', 'room=living_room', '--render', 'board',
        ], capture_output=True, text=True, timeout=30)
        print(result.stdout)
        print(result.stderr)
        result.check_returncode()
        assert requests == ['/api/version', '/api/state'], requests
        for text in ['Board Manager: reachable', 'Running: true', 'Observed throws: 2']:
            assert text in result.stdout, f'Missing rendered evidence: {text}'
        print('PASS: real Wasm host init → synthetic GETs → rendered state; no live device access')
finally:
    server.shutdown()
    server.server_close()
    thread.join()
