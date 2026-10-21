# Announcement-path feasibility

## Current component boundary

The component-owned announcement session sends eligible requests through the checked-in room-bound speech-stream adapter. Niles owns the speech-stream lifecycle and household routing. The component validates the configured endpoint and permission grant before opening host-managed WebSocket ingress; the separate HTTP polling path remains status-only. Announcement and speech-stream enablement are independent, disabled-by-default manifest settings. Submission can report unavailable or failed outcomes; a submitted request is not proof that anyone heard speech.

The checked-in evidence is [`component/src/lib.rs`](../component/src/lib.rs), [`component/wit/niles-integration.wit`](../component/wit/niles-integration.wit), [`component/niles-integration.toml`](../component/niles-integration.toml), and [WIT provenance](../component/wit/PROVENANCE.md). The provenance identifies a local source and revision but does not record license terms; it does not certify released Niles compatibility. Component import doubles in [`component/src/tests.rs`](../component/src/tests.rs) and [`component/src/test_host.rs`](../component/src/test_host.rs) cover simulated callbacks and adapter behavior. The optional built-Wasm harness in [`component/tests/host_proof.rs`](../component/tests/host_proof.rs) uses loopback HTTP/WebSocket fixtures and `RecordingSpeech`; it is not installed-host compatibility or sound proof. Its developer-local dependencies are described in the [README](../README.md).

The implemented seam is room-bound. V1's persisted selected-satellite-subset requirement remains unresolved: the [settings and request types](../crates/autodarts-core/src/lib.rs) carry one syntactically validated room, with no satellite-list setting or sorting/deduplication. This does not establish satellite validation, authorization or subset routing. Do not invent satellite-ID or private audio imports, substitute notification delivery, or copy direct Wyoming transport. See the [core communication inventory](core-communication.md).

## Historical Niles research

The following observations belong to the earlier investigation and its inspected revisions, not the current add-on contract.

Source snapshot: `/home/jleth/projects/niles`, commit [`eea62f46f275b31da8517d7f895fe4d0178f543d`](https://github.com/MarkNygaard/niles/commit/eea62f46f275b31da8517d7f895fe4d0178f543d), clean working tree when inspected, local branch 41 commits behind its origin tracking ref. This is the exact source basis, not a claim about newer upstream code.

A follow-up fetched `origin/main` and independently inspected its WIT at [`2f4d54b739e001d2e4e1fe4e173bf4400dae476a`](https://github.com/MarkNygaard/niles/commit/2f4d54b739e001d2e4e1fe4e173bf4400dae476a) without changing the checkout. The newer contract added MQTT, secrets and inbound interfaces, but the inspected full plugin import list had no dedicated announcement/Piper/satellite audience interface. `notify.say` was explicitly notification delivery, not the requested announcement contract. This updates that revision-specific contract check only; it does not describe the checked-in add-on WIT above.

The investigation examined private host functions at the earlier `eea62f46f275b31da8517d7f895fe4d0178f543d` revision:

- `crates/niles-bin/src/main.rs` `WyomingDelivery` implemented notification delivery: it resolved a room, used a live peer when present, otherwise selected the configured IP, synthesized via Piper and decoded/resampled PCM.
- `crates/niles-bin/src/speak.rs` `speak_back` performed Piper synthesis, WAV-to-PCM conversion and satellite format/volume processing.
- `crates/niles-bin/src/push.rs` `speak_to` connected to configured satellite IP on port 10301 and wrote Wyoming audio messages.
- `crates/niles-wyoming/src/server.rs` `WyomingSender::send_audio_then` implemented live transport.
- `crates/niles-bin/src/satellites.rs` `SatelliteRegistry` mapped configured IPs to room and volume; `ip_for(room)` returned one configured IP. The inspected route was room resolution, not a persisted selected-subset or satellite-ID audience API.

Those symbols were host-internal Rust, not exported plugin interfaces. The investigation found that Niles itself could perform those operations, not that an add-on could call them. Its notification distinction remains useful historical context: notification delivery and announcements are not interchangeable. No private route or direct device transport should be copied into this add-on.

## Reuse, proof and remaining work

The older investigation reported that no implementation artifact had been copied or reused at that point. Current WIT provenance records the checked-in contract's source and revision; it does not substantiate the older licensing assertion. Core parser/session tests and deterministic replay establish synthetic policy/request behavior; component import doubles establish synthetic callback behavior; the optional runtime harness records synthetic speech outcomes. None establishes actual audio, selected-subset semantics, compatible installed operation, or live event/removal/miss behavior.

Speech delivery and event-feed semantics are separate questions. The current checked-in contract has a WebSocket callback and room-bound speech stream; real Board Manager ordering/removal/miss behavior, released-host compatibility, target selection and audible household delivery remain unproven. Authoritative X01 scores, turn-completion, corrections, busts and checkout semantics remain outside this add-on's local detection-derived totals.
