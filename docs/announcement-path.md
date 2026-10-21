# Announcement-path feasibility

Status: actual host adapter and audible transport remain unavailable and deferred. The addon now has an injectable policy/request boundary and synthetic recording journey; this is not a Niles binding or executable transport proof.

## Finding

Score speech is intended as Piper synthesis → selected satellites. Niles has a real host-owned audio route, but the plugin contract does not expose it. The apparent WIT `notify.say` route is a separate notification feature; its permission/quota must not be treated as a global limit on audio announcements.

Source snapshot: `/home/jleth/projects/niles`, commit `eea62f46f275b31da8517d7f895fe4d0178f543d`, clean working tree when inspected, local branch 41 commits behind its origin tracking ref. This is the exact source basis, not a claim about newer upstream code.

## Actual host route (private functions)

Main follow-up: fetched `origin/main` and independently inspected its WIT at `2f4d54b739e001d2e4e1fe4e173bf4400dae476a` without changing the checkout. The newer contract adds MQTT, secrets and inbound interfaces, but its full plugin import list still has no dedicated announcement/Piper/satellite audience interface. `notify.say` remains explicitly notification delivery (lines 389-406), not the requested announcement contract. This updates the contract check only; internal transport references below remain pinned to the local source revision above. New ingress capabilities deserve separate assessment before concluding event delivery is blocked.

- `crates/niles-bin/src/main.rs:1312-1410`: `WyomingDelivery` implements the notification delivery callback. It resolves a room, uses the live peer when present, otherwise selects the configured IP, synthesizes via Piper and decodes/resamples PCM.
- Live connection: `crates/niles-bin/src/speak.rs:386-415`, `speak_back`, performs Piper synthesis, WAV→PCM conversion, satellite format/volume processing and `WyomingSender::send_audio_then`.
- Idle satellite: `crates/niles-bin/src/push.rs:22-87`, `speak_to`, connects to configured satellite IP on port 10301 and writes Wyoming AudioStart/AudioChunk/AudioStop. Caller is `main.rs:1388-1404`.
- Live Wyoming transport: `crates/niles-wyoming/src/server.rs:219-301`, `WyomingSender::send_audio_then`.
- `crates/niles-bin/src/satellites.rs:11-87`, `SatelliteRegistry`, maps configured IPs to room and volume; `ip_for(room)` returns one configured IP. The inspected route is room resolution, not a persisted selected-subset or satellite-ID audience API.

These implementation symbols are host-internal Rust, not exported plugin interfaces. Their existence proves Niles itself can perform these operations, not that an add-on can call them.

## Add-on boundary

Pinned add-on contract: `component/wit/niles-integration.wit` is Niles integration 0.1; provenance records core revision and license at `component/wit/PROVENANCE.md`. WIT `notify.say` at lines 360-377 accepts message, priority and optional room. It does not accept PCM or satellite identities. Plugin world imports at lines 593-605 include notify, HTTP, devices, events and timers, but no Piper, audio, Wyoming or satellite interface. Sandbox notes at lines 586-592 refuse sockets and omit `wasi:http/outgoing-handler`. Household event subscription is not arbitrary provider-frame ingress.

Conclusion: no currently supported add-on path to Piper→satellite announcement audio or selected-satellite targeting was found. Actual host integration and safety design are deferred. The addon-side injectable request boundary is implemented independently; its recording implementation is synthetic only. Do not build an adapter against private functions, equate notifications with announcements, duplicate Wyoming transport, or claim an executable protocol proof. The add-on cannot exercise Niles's actual transport through its supported surface; synthetic recording proves requests only, not actual delivery.

## Smallest candidate host seam (proposal, not an API)

If V1 keeps persisted selected-satellite choice, a minimal candidate is a host-owned, permission-gated announcement operation taking text and an explicit list of validated configured satellite identities. Niles retains Piper synthesis and selects the appropriate live-send or idle-push route. The host contract must define target validation, delivery result/error, limits and behavior when targets are unavailable. This is a proposal only, deferred to a later review; no API is claimed. No Niles/core changes, sidecar, localhost backdoor or direct device write is authorized here. Room-only targeting is not equivalent to the agreed selected subset. See [core communication inventory](core-communication.md) for addon source-by-source status and safety questions.

## Reuse/licensing and proof

No implementation artifact was copied or reused. Existing WIT provenance identifies the Niles integration contract as MIT OR Apache-2.0 material. An offline synthetic journey now exercises policy → recording requests and verifies the explicit unavailable production placeholder; it is not a transport proof. Exercising actual audio would require invoking private Niles host code, while this add-on has no supported import for it. No PCM was sent and no live API, provider, device or audio operation was performed.

## Separate unresolved work

Speech delivery feasibility is distinct from event-feed feasibility. Current WIT has no WebSocket ingress; authoritative Autodarts match/score, turn-completion, correction, bust and checkout semantics remain separately unverified. Neither question is answered by the other.
