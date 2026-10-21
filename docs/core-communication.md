# Add-on ↔ Niles core communication inventory

Scope: checked-in add-on sources and pinned WIT contract. Niles household room/satellite authority and speech delivery remain core-owned. No proposed interaction below is represented as an existing API.

| Boundary | Direction and payload | Status / owner | Limits and proof |
|---|---|---|---|
| Lifecycle | Core invokes component lifecycle exports | Implemented via pinned WIT | Synthetic component tests and local synthetic-host check only |
| Configuration | Core config supplies Board Manager address, room, poll interval, announcement enabled/mode/audience | Implemented parsing; Niles owns persistence | Disabled by default; audience identities are opaque intent, not resolved/authorized satellites |
| HTTP state | Add-on → Board Manager GET `/api/version`, GET `/api/state` | Implemented status poll | Timer poll is not an event feed; snapshot handling does not announce existing darts |
| Detection control | Add-on → Board Manager bodyless PUT `/api/start` or `/api/stop`, then state reread | Implemented | Detection only; no reset/game start; synthetic callback tests only |
| Event provider | Board Manager → add-on `{type,data}` envelopes; candidate types `state`, `motion_state` | `AnnouncementSession::ingest_json` accepts one bounded raw JSON envelope and invokes parser/reducer; no real event ingress is wired | Bounded synthetic inputs only; no claims about production event ordering or miss encoding |
| Session lifecycle/config | Add-on owner holds validated `AnnouncementSettings` + `LocalJourney`; settings replacement and connection reset invalidate state | Implemented transport-independent reusable application library | Replay/tests exercise owner; not wired into component lifecycle/event transport |
| Reducer recovery | Session receives JSON and maps parse failures to reducer invalidation | Implemented library behavior | Nonempty baselines suppress; invalid/corrected/ambiguous collections require clean empty baseline; first explicit partial/full marker completes an eligible cached local collection once |
| Delivery request | Session → injected `AnnouncementDelivery`: numeric text, opaque configured audience, epoch/collection-scoped key, explicit outcome | Policy boundary implemented; recording adapter and explicit `UnavailableDelivery` | No Niles integration, notification substitution, private API, sidecar or direct satellite writes |
| Speech/audience | Future core adapter → Niles speech/satellite route | Not available in pinned supported imports | Core must own satellite resolution/authorization and speech routing; no audible proof |
| Diagnostics | Component → Niles UI/status | Existing status + intent diagnostics | Must continue to state delivery unavailable; no score-ingestion claim from slow poll |

## Candidate event adapter inventory

A future supported ingress adapter should own an `AnnouncementSession` per configured board/connection and call `ingest_json` for each bounded raw envelope. The session parses and invalidates automatically on parse failure; callers must not silently drop malformed input. Apply validated settings through `reconfigure` and call `connection_reset` on reconnect. Both operations require a new clean baseline. Nonempty snapshots suppress the entire existing collection and later appends. Only a clean empty/count-zero state restores eligibility. HTTP snapshots may initialize/reset baseline but cannot be promoted to real-time append events. No adapter is wired because pinned component WIT has no WebSocket client import.

The deferred core speech interface is not claimed to exist. The session-facing adapter contract is the current `AnnouncementDelivery` trait: `AnnouncementRequest` carries numeric text, configured opaque audience IDs, and a deduplication key; delivery returns explicit success or unavailable/failure. The host/core must resolve and authorize identities and own speech routing, error semantics, and actual delivery. This interface does not assert any WIT import or Niles implementation.

## Candidate total and order policy

The approved local contract is per-dart announcements as the API reports them in Each dart → Total mode, and no dart announcements in TotalOnly. The first `motion_state.isTakeoutPartial == true` or `isTakeoutFull == true` completes an eligible cached local collection once; a later partial/full marker is suppressed. This local detection total is not a game-authoritative score or game-turn judgment. If throws shrink/clear first, retain a bounded last validated pre-removal snapshot; shrink by itself never announces, but a later explicit marker can complete that snapshot. TotalOnly clear-before-marker also preserves the eligible cached collection and remains silent until the explicit marker. If evidence suggests correction, is invalid/out-of-order or otherwise ambiguous, suppress. After removal/clear, require a clean empty count-zero baseline before another collection. Source events provide no reliable timestamps, so no timestamp semantics are inferred. These are add-on policy rules, not proven universal Board Manager semantics or live compatibility.

Target lists are sorted and deduplicated. Delivery attempt keys include connection epoch and collection serial and are recorded before calling the adapter, preventing an uncertain failure from being retried as definitely unheard. Disabled intent, mode/audience change, and reconnect reset the reducer and require a new clean empty baseline. Recording-adapter proof is add-on policy only, not host transport or speech proof.
