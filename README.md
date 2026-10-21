# Niles Autodarts

> **Development checkpoint, not an installable release.** The checked-in component includes HTTP status and detection controls, host-managed WebSocket callbacks feeding its owned announcement session, and a room-bound speech-stream adapter. The implementation contract and tests are local evidence; compatible released/installed-host operation, real Board Manager event behavior, selected-satellite subset fulfillment and audible output remain unproven. The optional `component/host-proof` crate references a developer-local Niles checkout and is not portable as configured.

This standalone [Niles](https://github.com/MarkNygaard/niles) add-on reads local Autodarts Board Manager status, controls Board Manager detection, and can announce locally derived dart products and collection sums from bounded `{type,data}` event envelopes. It does not depend on cloud authentication, X01 match authority, game control or game outcomes. Local totals are simple sums, not credited match scores, remaining scores, busts or checkout results. Bull numbers 25/50 are valid; miss encoding remains unresolved and is not claimed as live supported.

The component validates its configured endpoint and permission grant before opening the host-managed WebSocket. Callback frames enter the owned session; a separate HTTP timer poll is status-only. Eligible requests use a room-bound speech stream with explicit unavailable/failure outcomes. Announcement enablement and speech-stream enablement are independent, disabled-by-default settings in [`component/niles-integration.toml`](component/niles-integration.toml). Detection on/off is a distinct Board Manager action, not physical power, reset or game creation. There is no notification fallback, sidecar or direct satellite transport.

The first explicit partial or full takeout marker completes an eligible cached collection once. Each dart → Total announces darts as reported and a local total at that marker; Total only stays silent until the marker. Clear alone never announces. Corrections, invalid or ambiguous ordering, and resets suppress pending collections until a clean empty baseline. The room-bound route does not establish fulfillment of V1's selected-satellite subset requirement: configured audience identities remain opaque and unvalidated. V2 TV scoreboard and V3 game selection remain later roadmap scope.

See the [canonical plan](docs/plan.md) for detailed parser, reducer and reset policy, source links, and the evidence inventory. The [core communication inventory](docs/core-communication.md) and [announcement path](docs/announcement-path.md) describe current boundaries and unresolved targeting/audio evidence.

## Verification

These are reference procedures; they were not run for this documentation change.

- `bash scripts/verify.sh` runs format checks, workspace build, strict Clippy, workspace tests and release `wasm32-wasip2` component build. Toolchain defaults to Rust 1.99.0 and can be overridden with `RUST_TOOLCHAIN`.
- `cargo run -q -p autodarts-core --example announcement_replay` runs the deterministic synthetic three-collection session replay.
- `uv run --with websockets==15.0.1 python scripts/test_probe_board.py` runs loopback probe tests.
- `python3 scripts/verify-host.py /absolute/path/to/niles` uses a supplied Niles CLI dev host for synthetic HTTP and dry-run diagnostics.
- `bash scripts/verify.sh --host-proof /absolute/path/to/niles` opts into the separate synthetic Niles runtime/Wasm harness and requires an executable host argument. The isolated `component/host-proof` crate also depends on absolute `/home/jleth/projects/niles-nil43` Rust crate paths in `component/host-proof/Cargo.toml`; providing a CLI path does not resolve these developer-local dependencies. This crate is not an ordinary runtime dependency and is excluded from default verification.

Core parser/session tests, component synthetic import doubles, the optional runtime harness, loopback probes and CLI dry-run each cover only their described synthetic paths. None establishes installed compatibility, real Board Manager event ordering or miss encoding, household target selection, or audible speech. No secrets, household captures or local configuration belong in Git.
