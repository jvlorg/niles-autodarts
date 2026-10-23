# Announcement-path feasibility

Status: the checked-in component has a room-bound Niles speech-stream path and explicit host-outcome bookkeeping. It is an implementation candidate, not an installed or audible proof. Niles retains household and speech authority.

## Current path

The component opens Board Manager `/api/events` when announcement intent is enabled. `speech_stream_enabled` separately gates a room-bound stream for the configured room. `AnnouncementSession` parses bounded text frames and emits numeric request intent. The component submits text to Niles, records host request IDs, polls status, and closes the stream on stop or reset. HTTP `/api/state` remains status/baseline-only.

The component distinguishes accepted submission from delivery. Pending requests remain visible across polling failures; terminal outcomes are retained individually; no utterance is retried automatically. Reconnect, disablement, settings changes, overflow, and malformed/binary/oversized ingress invalidate the speech generation and locally discard unresolved requests. Old connection IDs are ignored. Overflow does not imply a new WebSocket connection: speech reopens without waiting for a nonexistent Connected callback. Because released host close is asynchronous, an unavailable reopen can resume on subsequent event/timer callbacks without replaying attempted commentary.

## Modes and scheduler constraint

`Each dart → Total` submits each eligible dart and then the total. `Total only` submits only the total at the first explicit partial/full removal marker. These are detection-derived local values, not game-authoritative scores. The released Niles scheduler permits one replaceable pending request; therefore `Accepted` does not promise that every rapid utterance reaches a satellite. This is a host scheduling contract, not a reason to invent an add-on retry queue.

## Proof and unresolved work

Fake-import callback tests cover lifecycle, request IDs, polling recovery, every released terminal outcome, actual history rollover, pending-bookkeeping rejection, byte-size boundaries, and overflow recovery without reconnect. The compiled proof uses `niles_plugins::Runtime` plus the pinned release's real `SpeechSessionManager` and semaphore-gated recording output. The former OpenSSL blocker came from the proof's direct `ort` dependency enabling native-TLS defaults that the pinned host disables; matching its feature configuration removes that accidental dependency.

The exact unchanged candidate only permits WebSocket ports 80/3180/443, so compiled scenarios require free loopback port 3180. The original ephemeral-port acceptance criterion is not satisfied; broadening production permissions to make a test pass was rejected. Active-transport preservation is tested separately through the same real manager's public terminal receivers, not presented as compiled-component evidence: the released adapter withholds later outcomes behind the active pending request. The released WIT close import is void and polling is session-wide, so close-error/per-request-poll-error injection is not an available contract.

Synthetic eligibility and recording output do not establish installed-host compatibility, live event ordering/miss encoding, household routing or human audibility. No host-private Rust function is used as an add-on API. No live device writes, persistent installation, deployment or release publication occur in these fixtures.

## Deep-review verification

- Before the source fix, three new callback regressions failed: overflow recovery without reconnect, pending-speech invalidation on untrusted frames, and collection of completed results before the slow timer. All pass after the fixes.
- `bash scripts/verify.sh --quick`: passed formatting, workspace build, strict Clippy, 25 core tests, 23 component tests, release Wasm build, both-mode replay, 8 probe tests and guidance equality.
- `bash scripts/prepare-host.sh`: verified host `4f3c0a10b2accd022354a23b9f961b222c27eb56` and built its CLI in the shared target directory.
- `cargo +1.99.0 clippy --locked --manifest-path component/host-proof/Cargo.toml --all-targets -- -D warnings`: passed after disabling the accidental `ort` defaults.
- `NILES_COMPONENT_WASM="$PWD/verification/review-smoke/integration.wasm" cargo +1.99.0 test --locked --manifest-path component/host-proof/Cargo.toml -- --nocapture`: passed the compiled synthetic scenarios and separately labeled manager scenario. These initial smoke assets came from the working build, not a clean release candidate.
- `python3 scripts/verify-host.py /home/harness/projects/.cargo-target/niles-autodarts/debug/niles --package-dir verification/review-smoke`: exact-file checker and synthetic HTTP/render diagnostic passed. Diagnostic state is temporary; existing developer settings/state are refused rather than reused.
- `python3 scripts/test_release.py`: 8 offline packaging safety regressions passed. Their fake compiler/checker do not establish real host behavior. Separate real pinned checker runs rejected corrupted Wasm and malformed declared SVG fixtures.
- `bash -n` on each changed shell script and actionlint 1.7.7 on the workflow passed. A throwaway harness executed the workflow's actual tag, artifact-integrity, moved-tag and approval-flag guards: valid assets accepted; invalid tags, missing/duplicate checksums, corrupted assets, wrong source/host, extra files and moved tags rejected; manual `publish=false` remained nonpublishing. No GitHub approval/publication run was performed.

The recorded compiled burst accepted host IDs `1/2/3/4`, texts `20/5/1/26`, logical keys `epoch:1:collection:2:dart:1`, `:dart:2`, `:dart:3`, `:total:3`. Outcomes were `Replaced/Replaced/Replaced/Delivered`; recorded transport contained only `26`. Ordered input delivered `20/5/1/26`, with a subsequent collection's `20` proving repeated markers did not repeat a total. TotalOnly delivered `26/20` across clear-before-marker and repeated-removal scenarios. Closing during preparation produced actual host `Cancelled`; reconnect and room switch each suppressed nonempty startup collections before delivering a fresh-baseline `20`. The manager-only active-output scenario delivered `20/26` and replaced pending `5`.
