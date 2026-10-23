# Add-on ↔ Niles core communication inventory

The add-on owns Board Manager parsing, bounded local collection policy, and request intent. Niles owns household membership, room and satellite authority, speech synthesis, arbitration, permissions, and final transport. This inventory describes the checked-in implementation; synthetic and compiled evidence do not establish installed-host, live-device, selected-subset, or audible behavior.

| Boundary | Direction and payload | Status / owner | Proof boundary |
|---|---|---|---|
| Lifecycle | Niles invokes component lifecycle exports | Implemented by pinned WIT | Fake-import tests; compiled proof is synthetic |
| Configuration | Board Manager address, room, poll interval, enabled flags, mode, opaque audience intent | Parsed by add-on; persistence and authority remain Niles-owned | Disabled by default; identities are not resolved here |
| HTTP state | Add-on GETs `/api/version` and `/api/state` | Status and baseline poll only | Never promoted to real-time event ingress |
| Detection control | Add-on bodyless PUT `/api/start` or `/api/stop` | Implemented | Detection only; no power, reset, or game creation |
| Event provider | Board Manager `/api/events` WebSocket `{type,data}` text | Active-connection text frames up to 64 KiB enter `AnnouncementSession` | Non-text, malformed, oversized, disconnect, and stale frames invalidate; live ordering remains open |
| Reducer | Session validates darts and explicit partial/full removal markers | Implemented transport-independent policy | Synthetic JSON only; local total is not a credited match score |
| Delivery boundary | Session submits numeric text, room-bound intent, collection key, and mode | Core adapter records attempts; component adapter submits to Niles speech stream | `Accepted` means adapter accepted the request, not audible delivery |
| Speech stream | Component opens Niles 0.5 room-bound stream and submits text | Niles scheduler owns pending replacement and final output | IDs and status are tracked; selected-satellite routing and audibility remain unproven |
| Outcomes | Host returns pending or terminal outcome | Component preserves pending requests, polls status, records bounded terminal history | Poll errors retain pending; no automatic retry |
| Diagnostics | Component status exposes HTTP, WebSocket, reducer, and speech lifecycle state | Implemented | Diagnostics are not delivery proof |

## Active event and speech path

Announcement intent opens `/api/events`; `speech_stream_enabled` separately opens the configured room stream. A request is keyed by connection generation and collection serial. The component records the host request ID, polls `pending`, `delivered`, `busy`, `quiet`, `expired`, `failed`, `cancelled`, `replaced`, `rate-limited`, and `no-satellite`, and exposes those outcomes without collapsing them into one status. Reset and reconnect detach stale sessions before closing them and mark unresolved requests as locally discarded.

`Each dart → Total` submits each eligible dart and then the total. `Total only` submits only the total after the first explicit removal marker. Clear/shrink alone never submits. The room is a Niles authority boundary, not a selected-satellite list.

## Verification limits

`component/src/tests.rs` uses deterministic fake imports. `crates/autodarts-core/examples/announcement_replay.rs` exercises both policies over three collections. `component/tests/host_proof.rs` is wired to the pinned release checkout's real `SpeechSessionManager`, but its compilation requires the host checkout's native OpenSSL development dependency. None of these artifacts performs a live device write, household capture, installation, deployment, push, or audible test.
