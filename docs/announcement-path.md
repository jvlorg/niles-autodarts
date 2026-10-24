# Announcement-path feasibility

Status: the checked-in component has a room-bound Niles speech-stream path and explicit host-outcome bookkeeping. It is an implementation candidate, not an installed or audible proof. Niles retains household and speech authority.

## JVL-13 supplied reduced real capture and alpha.2 offline evidence

`board-manager-1.0.7-reduced-capture.json` is the supplied reduced real capture: a 14-envelope ordered projection of the Board Manager messages, with coordinate/camera/timing data removed and consecutive identical motion flags coalesced. It is not a reconstructed baseline or a synthetic continuation. The capture begins with the nonempty 3/7/7 collection and contains the real zero-count `Takeout finished` state with omitted `throws`; no empty state was added. The initial nonempty collection remains silent, and that captured zero-count event establishes the safe baseline.

`numThrows: 0` with omitted `throws` is normalized to the canonical empty state. Explicit empty arrays remain valid; missing/nonzero arrays, nulls, contradictory counts, malformed darts, and out-of-range values remain fail-closed. Typed diagnostics expose policy readiness separately from the last ignored, suppressed, invalid, or processed event. `board-manager-1.0.7-synthetic-continuation.json` remains separate and exercises three later identical 3/7/7 collections in both modes; it yields `3,7,7,17` per collection in Each dart → Total and `17` per collection in Total only.

Core replay, fake-WIT component ingress, and compiled pinned-host proof now consume the exact reduced real capture before the separate synthetic continuation. The real-capture phase asserts no initial nonempty replay, baseline recovery at the omitted-array event, and no speech submissions before continuation. This remains offline policy/submission evidence: it does not establish installation, live event ordering or miss encoding, household routing, host compatibility, or audible playback.

The current local candidate is `v0.1.0-alpha.2`. Full final-head verification results, suite counts, package hashes, and the exact verified commit are recorded below only after the final clean commit is verified. The compiled proof requires free loopback port 3180 and uses the pinned host; no live device writes, installation, deployment, release publication, or audible acceptance occurs here.

Existing synthetic regressions continue to cover duplicate/progressive shrink, ambiguous scoring, stopped-board pending speech, failed submission, reconnect and reconfiguration safety. Those edge cases remain in synthetic coverage and are not mixed into the supplied real capture.

## Current path

The component opens Board Manager `/api/events` when announcement intent is enabled. `speech_stream_enabled` separately gates a room-bound stream for the configured room. `AnnouncementSession` parses bounded text frames and emits numeric request intent. The component submits text to Niles, records host request IDs, polls status, and closes the stream on stop or reset. HTTP `/api/state` remains status/baseline-only.

The component distinguishes accepted submission from delivery. Pending requests remain visible across polling failures; terminal outcomes are retained individually; no utterance is retried automatically. Reconnect, disablement, settings changes, overflow, and malformed/binary/oversized ingress invalidate the speech generation and locally discard unresolved requests. Old connection IDs are ignored. Overflow does not imply a new WebSocket connection: speech reopens without waiting for a nonexistent Connected callback. Because released host close is asynchronous, an unavailable reopen can resume on subsequent event/timer callbacks without replaying attempted commentary.

## Modes and scheduler constraint

`Each dart → Total` submits each eligible dart and then the total. `Total only` submits only the total at the first explicit partial/full removal marker. These are detection-derived local values, not game-authoritative scores. The released Niles scheduler permits one replaceable pending request; therefore `Accepted` does not promise that every rapid utterance reaches a satellite. This is a host scheduling contract, not a reason to invent an add-on retry queue.

## Proof and unresolved work

Fake-import callback tests cover lifecycle, request IDs, polling recovery, every released terminal outcome, actual history rollover, pending-bookkeeping rejection, byte-size boundaries, and overflow recovery without reconnect. The compiled proof uses `niles_plugins::Runtime` plus the pinned release's real `SpeechSessionManager` and semaphore-gated recording output. The former OpenSSL blocker came from the proof's direct `ort` dependency enabling native-TLS defaults that the pinned host disables; matching its feature configuration removes that accidental dependency.

The exact unchanged candidate only permits WebSocket ports 80/3180/443, so compiled scenarios require free loopback port 3180. The original ephemeral-port acceptance criterion is not satisfied; broadening production permissions to make a test pass was rejected. Active-transport preservation is tested separately through the same real manager's public terminal receivers, not presented as compiled-component evidence: the released adapter withholds later outcomes behind the active pending request. The released WIT close import is void and polling is session-wide, so close-error/per-request-poll-error injection is not an available contract.

Synthetic eligibility and recording output do not establish installed-host compatibility, live event ordering/miss encoding, household routing or human audibility. No host-private Rust function is used as an add-on API. No live device writes, persistent installation, deployment or release publication occur in these fixtures.

## Final JVL-13 verification

The supplied reduced real capture is replayed as an ordered 14-envelope array through core JSON ingress, fake-WIT component callbacks, and the compiled pinned-host proof. It stays silent through the initial nonempty 3/7/7 collection; the captured zero-count state with omitted `throws` establishes `Collecting (0 darts)`. The separate synthetic continuation then produces three successive collections: `3,7,7,17` repeated three times in Each dart → Total and `17` repeated three times in Total only. No synthetic baseline is inserted into the real capture.

The complete `bash scripts/verify.sh` chain passed on source head `f19753643dd936376a6659ce24b9168ec2f2a60e` and was pushed to PR #9 before the documentation-only evidence update. It included:

- 35 core tests, 26 component tests, 1 compiled pinned-host proof test, 8 maintained probe tests, 9 release-safety tests, and 0 failing doc-tests.
- Formatting, workspace build, strict workspace Clippy, release Wasm build, maintained replay, pinned host preparation at `4f3c0a10b2accd022354a23b9f961b222c27eb56`, exact WIT comparison, host-proof formatting/strict Clippy, exact package verification, and actionlint.
- `sha256sum --check SHA256SUMS` passed for the packaged alpha.2 candidate.

The verified package recorded these SHA-256 values:

| Candidate asset | SHA-256 |
|---|---|
| `integration.wasm` | `f24227e62608da2ad7d41e2587ed96e756dbf564abf1a2630be32989e01b2b06` |
| `niles-integration.toml` | `8e25ae3fc2377aa54ab1ba13cf93ca8ce98befce26548650e8d48b8ec152e91a` |
| `provenance.json` | `9bcf94721e7dbe344e3efa4a816df29ac0e2c9aa83f34ae32a7d82bd26cc13ac` |
| `SHA256SUMS` | `a8c3171a815b126c812faa108a381976fdc9f57c75f1c27e04ff9ca8fdc0a669` |

The final documentation-only commit requires the same maintained chain to be rerun on the resulting PR head; that final head and its matching verification result belong in the PR description and acceptance report. This evidence remains offline policy, compiled ingress, and Niles scheduler proof. It does not establish installed-host compatibility, live event ordering or miss encoding, household routing, or audible playback. No live-device writes, installation, deployment, release publication, or audible retest occurred.
