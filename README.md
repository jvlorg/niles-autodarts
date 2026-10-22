# Niles Autodarts

Standalone [Niles](https://github.com/MarkNygaard/niles) add-on for local Autodarts Board Manager status, detection control, and score-announcement policy. This is development source, not an installable release. See the [canonical plan](docs/plan.md), [core communication inventory](docs/core-communication.md), and [announcement path](docs/announcement-path.md) for implementation scope and evidence.

## Implementation status

The transport-independent `AnnouncementSession` validates settings, parses bounded local JSON and reduces collection state. The component connects to the Board Manager `/api/events` WebSocket when announcement intent is enabled and routes active-connection text frames through the session. Its optional room-bound speech adapter opens only when `speech_stream_enabled` is enabled for the configured room; it submits requests, tracks host request IDs, polls status, and closes the session on stop. Core replay and component callback tests cover these paths with deterministic synthetic data and mock/recording delivery.

This evidence does not establish compatible installed-host operation, live Board Manager event ordering or miss encoding, selected-satellite routing, or audible playback. Configured audience identities remain intent; household resolution and speech authority stay in Niles. The optional compiled host-proof crate uses synthetic events and recording speech, and its Cargo manifest depends on `niles-core` and `niles-plugins` through absolute paths in a developer-local Niles checkout, so it is not portable as configured. Supplying only a Niles CLI executable does not resolve those dependencies.

The parser accepts local `{type,data}` envelopes and validates local dart products; it does not depend on cloud authentication, X01 match authority, game control, or game outcomes. Local totals are simple detection-derived sums, not credited match scores. Bull numbers 25/50 are valid; miss encoding remains unresolved and is not claimed as live-validated.

The reducer treats full removal as a collection boundary under its explicit candidate policy, not as a verified universal Board Manager turn semantic. It rejects malformed/range-invalid/count-mismatched states and suppresses ambiguous ordering, correction, reconnect and missing-throws cases. Each dart → Total announces individual local darts and then the eligible total; Total only remains silent until the first explicit partial/full removal marker. Clear/shrink alone never announces. No special/game outcome speech is generated.

The slow `/api/state` poll is status/baseline-only and is not a real-time event feed. Detection on/off controls Board Manager detection only; it does not control physical power, reset, or game creation. V1 includes room selection and local announcements in both modes, while selected-satellite audience behavior remains unresolved. Announcement settings default disabled. V2 TV scoreboard and V3 game selection remain later roadmap scope.

## Verification
### Evidence at a glance

| Evidence | Existing basis | Limits |
|---|---|---|
| Source implementation | [Component source](component/src/lib.rs): conditional `/api/events` ingress and gated room-bound speech open/submit/status/close. | Source wiring is not proof of installed-host compatibility, selected-satellite routing or audible playback. |
| Synthetic tests | [Core policy](crates/autodarts-core/src/local_score.rs), [replay](crates/autodarts-core/examples/announcement_replay.rs), and [component callbacks](component/src/tests.rs) with [fake imports](component/src/test_host.rs): deterministic policy and callback/speech-lifecycle coverage. | Synthetic inputs and mock/recording delivery, not live Board Manager events or real speech output; no tests were run for this documentation revision. |
| Optional compiled host proof | [Host-proof test](component/tests/host_proof.rs): if run, the compiled component through `niles_core::Runtime` with synthetic events and recording speech. | [Manifest](component/host-proof/Cargo.toml) uses absolute developer-local Niles checkout dependencies; a CLI path alone is insufficient. Not installed CLI speech-flow, live-event or audible proof. |
| Live / audible verification | Not established by the existing evidence. | Installed-host compatibility, live event ordering and miss encoding, selected-satellite routing, and audible playback remain unproven. |

See [Verification and proof boundaries](docs/plan.md#verification-and-proof-boundaries) for the detailed evidence and exclusions.

`bash scripts/verify.sh` runs formatting, workspace build, strict Clippy, workspace tests and the release Wasm component build. The optional `--host-proof /absolute/path/to/niles` branch first checks that the CLI executable exists, then runs the separate compiled `component/host-proof` test. That crate currently uses absolute developer-local Niles checkout paths; a CLI path alone cannot make a fresh checkout portable. Its proof uses synthetic events and recording speech and does not prove an installed CLI speech flow.

`uv run --with websockets==15.0.1 python scripts/test_probe_board.py` runs the maintained probe tests. Separately, `python3 scripts/verify-host.py /absolute/path/to/niles` exercises synthetic loopback HTTP status/render behavior through a Niles CLI and built component; it does not exercise WebSocket or speech lifecycle behavior.

The publication record for revision `c0e6b23629914e371c7e2525b4c9c1d972fcf3b9` reported that `bash scripts/verify.sh` passed formatting, workspace build, strict Clippy, 36 unit tests and the release Wasm component build; it also recorded the announcement replay and eight loopback probe tests passing. These are historical results, not checks run for this documentation change or a guaranteed current test count. No live board operation, installation or deployment is evidenced by that record. Keep credentials, household captures and local configuration out of Git.
