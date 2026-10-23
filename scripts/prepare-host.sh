#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
remote="https://github.com/MarkNygaard/niles.git"
pin_file="$root/scripts/niles-host-revision"
cache="$root/.cache/niles"
cd "$root"
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  export CARGO_TARGET_DIR="$(realpath -m -- "$CARGO_TARGET_DIR")"
fi

if [[ ! -s "$pin_file" ]]; then
  printf 'missing host revision pin: %s\n' "$pin_file" >&2
  exit 2
fi
revision="$(tr -d '[:space:]' < "$pin_file")"
if [[ ! "$revision" =~ ^[0-9a-f]{40}$ ]]; then
  printf 'invalid host revision pin: %s\n' "$revision" >&2
  exit 2
fi

if [[ -e "$cache" && ! -d "$cache/.git" ]]; then
  printf 'host cache exists but is not a git checkout: %s\n' "$cache" >&2
  exit 2
fi

if [[ ! -e "$cache" ]]; then
  mkdir -p "$(dirname "$cache")"
  git init "$cache" >/dev/null
  git -C "$cache" remote add origin "$remote"
  if ! git -C "$cache" fetch --depth=1 origin "$revision"; then
    printf 'unable to fetch pinned Niles revision; network access or git prerequisites are required\n' >&2
    exit 1
  fi
  git -C "$cache" checkout --detach "$revision" >/dev/null
else
  actual_remote="$(git -C "$cache" remote get-url origin 2>/dev/null || true)"
  if [[ "$actual_remote" != "$remote" ]]; then
    printf 'host cache remote mismatch: expected %s, found %s\n' "$remote" "${actual_remote:-<none>}" >&2
    exit 2
  fi
  if ! git -C "$cache" diff --quiet || ! git -C "$cache" diff --cached --quiet; then
    printf 'host cache has tracked modifications; refusing to reset %s\n' "$cache" >&2
    exit 2
  fi
  actual_head="$(git -C "$cache" rev-parse HEAD 2>/dev/null || true)"
  if [[ "$actual_head" != "$revision" ]]; then
    printf 'host cache revision mismatch: expected %s, found %s\n' "$revision" "${actual_head:-<none>}" >&2
    exit 2
  fi
fi

# Resolve the checkout's released rust-toolchain.toml, not the caller's default.
if ! (cd "$cache" && cargo build --locked --manifest-path "$cache/Cargo.toml" --bin niles); then
  printf 'pinned Niles checkout is verified, but the CLI build failed; check supported Rust/native prerequisites or dependency access\n' >&2
  exit 1
fi

target_dir="${CARGO_TARGET_DIR:-$cache/target}"
cli="$target_dir/debug/niles"
if [[ ! -x "$cli" ]]; then
  printf 'host CLI build completed without expected executable: %s\n' "$cli" >&2
  exit 1
fi
printf 'Niles host revision: %s\n' "$revision"
printf 'Niles host checkout: %s\n' "$cache"
printf 'Niles CLI: %s\n' "$cli"
