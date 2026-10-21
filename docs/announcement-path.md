# Announcement-path feasibility

The earlier contract research below records why the first implementation had no supported event ingress or speech binding at that time. The checked-in implementation now includes component WebSocket ingress and an optional room-bound Niles speech-stream adapter. Current source and evidence limits are summarized in the [canonical evidence ledger](plan.md#verification-and-proof-boundaries).

## Current checked-in path

[`component/src/lib.rs`](../component/src/lib.rs) owns an `AnnouncementSession` in `Runtime`. When announcements are enabled, the component initializes the Board Manager WebSocket path; bounded text callbacks for the active connection are passed to the session. The session parses and reduces local collection events, then the component adapter submits eligible utterances through the separately configured, room-bound speech stream and polls request outcomes. Announcement and speech-stream controls default disabled. The pinned [Niles integration WIT](../component/wit/niles-integration.wit) defines the WebSocket and speech-stream imports/callbacks; [`PROVENANCE.md`](../component/wit/PROVENANCE.md) records the contract source and its research state.

Configured room/audience values express intent. Niles retains household validation and speech transport authority. Speech submission acceptance means a bounded request was recorded; polling reports transport outcomes. Neither proves that audio played audibly or that a person heard it. The source implementation and mock-import tests do not prove a released compatible host, live Board Manager ordering/miss encoding, or end-to-end audible behavior.

## Historical host-contract research

These findings refer to the specifically inspected older Niles source snapshot at `/home/jleth/projects/niles`, commit `eea62f46f275b31da8517d7f895fe4d0178f543d`, with a then-current upstream WIT inspection at `2f4d54b739e001d2e4e1fe4e173bf4400dae476a`. They explain the earlier absence conclusion; they are not current claims about the pinned 0.5.0 contract or a released/installed host.

At that time, `notify.say` was a notification route rather than the requested high-frequency announcement contract, and host-internal code performed Piper synthesis, room resolution, and satellite delivery. Those private functions were not an add-on interface. The old plugin import list lacked the now-pinned WebSocket and speech-stream interfaces. Historical sources included `crates/niles-bin/src/main.rs`, `speak.rs`, `push.rs`, `crates/niles-wyoming/src/server.rs`, and `satellites.rs` in the inspected checkout. Their presence established Niles host behavior, not add-on access to it.

The historical upstream review-ready PR references recorded in [`component/wit/PROVENANCE.md`](../component/wit/PROVENANCE.md) do not establish that the contract was merged, released, installed, or supported as a minimum Niles version. This page does not check current upstream status.

## Policy and proof boundary

Do not substitute notification delivery for announcements, call private host functions, duplicate Wyoming transport, write to satellites directly, or add a sidecar. Keep local collection totals separate from authoritative X01 scores and game outcomes. Synthetic recording adapters and mock imports exercise request/policy behavior only. Optional compiled-component host proof and synthetic local HTTP/status checks have narrower limits documented in the [canonical evidence ledger](plan.md#verification-and-proof-boundaries). Real Board Manager event order and miss encoding remain open; audible validation requires separate authorization.
