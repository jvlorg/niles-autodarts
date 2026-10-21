#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
toolchain="${RUST_TOOLCHAIN:-1.99.0}"
cargo +"$toolchain" fmt --all -- --check
cargo +"$toolchain" build --workspace
cargo +"$toolchain" clippy --workspace --all-targets -- -D warnings
cargo +"$toolchain" test --workspace
cargo +"$toolchain" build -p niles-autodarts-component --target wasm32-wasip2 --release

if [[ "${1:-}" == "--host-proof" ]]; then
  host="${2:-}"
  if [[ -z "$host" || ! -x "$host" ]]; then
    printf 'Usage: %s --host-proof /absolute/path/to/niles\n' "$0" >&2
    exit 2
  fi
  NILES_COMPONENT_WASM="$root/target/wasm32-wasip2/release/niles_autodarts_component.wasm" \
    cargo +"$toolchain" test --manifest-path component/host-proof/Cargo.toml --test host_proof -- --nocapture
fi
