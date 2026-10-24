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

## Clean-source candidate verification

Source commit `7b34e16b8049cbdb41943c6ee7fadf718370349e` passed the complete `bash scripts/verify.sh` chain (exit 0) with actionlint 1.7.7 on `PATH` and `CARGO_TARGET_DIR=/home/harness/projects/.cargo-target/niles-autodarts`. The source was tracked-clean; the previous generated candidate was preserved separately rather than overwritten. Packaging built and checked the exact staged Wasm, the HTTP/render diagnostic and compiled scheduler proof consumed that candidate, and `sha256sum --check SHA256SUMS` passed. The compiler was `rustc 1.99.0 (b940084d7 2026-09-28)`; the CLI and runtime used pinned host `4f3c0a10b2accd022354a23b9f961b222c27eb56`.

| Candidate asset | SHA-256 |
|---|---|
| `integration.wasm` | `6439a8cf58d04e7ecbb20556171f86356a1cbb985f165b054575b23958d50fe5` |
| `niles-integration.toml` | `b1c1e4335175e5cfca14ddbd1d21a18c85c16736853f65dc0ff1634f6db550a6` |
| `provenance.json` | `41992fea86017570031bee0320709d95bede9768799ba3be8e8cb9ea65e83a58` |

The provenance records that source commit, not the later evidence-only documentation commit. It records the actual temporary staging path passed to `plugin check`; assets are atomically moved into `dist/v0.1.0-alpha.1` after successful validation without rebuilding.

Fresh-checkout proof also passed (exit 0): `git clone --no-hardlinks --single-branch <local-repository> /tmp/autodarts-fresh-review-4x7rab2f/source`, then `bash scripts/verify.sh` from that checkout with the same tool paths and shared Cargo build cache. No `.cache/niles`, target directory or candidate was copied. The preparation script fetched the canonical remote at the full pinned SHA and rebuilt the host from that fresh checkout; runtime proof and CLI checking used that same tree. Both manifest and Wasm hashes exactly match the table above. Fresh-run provenance hash was `f7463feb86e7929f722e591f403f9265289558c1e0f523d0018de9213654d213`; it differs because the actual checker staging path differs. The temporary checkout was removed after recording these results.

Release remains blocked on administrator setup: the read-only GitHub environments query returned `total_count: 0`. Configure `alpha-release` with required reviewers before setting `ALPHA_RELEASE_APPROVAL_CONFIGURED=true`. Local guards and successful synthetic checks are not protected-environment approval, installation, household routing or audible evidence. Earlier command transcripts are incomplete; that limits historical command auditing and is not evidence of skipped hooks or migrations.

## Alpha.2 final local verification

The alpha.2 implementation commit `7d7588f6d5d0b93740debf1000932b94bfecfacb` passed the complete `bash scripts/verify.sh` chain (exit 0), including 29 core tests, 25 component tests, the exact packaged Wasm/manifest check, the real pinned-host HTTP/render diagnostic, the compiled scheduler proof, probe tests, checksums, actionlint, and `cmp AGENTS.md CLAUDE.md`. The pinned host revision was `4f3c0a10b2accd022354a23b9f961b222c27eb56`; the proof used fixed loopback port 3180.

The generated `dist/v0.1.0-alpha.2` candidate from that run recorded these SHA-256 values: `integration.wasm` `c57ad8ed4e32d826c5126dadbfd8147c7c590359e25c92e66ce53b3d0c4f1b7e`, `niles-integration.toml` `8e25ae3fc2377aa54ab1ba13cf93ca8ce98befce26548650e8d48b8ec152e91a`, `provenance.json` `f898c43cc30c32b5546c829e43c3649be06fe0e0792ae4e37bb8512ba5ff4dda`, and `SHA256SUMS` `79a81595a1d92e56698336aa7bf00288357918d89d5bbcf0d7f7defc8c1b5686`. Any source change after that run, including later refactors, requires rebuilding the candidate before treating those hashes as release assets.
