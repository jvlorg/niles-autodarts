# Niles Autodarts

> **Development checkpoint, not an installable release.** The source now includes component WebSocket and room-bound speech-stream adapters, covered by synthetic host-import tests. The older library-only descriptions below and in `docs/` predate that wiring and need reconciliation; they must not be used as the current implementation inventory. Live event delivery, compatible installed-host operation and audible output remain unproven. The optional `component/host-proof` crate references a developer-local Niles checkout and is not portable as configured.

Publication verification: `bash scripts/verify.sh` passed formatting, workspace build, strict Clippy, 36 unit tests and the release Wasm component build. The maintained announcement replay and eight loopback probe tests passed. No live board operations or deployment were performed for this publication.

Standalone [Niles](https://github.com/MarkNygaard/niles) add-on for local Autodarts Board Manager status and detection control, plus a reusable library-only announcement session for bounded local JSON events.

Status: session/parser/reducer and recording/unavailable delivery boundary are synthetic-tested. The component has no real event ingress; the session is not wired into its lifecycle. Niles speech delivery is unavailable. Not installed or live-proven.

The candidate parses local `{type,data}` envelopes and validates local dart products; it does not depend on cloud authentication, X01 match authority, game control or game outcomes. Local totals are simple sums, not credited match scores. Bull numbers 25/50 are valid; miss encoding remains unresolved and is not claimed as supported.

Full removal as a collection boundary is an explicit candidate policy, not a verified Board Manager turn semantic. The reducer rejects malformed/range-invalid/count-mismatched states and suppresses ambiguous ordering, correction, reconnect and missing-throws cases. TotalOnly remains silent until explicit partial/full marker, including when clear arrives first. No special/game outcome speech is generated.

The Niles component polls HTTP state for status only; slow polling is not treated as a real-time event feed. Pinned component WIT lacks a WebSocket client import. Offline replay runs validated settings and synthetic JSON through the application session, reducer, and recording adapter for three collections; fixtures are examples, not captures. Run `cargo run -q -p autodarts-core --example announcement_replay`.

Detection on/off is Board Manager detection only—not physical power, reset or game creation. V1 also calls for room selection and announcements through selected room satellites in Each dart → Total and Total only modes; these are not complete end-to-end. Announcement settings are disabled by default. Audience IDs remain unvalidated intent. Production delivery is unavailable; there is no notification fallback, sidecar or direct satellite transport. V2 TV scoreboard and V3 game selection remain later roadmap scope.

See [canonical plan](docs/plan.md), [core communication inventory](docs/core-communication.md), and [announcement path boundary](docs/announcement-path.md).

## Verification

Run `bash scripts/verify.sh`, `uv run --with websockets==15.0.1 python scripts/test_probe_board.py`, and `python3 scripts/verify-host.py /absolute/path/to/niles` for formatting/build/tests, probe loopback tests, and synthetic local-host diagnostics respectively. These do not constitute live event, installation, audience-routing or audible proof. No secrets, household captures or local configuration belong in Git.
