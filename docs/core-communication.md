# Add-on ↔ Niles core communication inventory

Scope: checked-in add-on sources and pinned [Niles WIT contract](../component/wit/niles-integration.wit). Niles household room/satellite authority and speech transport remain core-owned. The component owns provider connection lifecycle and announcement policy integration; checked-in interfaces do not establish universal compatibility with installed Niles versions.

| Boundary | Direction and payload | Status / owner | Limits and proof |
|---|---|---|---|
| Lifecycle | Core invokes component lifecycle exports and connection callbacks | Implemented in pinned WIT and component | Synthetic component-import coverage; validate compatibility on intended installed host |
| Configuration | Core config supplies Board Manager address, room, poll interval, announcement enabled/mode/audience and `speech_stream_enabled` | Implemented parsing; Niles owns persistence | Announcement settings default disabled; audience identities are opaque intent, not resolved/authorized satellites |
| HTTP state | Add-on → Board Manager GET `/api/version`, GET `/api/state` | Implemented status poll | Timer poll is not an event feed; snapshots do not announce existing darts |
| Detection control | Add-on → Board Manager bodyless PUT `/api/start` or `/api/stop`, then state reread | Implemented | Detection only; no reset/game start; synthetic callback tests only |
| Event provider | Component → local Board Manager `/api/events`; callback messages contain `{type,data}` envelopes | Implemented WebSocket URL/grant construction, callbacks and `AnnouncementSession::ingest_json` in [`component/src/lib.rs`](../component/src/lib.rs) | Synthetic component tests; no live Board Manager delivery/order or miss-encoding evidence |
| Session lifecycle/config | Component runtime owns validated `AnnouncementSettings` + `LocalJourney`; reconfiguration, connection resets and parse errors invalidate state | Implemented application session and component integration | Replay and synthetic lifecycle tests; a clean baseline is required after invalidation |
| Reducer recovery | Session parses each JSON frame and maps parse failures to invalidation | Implemented | Nonempty baselines suppress; invalid/corrected/ambiguous collections require clean empty baseline; first explicit partial/full marker completes an eligible cached local collection once |
| Speech stream | Component → Niles room-bound speech-stream import | Implemented when `speech_stream_enabled` is true and opening succeeds | Separate from announcement intent and WebSocket ingress; acceptance records a bounded request, polled outcomes report transport status, never human audibility |
| Delivery request | Session → component `AnnouncementDelivery`: numeric text, configured opaque audience, epoch/collection key, explicit outcome | Implemented room-bound stream adapter plus recording and explicit unavailable adapters | Niles resolves/authorizes household identities and owns speech transport; no notification substitution, private API, sidecar or direct satellite writes |
| Diagnostics | Component → Niles UI/status | Existing status and intent diagnostics | Status does not establish speech availability, score ingestion or human audibility |

## Component lifecycle and ownership

When announcement intent is enabled, the component builds a local Board Manager `/api/events` WebSocket URL and host grant, opens the connection, and processes matching callback messages through `AnnouncementSession::ingest_json`. The session owns JSON parsing and automatic invalidation on malformed input. WebSocket callback connection identity is checked so stale callbacks are ignored; disconnect/status and overflow events reset policy and require a clean baseline. Connection cleanup closes resources with the component lifecycle. The [component implementation](../component/src/lib.rs), [pinned WIT imports and callback exports](../component/wit/niles-integration.wit), and [synthetic component tests](../component/src/tests.rs) support these source and coverage claims.

Speech-stream creation has an independent `speech_stream_enabled` setting and uses the configured room. If ingress is enabled while speech is disabled or unavailable, events can reach policy without a usable speech stream. The delivery adapter submits bounded requests and polls outcomes; acceptance is not proof of playback. Niles remains responsible for household target resolution and actual speech delivery. Validate WIT/runtime compatibility and behavior on the intended installed host; the checked-in contract alone does not prove all hosts support it.

## Notifications and deferred compatibility work

Notifications remain distinct from score announcements. No notification fallback, direct satellite transport, sidecar, or Niles core modification is introduced. Remaining work is to validate compatibility and behavior on the intended installed host, observe actual Board Manager events and ordering (including miss encoding), and verify end-to-end audible output. Slow `/api/state` polling remains status/baseline-only and cannot replace `/api/events` ingress.

For full reducer rules, unresolved event semantics and evidence labels, see the [canonical plan](plan.md) and [announcement path](announcement-path.md).
