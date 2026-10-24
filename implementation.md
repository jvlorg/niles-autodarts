# JVL-15 implementation

## Scope and causal boundary

- `DEFAULT_TIMEOUT_MS` remains 2,000 ms; pinned ordinary callbacks allow 10 seconds and `http.send` caps the whole-response wait by the remaining callback budget. `dispatch_detection` distinguishes acknowledgement, timeout/oversized-response uncertainty, HTTP rejection, and the host's ambiguous connection error. The historical approximately two-second `command_timeout` and later Niles/provider HTTP 429 `token_quota_exceeded` remain separate failures; source establishes the timeout mechanism, not which effective deadline caused the historical result.
- The historical `Starting` → `Started` timestamps are receive-only user evidence, not timestamps in the reduced fixture. The labelled 15.575831-second synthetic schedule remains local test data.

## Implementation

- `component/src/lib.rs` owns one generation-scoped pending operation, a monotonic deadline starting before dispatch, one-second bounded readback callbacks, lifecycle/token filtering, duplicate suppression, opposite-command supersession, late completion, readback recovery, validated WebSocket confirmation, and operation/elapsed-time diagnostics. Expired operations require a new definitive observation before a new explicit attempt.
- Accepted or dispatch-uncertain startup returns supported `Ok(())` without publishing requested state. Last validated state remains authoritative until a fresh definitive non-`Starting` readback confirms it. Complete HTTP rejection remains an error. The pinned `command_connection` result is reported as unknown outcome, not rejection, and is not retried.
- `component/src/tests.rs` exercises actual `set-device`, timers, fake HTTP, reports, the fixture-labelled 15.575831-second event schedule, response timeout, duplicates, expiry, late completion, contradictory `Starting`, unavailable-readback recovery, complete rejection, ambiguous start/stop connection errors, and timer registration failure. Core detection separately proves connection ambiguity versus HTTP rejection.
- `component/tests/host_proof.rs` uses the pinned runtime, loopback Board Manager, and real Niles `SetDevice` serializer. It covers acknowledged and delayed startup, false observation until later running, duplicate suppression, stop rejection, an incomplete response, and a known closed endpoint. Startup completion is gated until pending-state and duplicate assertions finish. The unreachable case aborts and awaits the fixture listener and checks connection refusal before issuing the command. Fixture-side evidence is not an add-on classification heuristic.
- `crates/autodarts-core/tests/fixtures/board-manager-jvl15-startup-synthetic-scenarios.json` is synthetic timing only and remains separate from supplied captures.

## Verification

The preceding implementation revision recorded these results (50 core and 55 component tests):

- `bash scripts/verify.sh --quick` — passed: workspace tests/build/clippy, Wasm build, announcement replay, synthetic probe, and `cmp AGENTS.md CLAUDE.md`. Release checks belong only to the full maintained chain.
- An earlier dirty-tree `bash scripts/verify.sh` attempt passed through pinned-host clippy and stopped at packaging because the source tree was intentionally modified. That run is recorded as failed and is not used as evidence of final verification.
- Clean maintained `bash scripts/verify.sh` — recorded as passing with exit status 0: the pinned host prepared at `4f3c0a10b2accd022354a23b9f961b222c27eb56`; package/host checks, exact WIT comparison, release checks, checksums, actionlint, and loopback compiled host proof passed. Its connection-close fixture accepted TCP before closing and did not establish a pre-dispatch failure; the deep review replaces that fixture with an actually closed listener.

No live device write, installation, release publication, LLM call, or live voice/audible test is part of this verification.

## JVL-16 implementation

### Changed

- `component/src/lib.rs`: commentary opening now requires a validated current `running` observation. Stopped, Starting, unavailable/disconnected, reconnect, overflow, stop, reconfiguration, and stale lifecycle paths clear authorization and reuse the existing invalidation/close helpers. Board observation remains connected; no automatic detection start was added. Rendered diagnostics now separate observation connection, detection state, speech session state, settings, and core baseline readiness.
- `component/src/tests.rs`: added `jvl16_` focused coverage for idempotent stopped-session closure, preserved observation connection/no wake command, Starting and stale-dart recovery, pending-total cancellation, old timer/callback suppression, and updated eager-open assumptions.
- `component/tests/host_proof.rs`: compiled proof now supplies a validated running observation without establishing a score baseline, so startup/lifecycle proof remains compatible with observation-gated session opening and stale darts remain silent.
- `docs/plan.md` and `README.md`: documented observation-gated session lifetime and offline evidence boundaries.

### Executed checks

- `cargo test --manifest-path component/Cargo.toml --lib jvl16_` — passed (3 tests).
- `cargo test --manifest-path component/Cargo.toml --lib stopped_` — passed (6 tests).
- `cargo test --manifest-path component/Cargo.toml --lib` — passed (59 tests).
- `cargo run -p autodarts-core --example announcement_replay` — passed.
- Parsed equality comparison for all preserved `.harness/artifacts/board-manager-*.json` fixtures — passed.
- `bash scripts/verify.sh --quick` — passed.
- `cargo fmt --all` — completed.

The pre-review candidate's maintained `bash scripts/verify.sh` passed after the host-proof lifecycle update. It prepared Niles `4f3c0a10b2accd022354a23b9f961b222c27eb56`, compared the exact WIT, passed package checks, exact-asset diagnostics, the loopback compiled host proof on port 3180, checksums, and actionlint. Full exact-head output is preserved in `.harness/artifacts/verify-full-exact-head.log`; exit status is `.harness/artifacts/verify-full-exact-head.exit`. This evidence predates the review corrections below and does not verify their changed Wasm.

### JVL-16 review corrections

- Fixed observation-connection classification: `reconnecting` previously matched `connecting`, and disconnect reason text containing `connected` could claim a connected observation stream.
- Preserved the host-open error when commentary is rejected without a speech session, rather than overwriting permission failures with generic unavailability.
- Both diagnostic regressions failed before the fixes. Added focused coverage for the rendered state dimensions, stop during asynchronous-close retry, current-running observations with disabled/invalid/host-denied settings, and idle HTTP stop on ordinary and accelerated readback. Existing tests now assert timer cancellation, failed-startup session closure, and safe recovery into the newly configured room.
- `cargo test --manifest-path component/Cargo.toml --lib` passed with 63 tests; `bash scripts/verify.sh --quick` passed. Logs are `.harness/artifacts/astra-regression-before.log`, `astra-regression-after.log`, and `astra-verify-quick.log`, with separate exit-status files. An earlier quick invocation was interrupted by the tool timeout and is not passing evidence.
- The workflow's final candidate stage must run the maintained full chain, including pinned-host lifecycle proof, against the new PR head. No installed UI, live-board, or audible acceptance was performed by this review.

### Remaining limitations

No real-board access, deployed Niles access, installation, service change, audible test, release, push, or PR publication was performed. `Accepted`/transport outcomes remain distinct from audible delivery. The pinned JVL-15 `command_connection` dispatch-evidence limitation remains unchanged.

## Deep-review corrections

- Accelerated detection readback previously published a stopped observation without cancelling queued announcements during a pending start. The new callback regression reproduced an extra stale submission before the fix and passes after both HTTP polling paths share the existing cancellation policy. WebSocket invalidation remains on its existing ingress path.
- Connection-error regressions now use distinct opaque transport messages, assert uncertainty versus rejection rather than complete prose, and exercise later callbacks to detect automatic redispatch. Complete HTTP rejection is covered for start and stop.
- The compiled proof now synchronizes startup completion and distinguishes recorded incomplete-response dispatch from a verified closed loopback listener. The host still exposes the same ambiguous error category.
- Review verification: the stopped-readback regression failed before the fix; `bash scripts/verify.sh --quick` then passed with 50 core tests, 56 component tests, and 8 synthetic probe tests. An intermediate quick run caught duplicate WebSocket invalidation; that failed run is not passing evidence. The final committed candidate's full-chain result and exact revision belong in PR #11.

## Host-contract limitation

Pinned `set-device` remains synchronous `result<_, string>` with no structured progress, returned observation, or asynchronous terminal-result channel. A6 remains a separate host decision: the pinned WIT's `connection(string)` conflates pre-dispatch connection failure and post-dispatch incomplete response. This revision reports that result as unknown outcome without parsing human text, treating every connection failure as pending, or claiming transport classification. A generic host error/apology can remain even when a later observation shows the board running. Local passing tests do not remove that host limitation.
