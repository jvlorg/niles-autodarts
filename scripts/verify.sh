#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
toolchain="${RUST_TOOLCHAIN:-1.99.0}"
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  export CARGO_TARGET_DIR="$(realpath -m -- "$CARGO_TARGET_DIR")"
fi
host_target_dir="${CARGO_TARGET_DIR:-$root/.cache/niles/target}"
# Emit the exact commands, including proof inputs, in local/CI evidence.
set -x

quick() {
  cargo +"$toolchain" fmt --all -- --check
  cargo +"$toolchain" build --workspace
  cargo +"$toolchain" clippy --workspace --all-targets -- -D warnings
  cargo +"$toolchain" test --workspace
  cargo +"$toolchain" build -p niles-autodarts-component --target wasm32-wasip2 --release
  cargo +"$toolchain" run -p autodarts-core --example announcement_replay
  uv run --with websockets==15.0.1 python scripts/test_probe_board.py
  cmp AGENTS.md CLAUDE.md
}

if [[ $# -eq 1 && "$1" == "--quick" ]]; then
  quick
  exit 0
fi
if [[ $# -ne 0 ]]; then
  printf 'Usage: %s [--quick]\n' "$0" >&2
  exit 2
fi

quick
python3 scripts/test_release.py
bash scripts/prepare-host.sh
cmp component/wit/niles-integration.wit .cache/niles/schemas/niles-integration-0.5.wit
cargo +"$toolchain" fmt --manifest-path component/host-proof/Cargo.toml --all -- --check
cargo +"$toolchain" clippy --locked --manifest-path component/host-proof/Cargo.toml --all-targets -- -D warnings
bash scripts/package.sh --tag v0.1.0-alpha.2 --output dist/v0.1.0-alpha.2
python3 scripts/verify-host.py "$host_target_dir/debug/niles" --package-dir dist/v0.1.0-alpha.2
NILES_COMPONENT_WASM="$root/dist/v0.1.0-alpha.2/integration.wasm" \
  cargo +"$toolchain" test --locked --manifest-path component/host-proof/Cargo.toml -- --nocapture
uv run --with websockets==15.0.1 python scripts/test_probe_board.py
(cd dist/v0.1.0-alpha.2 && sha256sum --check SHA256SUMS)
actionlint .github/workflows/release.yml
cmp AGENTS.md CLAUDE.md
