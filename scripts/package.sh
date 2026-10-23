#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
version="0.1.0-alpha.1"
toolchain="${RUST_TOOLCHAIN:-1.99.0}"
tag=""
output=""

usage() {
  printf 'Usage: %s --tag v%s --output dist/v%s\n' "$0" "$version" "$version" >&2
}
while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag) tag="${2:-}"; shift 2 ;;
    --output) output="${2:-}"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done
if [[ "$tag" != "v${version}" ]]; then
  printf 'tag must be v%s\n' "$version" >&2
  exit 2
fi
if [[ -z "$output" ]]; then
  usage
  exit 2
fi
if [[ "$output" != /* ]]; then
  output="$root/$output"
fi
dist_root="$(realpath -m -- "$root/dist")"
output="$(realpath -m -- "$output")"
if [[ "$output" != "$dist_root/"* ]]; then
  printf 'output must be inside %s\n' "$dist_root" >&2
  exit 2
fi
if [[ -e "$output" ]]; then
  printf 'refusing to overwrite existing candidate directory: %s\n' "$output" >&2
  exit 2
fi
if ! git -C "$root" diff --quiet || ! git -C "$root" diff --cached --quiet; then
  printf 'release packaging requires a clean tracked source tree\n' >&2
  exit 2
fi

bash "$root/scripts/prepare-host.sh"
revision="$(tr -d '[:space:]' < "$root/scripts/niles-host-revision")"
cargo +"$toolchain" build -p niles-autodarts-component --target wasm32-wasip2 --release
target_dir="${CARGO_TARGET_DIR:-$root/target}"
wasm="$target_dir/wasm32-wasip2/release/niles_autodarts_component.wasm"
[[ -f "$wasm" ]] || { printf 'missing component Wasm: %s\n' "$wasm" >&2; exit 1; }

mkdir -p "$output"
cp "$root/component/niles-integration.toml" "$output/niles-integration.toml"
cp "$wasm" "$output/integration.wasm"
host_target_dir="${CARGO_TARGET_DIR:-$root/.cache/niles/target}"
cli="$host_target_dir/debug/niles"
"$cli" plugin check "$output"

python3 - "$output" "$tag" "$revision" "$root" <<'PY'
import hashlib
import pathlib
import subprocess
import sys
import tomllib

stage = pathlib.Path(sys.argv[1])
tag = sys.argv[2]
host_revision = sys.argv[3]
root = pathlib.Path(sys.argv[4])
manifest_path = stage / "niles-integration.toml"
wasm_path = stage / "integration.wasm"
with manifest_path.open("rb") as handle:
    manifest = tomllib.load(handle)
expected = tag.removeprefix("v")
checks = {
    "version": manifest.get("version") == expected,
    "min_niles": manifest.get("min_niles") == "0.5.0",
    "interface": manifest.get("interface") == "0.5",
    "wasm": wasm_path.is_file() and wasm_path.stat().st_size > 0,
}
if not all(checks.values()):
    raise SystemExit(f"candidate metadata/assets invalid: {checks}")
source = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
(stage / "PROVENANCE").write_text(
    f"source_revision={source}\nhost_revision={host_revision}\ntag={tag}\n"
)
rows = []
for path in sorted(stage.iterdir()):
    if path.name == "SHA256SUMS" or not path.is_file():
        continue
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    rows.append(f"{digest}  {path.name}")
(stage / "SHA256SUMS").write_text("\n".join(rows) + "\n")
PY

printf 'candidate staged: %s\n' "$output"
printf 'host revision: %s\n' "$revision"
printf 'checker: %s plugin check %s\n' "$cli" "$output"
