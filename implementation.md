# JVL-15 implementation

[[TASK 1/3]] Scope

- Causal boundary: `DEFAULT_TIMEOUT_MS` remains 2,000 ms; pinned ordinary callbacks allow 10 seconds and `http.send` caps the whole-response wait by the remaining callback budget. `dispatch_detection` distinguishes acknowledgement, timeout/oversized-response uncertainty, and HTTP rejection. The host's connection-error variant still conflates pre-dispatch and incomplete-response failures; see the mandatory blocker in `docs/core-communication.md`.
- The historical approximately two-second `command_timeout` and subsequent Niles/provider HTTP 429 `token_quota_exceeded` remain separate failures. The historical `Starting` → `Started` timestamps are receive-only user evidence, not timestamps in the reduced fixture.

[[TASK 2/3]] Implementation

- `component/src/lib.rs` owns one generation-scoped pending operation, a monotonic deadline starting before dispatch, one-second bounded readback callbacks, lifecycle/token filtering, duplicate suppression, opposite-command supersession, late completion, readback recovery, validated WebSocket confirmation, and operation/elapsed-time diagnostics. Expired operations require a new definitive observation before a new explicit attempt.
- Accepted or dispatch-uncertain startup returns supported `Ok(())` without publishing requested state. Last validated state remains authoritative until a fresh definitive non-`Starting` readback confirms it. Definite provider/local failures remain errors.
- `component/src/tests.rs` exercises actual `set-device`, timers, fake HTTP, reports, the fixture-labelled 15.575831-second event schedule, response timeout, duplicates, expiry, late completion, contradictory `Starting`, unavailable-readback recovery, rejection, unreachable board, and timer registration failure; core detection also rejects `Starting` with `running:true` as confirmation.
- `component/tests/host_proof.rs` extends the existing pinned-runtime proof with a loopback Board Manager and the real Niles `SetDevice` tool serializer. Both immediate acknowledgement and delayed response timeout retain false observation until later running; repeated commands issue one PUT, and stop rejection remains an error.
- `crates/autodarts-core/tests/fixtures/board-manager-jvl15-startup-synthetic-scenarios.json` is synthetic timing only and remains separate from supplied captures.

[[TASK 3/3]] Verification

Original implementation verification (before the Astra review changes):

- `bash scripts/prepare-host.sh` — passed; pinned host revision `4f3c0a10b2accd022354a23b9f961b222c27eb56`.
- `cargo test -p autodarts-core -p niles-autodarts-component` — passed: 50 core and 43 component tests.
- `cargo build --locked -p niles-autodarts-component --target wasm32-wasip2 --release` — passed.
- `cargo test --manifest-path component/host-proof/Cargo.toml --offline --no-run` — passed after the proof dependency/fixture changes.
- `bash scripts/verify.sh --quick` — passed; log `.harness/artifacts/jvl15-verify-quick.log`.
- `bash scripts/verify.sh` — passed with `exit_status=0`; log `.harness/artifacts/jvl15-verify-full.log`. This final run included the pinned host proof, compiled delayed startup result chronology, exact fixture probe, checksums, actionlint, and `cmp AGENTS.md CLAUDE.md`.
- Parsed supplied-fixture equality — passed; log `.harness/artifacts/jvl15-fixture-equality.log`.
- No live device write, installation, release, push, LLM call, or live voice/audible test was performed.

Host limitations: pinned `set-device` remains synchronous `result<_, string>` with no structured progress, returned observation, or asynchronous terminal-result channel. Mandatory A6 is still blocked by the host's ambiguous `connection(string)` error; the add-on cannot distinguish unreachable boards from post-dispatch incomplete responses without a core contract decision. Local passing tests do not waive that blocker.
