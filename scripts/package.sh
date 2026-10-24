#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
version="0.1.0-alpha.2"
toolchain="${RUST_TOOLCHAIN:-1.99.0}"
tag=""
output=""

usage() {
  printf 'Usage: %s --tag v%s --output dist/v%s\n' "$0" "$version" "$version" >&2
}
while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag|--output)
      [[ $# -ge 2 ]] || { usage; exit 2; }
      if [[ "$1" == --tag ]]; then tag="$2"; else output="$2"; fi
      shift 2 ;;
    *) usage; exit 2 ;;
  esac
done
if [[ "$tag" != "v$version" || -z "$output" ]]; then
  usage
  exit 2
fi
if [[ "$output" != /* ]]; then output="$root/$output"; fi
dist_root="$(realpath -m -- "$root/dist")"
output="$(realpath -m -- "$output")"
if [[ "$output" != "$dist_root/"* ]]; then
  printf 'output must be inside %s\n' "$dist_root" >&2
  exit 2
fi
if [[ -e "$output" || -L "$output" ]]; then
  printf 'refusing to overwrite existing candidate directory: %s\n' "$output" >&2
  exit 2
fi
if ! git diff --quiet || ! git diff --cached --quiet; then
  printf 'release packaging requires a clean tracked source tree\n' >&2
  exit 2
fi
source_revision="$(git rev-parse HEAD)"
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  export CARGO_TARGET_DIR="$(realpath -m -- "$CARGO_TARGET_DIR")"
fi

# Validate every add-on version before the expensive host/component builds.
python3 - "$root" "${tag#v}" <<'PY'
import pathlib
import sys
import tomllib

root = pathlib.Path(sys.argv[1])
expected = sys.argv[2]
def load(path):
    return tomllib.loads((root / path).read_text())
workspace = load("Cargo.toml")["workspace"]
versions = {"workspace": workspace["package"]["version"],
            "development manifest": load("manifest.toml")["package"]["version"]}
names = set()
for member in workspace["members"]:
    package = load(f"{member}/Cargo.toml")["package"]
    names.add(package["name"])
    version = package["version"]
    versions[member] = workspace["package"]["version"] if version == {"workspace": True} else version
    for item in package.get("metadata", {}).get("niles", []):
        versions[f"{member} Niles metadata"] = item["version"]
locked = {p["name"]: p["version"] for p in load("Cargo.lock")["package"] if p["name"] in names}
if set(locked) != names:
    raise SystemExit("Cargo.lock is missing an add-on package")
versions.update(locked)
manifest = load("component/niles-integration.toml")
versions["install manifest"] = manifest["version"]
if any(version != expected for version in versions.values()):
    raise SystemExit(f"candidate version mismatch (expected {expected}): {versions}")
if manifest.get("min_niles") != "0.5.0" or manifest.get("interface") != "0.5":
    raise SystemExit("candidate requires min_niles 0.5.0 and interface 0.5")
PY

bash "$root/scripts/prepare-host.sh"
revision="$(tr -d '[:space:]' < "$root/scripts/niles-host-revision")"
cmp component/wit/niles-integration.wit .cache/niles/schemas/niles-integration-0.5.wit
cargo +"$toolchain" build --locked -p niles-autodarts-component --target wasm32-wasip2 --release
target_dir="${CARGO_TARGET_DIR:-$root/target}"
wasm="$target_dir/wasm32-wasip2/release/niles_autodarts_component.wasm"
[[ -s "$wasm" ]] || { printf 'missing component Wasm: %s\n' "$wasm" >&2; exit 1; }

mkdir -p "$(dirname "$output")"
stage="$(mktemp -d "${output}.tmp.XXXXXX")"
trap 'rm -rf -- "$stage"' EXIT
cp "$root/component/niles-integration.toml" "$stage/niles-integration.toml"
cp "$wasm" "$stage/integration.wasm"
python3 - "$root/component" "$stage" <<'PY'
import pathlib
import shutil
import sys
import tomllib

source, stage = map(pathlib.Path, sys.argv[1:])
manifest = tomllib.loads((stage / "niles-integration.toml").read_text())
logo = manifest.get("logo")
if logo is not None:
    path = pathlib.PurePosixPath(logo)
    reserved = {"niles-integration.toml", "integration.wasm", "provenance.json", "SHA256SUMS"}
    if not logo or path.is_absolute() or ".." in path.parts or path.name in reserved or "\n" in logo or "\\" in logo:
        raise SystemExit(f"unsafe or duplicate logo asset: {logo!r}")
    asset = (source / path).resolve()
    if not asset.is_relative_to(source.resolve()) or not asset.is_file():
        raise SystemExit(f"missing or external declared logo: {logo}")
    (stage / path).parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(asset, stage / path)
PY
host_target_dir="${CARGO_TARGET_DIR:-$root/.cache/niles/target}"
cli="$host_target_dir/debug/niles"
printf 'checker: %s plugin check %s\n' "$cli" "$stage/integration.wasm"
"$cli" plugin check "$stage/integration.wasm"

# No successful provenance/checksums survive a failed build, asset check or source change.
if ! git diff --quiet || ! git diff --cached --quiet || [[ "$(git rev-parse HEAD)" != "$source_revision" ]]; then
  printf 'source changed during packaging; refusing candidate provenance\n' >&2
  exit 1
fi
python3 - "$stage" "$output" "$tag" "$revision" "$source_revision" "$toolchain" "$cli" <<'PY'
import hashlib
import json
import pathlib
import subprocess
import sys

stage, output = map(pathlib.Path, sys.argv[1:3])
tag, host_revision, source, toolchain, cli = sys.argv[3:]
provenance = {
    "source_revision": source,
    "source_clean": True,
    "tag": tag,
    "host_revision": host_revision,
    "toolchain": subprocess.check_output(["rustc", f"+{toolchain}", "--version"], text=True).strip(),
    "build": {"command": ["cargo", f"+{toolchain}", "build", "--locked", "-p", "niles-autodarts-component", "--target", "wasm32-wasip2", "--release"], "exit_code": 0},
    "check": {"command": [cli, "plugin", "check", str(stage / "integration.wasm")], "exit_code": 0},
}
(stage / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
rows = [f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.relative_to(stage).as_posix()}"
        for path in sorted(stage.rglob("*")) if path.is_file()]
(stage / "SHA256SUMS").write_text("\n".join(rows) + "\n")
if output.exists():
    raise SystemExit(f"refusing to overwrite candidate: {output}")
stage.rename(output)
PY
printf 'candidate staged: %s\n' "$output"
printf 'host revision: %s\n' "$revision"
