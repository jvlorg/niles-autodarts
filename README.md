# Niles Autodarts

Standalone Niles add-on for local Autodarts Board Manager status and detection control, with bounded score-announcement policy and component integration. This is a development checkpoint, not an installable or live-proven release.

The reusable `AnnouncementSession` owns parsing, `LocalJourney`, and request policy. The Niles component connects to local `/api/events` when announcement intent is enabled and sends frames through `AnnouncementSession::ingest_json`. A separately configured `speech_stream_enabled` gate controls opening the room-bound speech stream; event ingress alone does not mean speech is available. Niles retains household room/satellite authority and speech transport. See the [canonical plan](docs/plan.md), [core communication inventory](docs/core-communication.md), and [announcement path](docs/announcement-path.md).

The add-on validates local `{type,data}` events and computes detection-derived collection sums. These are not credited X01 scores, remaining scores, busts, or checkout results. Bull numbers 25 and 50 are valid; live miss encoding remains unresolved. Detection on/off controls Board Manager detection only, not physical power, reset, or game creation. Announcement settings default disabled and audience identities are configuration intent, not validated household targets. V1 room/mode behavior is not established end-to-end on an installed host. V2 TV scoreboard and V3 game selection remain later scope.

## Evidence and limits

| Area | Evidence type and source | What it establishes / limitation |
|---|---|---|
| Core/session replay | Synthetic-tested: [`announcement_replay.rs`](crates/autodarts-core/examples/announcement_replay.rs), parser/session tests in [`autodarts-core`](crates/autodarts-core/src/) | Exercises settings, JSON ingress, reducer and recording requests across three collections; fixtures are synthetic. |
| Component imports and lifecycle | Source-implemented and synthetic-tested: [`component/src/lib.rs`](component/src/lib.rs), [`component/src/tests.rs`](component/src/tests.rs), [pinned WIT](component/wit/niles-integration.wit) | Covers WebSocket callbacks, independent speech-stream setting, resets, malformed frames and cleanup using test imports; does not establish installed-host compatibility. |
| Optional compiled-host proof | Optional synthetic host-proof fixture: [`host_proof.rs`](component/tests/host_proof.rs), [`component/host-proof/Cargo.toml`](component/host-proof/Cargo.toml) | Supplies loopback events and `RecordingSpeech`; execution is not established here, and even a successful run would not establish installation or audible output. The standalone manifest depends on developer-local absolute `niles-core` and `niles-plugins` paths. Ordinary verification does not run this crate. |
| Installed, live and audible behavior | Not established | Compatible installed Niles behavior, real Board Manager event delivery/order and miss encoding, and end-to-end human-audible output remain to be validated. |

## Verification

The maintained application chain is `bash scripts/verify.sh`; it runs Rust formatting, workspace build, strict Clippy, tests, and the release Wasm build. The optional host-proof branch is separate and uses `--host-proof /absolute/path/to/niles`; its manifest still requires the developer-local absolute dependency paths described above. The application chain and host proof were not run for this documentation reconciliation.

No secrets, household captures, or local configuration belong in Git. No live board operation, provider call, deployment, or audible test was performed for this documentation reconciliation.
