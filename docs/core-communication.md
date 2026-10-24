# Add-on ↔ Niles core communication inventory

The add-on owns Board Manager parsing, bounded local collection policy, and request intent. Niles owns household membership, room and satellite authority, speech synthesis, arbitration, permissions, and final transport. This inventory describes the checked-in implementation; synthetic and compiled evidence do not establish installed-host, live-device, selected-subset, or audible behavior.

| Boundary | Direction and payload | Status / owner | Proof boundary |
|---|---|---|---|
| Lifecycle | Niles invokes component lifecycle exports | Implemented by pinned WIT | Fake-import tests; compiled proof is synthetic |
| Configuration | Board Manager address, one room, poll interval, enabled flags, mode | Parsed by add-on; persistence and authority remain Niles-owned | Both controls default disabled; room configuration is not household authorization |
| HTTP state | Add-on GETs `/api/version` and `/api/state` | Status and baseline poll only | Never promoted to real-time event ingress |
| Detection control | Add-on bodyless PUT `/api/start` or `/api/stop` | Implemented | Detection only; no power, reset, or game creation |
| Event provider | Board Manager `/api/events` WebSocket `{type,data}` text | Active-connection text frames up to 64 KiB enter `AnnouncementSession` | Non-text, malformed, oversized and overflow events discard pending speech and reset eligibility; stale connection IDs are ignored; live ordering remains open |
| Reducer | Session validates darts, explicit state/motion takeout evidence, and the third-dart deadline | Implemented transport-independent policy | Synthetic JSON only; local total is not a credited match score |
| Delivery boundary | Session submits numeric text, room-bound intent, collection key, and mode | Core adapter records attempts; component adapter submits to Niles speech stream | `Accepted` means adapter accepted the request, not audible delivery |
| Speech stream | Component opens Niles 0.5 room-bound stream and submits text | Niles scheduler owns pending replacement and final output | IDs and status are tracked; selected-satellite routing and audibility remain unproven |
| Outcomes | Host returns terminal outcomes in request-ID order; accepted requests remain locally pending until observed | Component polls on event ingress and the existing timer, retaining bounded history | Poll errors retain pending; no utterance retry |
| Diagnostics | Component status exposes HTTP, WebSocket, reducer, and speech lifecycle state | Implemented | Diagnostics are not delivery proof |

## Active event and speech path

Announcement intent opens `/api/events`; both announcement intent and `speech_stream_enabled` gate the configured room stream. A request is keyed by connection generation and collection serial. The component records the host request ID and exposes pending state separately from `delivered`, `busy`, `quiet`, `expired`, `failed`, `cancelled`, `replaced`, `rate-limited`, and `no-satellite`. Reset and reconnect detach stale sessions before closing them and mark unresolved requests as locally discarded. The released `close` import returns no result and completes asynchronously: a transient unavailable response to reopening is retried on subsequent event/timer callbacks, but no failed utterance is replayed.

`Each dart → Total` submits each eligible dart and then the total. `Total only` submits only the total after the third-dart deadline or explicit takeout, whichever comes first; one- and two-dart collections require explicit takeout. Clear/shrink alone never submits, and a later takeout after deadline completion only resets the removal episode. The room is a Niles authority boundary, not a selected-satellite list.

## Verification limits

`component/src/tests.rs` uses deterministic fake imports. `crates/autodarts-core/examples/announcement_replay.rs` exercises both policies over three collections. `component/tests/host_proof.rs` uses the pinned release's real scheduler with synthetic output gates; its direct `ort` version constraint must not re-enable native-TLS defaults absent from the released host. The unchanged candidate permits WebSocket ports 80/3180/443, so the compiled fixture requires free loopback port 3180. Active-transport preservation is separately checked through the real manager's public terminal receivers, not claimed as compiled-ingress evidence. See [announcement-path evidence](announcement-path.md) for actual run results and remaining boundaries. None of these fixtures performs a live device write, household capture, persistent installation, deployment or audible test.
