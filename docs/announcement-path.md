# Announcement-path feasibility

Current status: this repository pins a room-bound `speech-stream` import and implements a component lifecycle against it. Deployment compatibility and audible delivery remain unproven. See the [canonical plan](plan.md) for the current evidence ledger and remaining work.

## Historical research finding

The following observations came from an earlier Niles source snapshot and describe that research, not the current checked-in add-on contract or a currently installed host.

Source snapshot: `/home/jleth/projects/niles`, commit `eea62f46f275b31da8517d7f895fe4d0178f543d`, clean working tree when inspected, local branch 41 commits behind its origin tracking ref. This is the exact source basis, not a claim about newer upstream code.

A later contract check independently inspected upstream `origin/main` at `2f4d54b739e001d2e4e1fe4e173bf4400dae476a`. That historical check found MQTT, secrets and inbound interfaces but no dedicated announcement/Piper/satellite audience interface; `notify.say` was a separate notification feature. The internal transport references below were inspected in the earlier local source revision and are not exported plugin interfaces:

- `crates/niles-bin/src/main.rs:1312-1410`: `WyomingDelivery` implements the notification delivery callback. It resolves a room, uses the live peer when present, otherwise selects the configured IP, synthesizes via Piper and decodes/resamples PCM.
- Live connection: `crates/niles-bin/src/speak.rs:386-415`, `speak_back`, performs Piper synthesis, WAV→PCM conversion, satellite format/volume processing and `WyomingSender::send_audio_then`.
- Idle satellite: `crates/niles-bin/src/push.rs:22-87`, `speak_to`, connects to configured satellite IP on port 10301 and writes Wyoming AudioStart/AudioChunk/AudioStop. Caller is `main.rs:1388-1404`.
- Live Wyoming transport: `crates/niles-wyoming/src/server.rs:219-301`, `WyomingSender::send_audio_then`.
- `crates/niles-bin/src/satellites.rs:11-87`, `SatelliteRegistry`, maps configured IPs to room and volume; `ip_for(room)` returns one configured IP.

These historical host-internal Rust symbols showed Niles could perform those operations in that snapshot; they did not establish an add-on interface.

## Checked-in component path

The repository pins WebSocket and speech-stream imports in [`component/wit/niles-integration.wit`](../component/wit/niles-integration.wit). The component implementation in [`component/src/lib.rs`](../component/src/lib.rs) opens the optional WebSocket event path and a room-bound speech session, routes bounded text callbacks through `AnnouncementSession`, submits/polls speech requests, and closes handles on shutdown. Its settings and grants are in [`component/niles-integration.toml`](../component/niles-integration.toml). These are the repository’s pinned contract and implementation; they do not prove compatibility with an installed Niles version or resolved household targets.

Synthetic component tests in [`component/src/tests.rs`](../component/src/tests.rs) cover exercised room-bound open, callback-to-speech submission, request polling and lifecycle behavior. The WIT explicitly limits async request outcomes: submission acceptance and later transport status do not establish human audibility. Those tests establish only behavior with synthetic host imports. The optional real-host path uses recording speech and also does not prove audibility.

Niles remains responsible for household authorization, audience resolution and speech execution. The current component path is room-bound and does not establish arbitrary multi-room or selected-satellite targeting. Do not build against historical private functions, treat notification delivery as announcement delivery, duplicate Wyoming transport or claim an executable audio proof.

## Remaining work

Remaining work is compatible-host installation and contract verification, live Board Manager event ordering and miss-encoding observation, actual household-target verification, and end-to-end audio/audibility evidence. The ingress and room-bound speech adapter are present in source; their live host behavior remains unproven. Event-feed feasibility and speech delivery remain separate questions. The live encoding of misses as `(0,0)` versus `(0,1)` is unresolved, and authoritative match score, turn-completion, correction, bust and checkout semantics remain outside the local detection total.
