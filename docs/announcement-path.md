# Announcement-path feasibility

Status: the checked-in component has a room-bound Niles speech-stream path and explicit host-outcome bookkeeping. It is an implementation candidate, not an installed or audible proof. Niles retains household and speech authority.

## Current path

The component opens Board Manager `/api/events` when announcement intent is enabled. `speech_stream_enabled` separately gates a room-bound stream for the configured room. `AnnouncementSession` parses bounded text frames and emits numeric request intent. The component submits text to Niles, records host request IDs, polls status, and closes the stream on stop or reset. HTTP `/api/state` remains status/baseline-only.

The component distinguishes accepted submission from delivery. Pending requests remain visible across polling failures; terminal outcomes are retained individually; no request is retried automatically. Reconnect, disablement, settings changes, overflow, and malformed/stale ingress invalidate the speech generation and locally discard unresolved requests rather than allowing old commentary to cross a boundary.

## Modes and scheduler constraint

`Each dart → Total` submits each eligible dart and then the total. `Total only` submits only the total at the first explicit partial/full removal marker. These are detection-derived local values, not game-authoritative scores. The released Niles scheduler permits one replaceable pending request; therefore `Accepted` does not promise that every rapid utterance reaches a satellite. This is a host scheduling contract, not a reason to invent an add-on retry queue.

## Proof and unresolved work

Fake-import callback tests cover lifecycle, request IDs, polling recovery, terminal outcomes, overflow, and reset behavior. The compiled proof uses `niles_plugins::Runtime` plus the pinned release checkout's real `SpeechSessionManager` and a recording output callback. Its current environment prerequisite is the host checkout's native OpenSSL development package; no pass is claimed while that dependency is absent.

Even when compiled, the proof uses synthetic HTTP/WebSocket events and recording speech. It does not establish installed-host compatibility, live event ordering, live miss encoding, Niles household validation, configured-audience-to-selected-satellite routing, or human audibility. No host-private Rust function is treated as an add-on API. No live device, deployment, push, or publication is performed here.
