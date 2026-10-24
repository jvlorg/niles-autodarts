# JVL-15 implementation

## Scope and causal boundary

- `DEFAULT_TIMEOUT_MS` remains 2,000 ms; pinned ordinary callbacks allow 10 seconds and `http.send` caps the whole-response wait by the remaining callback budget. `dispatch_detection` distinguishes acknowledgement, timeout/oversized-response uncertainty, HTTP rejection, and the host's ambiguous connection error. The historical approximately two-second `command_timeout` and later Niles/provider HTTP 429 `token_quota_exceeded` remain separate failures; source establishes the timeout mechanism, not which effective deadline caused the historical result.
- The historical `Starting` → `Started` timestamps are receive-only user evidence, not timestamps in the reduced fixture. The labelled 15.575831-second synthetic schedule remains local test data.

## Implementation

- `component/src/lib.rs` owns one generation-scoped pending operation, a monotonic deadline starting before dispatch, one-second bounded readback callbacks, lifecycle/token filtering, duplicate suppression, opposite-command supersession, late completion, readback recovery, validated WebSocket confirmation, and operation/elapsed-time diagnostics. Expired operations require a new definitive observation before a new explicit attempt.
- Accepted or dispatch-uncertain startup returns supported `Ok(())` without publishing requested state. Last validated state remains authoritative until a fresh definitive non-`Starting` readback confirms it. Complete HTTP rejection remains an error. The pinned `command_connection` result is reported as unknown outcome, not rejection, and is not retried.
- `component/src/tests.rs` exercises actual `set-device`, timers, fake HTTP, reports, the fixture-labelled 15.575831-second event schedule, response timeout, duplicates, expiry, late completion, contradictory `Starting`, unavailable-readback recovery, complete rejection, ambiguous start/stop connection errors, and timer registration failure. Core detection separately proves connection ambiguity versus HTTP rejection.
- `component/tests/host_proof.rs` uses the pinned runtime, loopback Board Manager, and real Niles `SetDevice` serializer. It covers acknowledged and delayed startup, false observation until later running, duplicate suppression, stop rejection, an incomplete response, and a synthetic connection closed before request dispatch. The fixture knows which transport path it induced; the add-on classification does not infer from that fixture knowledge.
- `crates/autodarts-core/tests/fixtures/board-manager-jvl15-startup-synthetic-scenarios.json` is synthetic timing only and remains separate from supplied captures.

## Verification

Revision-scoped quick verification passed after the code and test changes:

- `bash scripts/verify.sh --quick` — passed: 50 core tests, 55 component tests, workspace build/clippy, Wasm build, announcement replay, synthetic probe, release checks, and `cmp AGENTS.md CLAUDE.md`.
- An earlier dirty-tree `bash scripts/verify.sh` attempt passed through pinned-host clippy and stopped at packaging because the source tree was intentionally modified. That run is recorded as failed and is not used as evidence of final verification.
- Clean maintained `bash scripts/verify.sh` — passed with exit status 0: the pinned host prepared at `4f3c0a10b2accd022354a23b9f961b222c27eb56`; package/host checks, exact WIT comparison, release checks, checksums, actionlint, and loopback compiled host proof passed. Workspace tests passed with 50 core and 55 component tests; the compiled host proof passed its single maintained test, including delayed startup, duplicate suppression, rejection, incomplete-response and pre-dispatch closed-connection fixtures.

No live device write, installation, release publication, push, LLM call, or live voice/audible test was performed.


## Host-contract limitation

Pinned `set-device` remains synchronous `result<_, string>` with no structured progress, returned observation, or asynchronous terminal-result channel. A6 remains a separate host decision: the pinned WIT's `connection(string)` conflates pre-dispatch connection failure and post-dispatch incomplete response. This revision reports that result as unknown outcome without parsing human text, treating every connection failure as pending, or claiming transport classification. A generic host error/apology can remain even when a later observation shows the board running. Local passing tests do not remove that host limitation.
