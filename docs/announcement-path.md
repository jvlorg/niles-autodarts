# Announcement path and evidence

## Implemented request lifecycle

The component owns `AnnouncementSession` and, when announcements are enabled, connects to the configured Board Manager `/api/events` endpoint under the declared WebSocket permission. Text frames up to 64 KiB enter JSON ingestion. Oversize text and binary frames enter invalid-input handling and invalidate the current journey. Unrelated accepted-format camera/status topics are ignored by policy. See [`component/src/lib.rs`](../component/src/lib.rs), [`component/niles-integration.toml`](../component/niles-integration.toml), and the checked-in [WIT 0.5 contract](../component/wit/niles-integration.wit).

Connected, disconnected, failed and overflow callbacks reset the session and require a clean empty baseline. Disablement, settings changes and reconnects also invalidate policy state. Component `stop` closes owned WebSocket and speech-stream handles. The slower `/api/state` timer poll remains status/baseline-only; it is not a real-time event feed.

For a configured-room request, the component opens a host speech stream when `speech_stream_enabled` is enabled, submits session requests, and polls asynchronous request statuses. `announcements_enabled` defaults off and controls Board Manager event ingress; speech-stream enablement is a separate setting. Niles retains household and room-routing authority. Room-bound requests do not prove a selected-satellite subset or validated household targets. Submission acceptance or a synthetic `Delivered` status does not establish that a person heard audio.

Core policy owns validated settings, JSON parsing, `LocalJourney`, collection eligibility and request identity. Each delivery attempt is recorded before adapter invocation, so unavailable or failed attempts are not blindly retried. Ambiguous/corrected/out-of-order evidence suppresses completion; duplicate partial/full markers do not repeat a collection. Eligible totals, incremental dart requests and clear/marker ordering follow the policy in the [canonical plan](plan.md). The core unavailable and recording adapters are separate from the component's host-bound speech adapter; an unavailable-adapter or replay statement does not mean the component lacks a speech path.

## Evidence levels

| Evidence | What it establishes | What it does not establish |
|---|---|---|
| Source and WIT wiring | Component lifecycle owns the session, WebSocket callbacks, room speech handle, submit/status calls and cleanup; WIT declares the imported interfaces | Compatibility with a released or installed Niles host |
| Deterministic component fixtures | Synthetic callback and recording-host cases exercise enabled/disabled behavior, event frames, policy requests, status polling and resets in [`component/src/tests.rs`](../component/src/tests.rs) and [`component/src/test_host.rs`](../component/src/test_host.rs) | Live Board Manager event semantics, household routing or human audibility |
| Optional compiled component test | [`compiled_component_runs_through_niles_websocket_policy_and_recording_speech_host`](../component/tests/host_proof.rs) can exercise a compiled component through Niles runtime WebSocket policy and a recording speech host | Real speech output; the host records requests and returns synthetic statuses |
| Installed/live/audible behavior | Not established by the evidence above | Installed-host compatibility, live event order and miss encoding, and end-to-end audible delivery remain unproven |

The optional compiled test is defined by [`component/host-proof/Cargo.toml`](../component/host-proof/Cargo.toml). Its path dependencies hardcode `/home/jleth/projects/niles-nil43`; supplying a Niles executable does not make those Cargo dependencies portable. [`scripts/verify.sh`](../scripts/verify.sh) accepts `--host-proof /absolute/path/to/niles` only with an executable argument and builds/tests through that local dependency setup. This optional path is not an installation procedure and does not prove audible behavior. This documentation task did not run it.

Separately, [`scripts/verify-host.py`](../scripts/verify-host.py) uses synthetic HTTP fixtures and a developer-supplied CLI in dry-run mode to inspect status rendering. It does not exercise WebSocket speech, install the component, or prove live/audible delivery. This documentation task did not run it. The normal maintained application verification chain is described in [`scripts/verify.sh`](../scripts/verify.sh); runtime suites were out of scope for this documentation change.

The checked-in [WIT provenance](../component/wit/PROVENANCE.md) identifies the frozen reference contract and explicitly does not establish a released minimum Niles version supporting it. Keep source-implemented, synthetic-tested, optional-host-tested, installed, live and audible evidence distinct.
