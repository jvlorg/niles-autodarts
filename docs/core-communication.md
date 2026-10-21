# Add-on ↔ Niles core communication inventory

Scope: checked-in add-on sources and the frozen WIT 0.5 contract. Provider parsing and detection policy stay in this add-on; Niles owns household configuration, room routing and speech delivery. The reference component implementation is [`component/src/lib.rs`](../component/src/lib.rs), the [WIT contract](../component/wit/niles-integration.wit), [provenance](../component/wit/PROVENANCE.md) and [permission manifest](../component/niles-integration.toml).

| Boundary | Direction and payload | Status / owner | Limits and proof |
|---|---|---|---|
| Lifecycle | Core invokes component lifecycle exports and callbacks | Component owns runtime session and host handles | Deterministic component fixtures; installed-host compatibility remains unproven |
| Configuration | Core supplies Board Manager address, room, poll interval, announcement enabled/mode, and speech-stream enabled | Component validates settings; Niles owns persistence and room identity | Announcements and speech stream default off; configured values do not prove household target membership |
| HTTP state | Add-on → Board Manager GET `/api/version`, GET `/api/state` | Implemented status poll | Timer poll is not a real-time event feed; snapshots do not announce existing darts |
| Detection control | Add-on → Board Manager bodyless PUT `/api/start` or `/api/stop`, then state reread | Implemented detection command/status behavior | Detection only; no hardware power, reset or game start; synthetic callback fixtures only |
| Event ingress | Board Manager WebSocket text → component callback → bounded JSON session parser/reducer | Implemented using WIT WebSocket import and manifest permission | Oversize and binary inputs invalidate; live event ordering and miss encoding remain unverified |
| Session policy | Component owns `AnnouncementSession`; core policy owns validated settings, `LocalJourney`, eligibility and request identity | Implemented in the add-on | Clean empty baseline required after reset; deterministic fixtures are synthetic evidence |
| Speech request | Session → component `AnnouncementDelivery` → WIT room speech stream submit/status | Implemented host-bound adapter | Request acceptance/status is not evidence of audible output; room-bound stream does not prove selected-satellite control |
| Cleanup | Component closes owned WebSocket and speech stream handles on stop | Implemented | Host runtime behavior after installation is not yet established |
| Diagnostics | Component → Niles UI/status | Existing status and intent diagnostics | Status describes connection/request state, not game score authority or human audibility |

## Implemented integration flow

When announcements are enabled, the component connects to Board Manager `/api/events` under the manifest's declared WebSocket permission. Text frames up to 64 KiB enter JSON ingestion; oversize frames and binary messages enter invalid-input handling. The session parser/reducer ignores unrelated accepted-format camera/status topics. Connected, disconnected, failed and overflow callbacks reset policy and require a clean empty baseline. Disablement, settings reconfiguration and connection changes also invalidate the journey; `stop` closes component-owned handles.

The component opens a speech stream for the configured room when its separate speech-stream setting is enabled. It submits numeric requests through the host interface and polls asynchronous statuses. Niles retains household and room routing authority. A room-bound request does not establish a selected satellite subset or validated household targets. Acceptance and synthetic `Delivered` outcomes do not establish human audibility.

The slower `/api/state` timer poll remains status/baseline-only and is not promoted to real-time event ingress. The checked-in WIT 0.5 schema is a frozen development contract; [its provenance](../component/wit/PROVENANCE.md) does not establish a released or installed-host compatibility guarantee.

## Policy and proof boundaries

The first explicit `motion_state.isTakeoutPartial == true` or `isTakeoutFull == true` completes an eligible cached local collection once. Duplicate markers are suppressed. In TotalOnly, clear-before-marker preserves the eligible collection silently until the marker. For Each dart → Total, a bounded last validated pre-removal snapshot may be completed only by a later explicit marker. Correction, invalid/out-of-order evidence and ambiguous ordering suppress; after completion or reset, require a clean empty count-zero baseline. These are add-on policy rules, not validated universal Board Manager semantics.

Each delivery request is keyed by connection epoch and collection serial, and attempts are recorded before invoking the adapter to prevent blind retries. The core's unavailable and recording adapters are distinct from the component's host-bound speech adapter. Component fixtures prove synthetic callback and recording-host behavior, not a live provider, installed host, selected-satellite route or audible result. See [announcement path and evidence](announcement-path.md).
