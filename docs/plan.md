# Niles Autodarts — canonical plan

## GOAL STATUS

Implemented: validated announcement settings and a reusable session/reducer; component `Runtime` owns the session plus WebSocket and speech-stream state. The component uses the pinned Niles integration 0.5.0 WebSocket and speech-stream imports, with lifecycle and WebSocket callbacks exported in WIT. See [`component/src/lib.rs`](../component/src/lib.rs), [`component/wit/niles-integration.wit`](../component/wit/niles-integration.wit), and [`component/wit/PROVENANCE.md`](../component/wit/PROVENANCE.md) for implementation and contract provenance.

Missing evidence: compatibility with a released/installed Niles host, real Board Manager event ordering and miss encoding, and end-to-end audible behavior. The pinned contract's upstream history does not establish a released minimum host version. Local totals remain detection-derived collection sums, not game-credited X01 authority.

## V1 — local Board Manager score announcements and detection control

V1 uses only the locally configured Autodarts Board Manager. No cloud account, match feed, X01 authority, game selection, match outcome or special/game speech is required. Local dart products and collection sums are not credited X01 scores, remaining scores, busts or checkout results.

The event parser accepts `{type,data}` envelopes and state fields `numThrows`, optional `throws[]`, and `segment.number`/`segment.multiplier`. It bounds envelope bytes and dart array length, rejects non-object states, invalid range/multipliers and count mismatches. Bull 25 and 50 are valid. Multiplier-zero is rejected; maintained source evidence does not resolve whether misses are encoded as `(0,0)` or `(0,1)`, so no claim of validated miss support is made. A malformed JSON input is reported by `AnnouncementSession` to the reducer as invalid; callers cannot silently drop parse errors through the session API.

Reducer phases are awaiting a clean baseline, collecting, and awaiting clear. Only an empty, count-zero state establishes a usable baseline. A nonempty baseline suppresses that entire existing collection, including later appends. Unrelated camera/status topics are ignored. It accepts one append at a time, with at most three darts. Each dart is announced as local API reports it; TotalOnly remains silent until the total boundary. The first explicit `isTakeoutPartial=true` or `isTakeoutFull=true` marker completes the eligible cached collection once; subsequent markers cannot repeat it. These are local detection values, not authoritative game scores.

For marker-before-clear, the validated cached collection is spoken once. In TotalOnly, a clear-before-marker preserves its eligible cached collection until the explicit removal marker; clear itself never announces. For Each dart → Total, if a shrink/empty state precedes the marker, retain only the last validated pre-removal snapshot as a bounded candidate; shrink alone never announces. The first subsequent explicit removal marker may complete that candidate. Corrections, invalid/out-of-order evidence, malformed/stopped input and reconnect/disable/settings changes invalidate it; ambiguity suppresses. After completion, require a clean empty baseline before another collection. No reliable timestamps are supplied by the source, so no time-based expiry is claimed.

`AnnouncementSession` owns validated settings, parsing and reducer state; callers provide each JSON envelope and an `AnnouncementDelivery`. Request keys include connection epoch and collection serial. Each delivery attempt is recorded before invoking the injectable boundary, so unavailable/failed attempts are not blindly retried. Target identities are sorted and deduplicated. Disabling, changing mode/audience, or reconnecting resets policy and requires a clean empty baseline. Parse failures automatically invalidate the current journey.

A slow `/api/state` timer poll remains status/baseline-only, not a real-time event stream. When announcements are enabled, component initialization opens the WebSocket path; a separately configured speech-stream control can open a room-bound Niles speech session. Both announcement and speech-stream controls default disabled. The component passes bounded callback JSON to its owned session, ignores callbacks for a different connection identity, and resets the journey on connection-state changes. Stop and reinitialization close active WebSocket and speech sessions; settings replacement follows the same reset-and-clean-baseline policy. These lifecycle paths are implemented and have mock-import coverage, not installed-host compatibility proof.

The speech adapter submits utterances and polls request outcomes through Niles. Submission acceptance records a bounded request; polled outcomes describe transport status and do not prove audible playback or exact-once physical sound. Unavailable or failed delivery attempts are recorded by the session and are not blindly retried. The session's explicit unavailable and recording adapters remain available for library/test behavior; the component's optional speech-stream adapter is implemented, while installed and live delivery remain unproven.

V1 product scope remains Board Manager detection on/off (not physical power/reset/game creation), room selection and local announcements through selected room satellites in both Each dart → Total and Total only modes; these are not all implemented end-to-end. Announcement settings default disabled; identities are configuration intent, not validated household targets. Preserve V2 TV scoreboard and V3 game selection as later scope; do not collapse those roadmap boundaries into V1.

## Verification and proof boundaries

The evidence below separates source implementation from synthetic exercises, optional host checks, installation, live board behavior, and human audibility. This documentation reconciliation adds no runtime or live evidence.

| Evidence/source | Permitted conclusion | Limit |
| --- | --- | --- |
| [`component/src/lib.rs`](../component/src/lib.rs), [pinned WIT](../component/wit/niles-integration.wit), [`component/niles-integration.toml`](../component/niles-integration.toml), [`manifest.toml`](../manifest.toml) | Source implements component session ownership, WebSocket and speech-stream lifecycle/callbacks, permissions, and settings | Does not establish installed-host compatibility or live operation |
| [`crates/autodarts-core/src/local_score.rs`](../crates/autodarts-core/src/local_score.rs), [`announcement_session.rs`](../crates/autodarts-core/src/announcement_session.rs), [`announcement_replay` example](../crates/autodarts-core/examples/announcement_replay.rs) | Maintained synthetic parser/session/reducer coverage and replay through a recording adapter | Replay bypasses component/host transport; no board or audio proof |
| [`component/src/tests.rs`](../component/src/tests.rs) | Mock-import coverage for room/config lifecycle, callback scoring, baseline/disconnect handling, malformed messages, and stop/reconfiguration cleanup | Mock imports are not an installed Niles host |
| [`component/tests/host_proof.rs`](../component/tests/host_proof.rs), [`component/host-proof/Cargo.toml`](../component/host-proof/Cargo.toml) | An optional compiled-component test is present for a local Niles runtime, synthetic loopback events, and recording speech | Requires developer-local absolute source paths; test presence is not an execution result and proves neither live events nor audibility |
| [`scripts/verify-host.py`](../scripts/verify-host.py) | Synthetic local HTTP and `plugin dev --dry-run --allow-loopback` status-path assertions | Version/state GET and rendered status only; not WebSocket or speech proof |
| [`component/wit/PROVENANCE.md`](../component/wit/PROVENANCE.md) | Records the pinned contract's origin in a developer-local Niles checkout and upstream research state | Not a supported released minimum version, deployment record, or current upstream-status check |

Maintained core tests cover repeated identical collections, incremental speech, both modes, first partial/full removal exactly once, duplicate markers, both clear/marker orders including TotalOnly clear-before-marker, correction/no-marker suppression, resets, bounded unique identities, parse-error invalidation, unavailable no-retry, and synthetic miss/bull cases. Those miss cases do not settle live miss encoding. The replay runs settings → JSON → session/reducer → recording adapter and does not exercise component or host transport. Historical command results, when quoted elsewhere, retain their original context and are not results from this documentation task.

### Remaining work

- Validate the pinned component against a compatible installed Niles host and establish whether a released host supports this contract; provenance alone does not answer this.
- Reproduce the optional compiled-component host proof where its developer-local `/home/jleth/projects/niles-nil43/...` source dependencies exist. The checked-in test and verifier are not proof that it has run in a fresh checkout.
- Observe real Board Manager event ordering and miss encoding, including whether misses use `(0,0)` or `(0,1)`.
- Perform separately authorized end-to-end audible validation if needed; transport acceptance and outcomes do not establish human audibility.
- Complete V1 room targeting and announcements through selected room satellites in both modes as validated product behavior; preserve V2 TV scoreboard and V3 game selection as later roadmap scope. Detection on/off remains distinct from physical power/reset and game creation.

Live/device operations and audible tests require separate authorization. Documentation claims alone do not establish runtime behavior, installation, release, or live proof.
