# JVL-15 implementation

[[TASK 1/3]] Scope

- Causal boundary: `DEFAULT_TIMEOUT_MS` remains 2,000 ms; pinned WIT `http.send` waits for a complete response and caps `timeout-ms` by remaining callback time. `dispatch_detection` now distinguishes 2xx acknowledgement, response-timeout/oversized-response uncertainty, and definite HTTP/connection rejection.
- The historical approximately two-second `command_timeout` and subsequent Niles/provider HTTP 429 `token_quota_exceeded` remain separate failures. The historical `Starting` → `Started` timestamps are receive-only user evidence, not timestamps in the reduced fixture.

[[TASK 2/3]] Implementation

- `component/src/lib.rs` owns one monotonic generation-scoped pending operation, one-second bounded readback callbacks, an independent 30-second monitoring expiry, lifecycle/token filtering, duplicate suppression, opposite-command supersession, late completion, readback retry after unavailable state, WebSocket non-definitive observations, and diagnostics.
- Accepted or dispatch-uncertain startup returns supported `Ok(())` without publishing requested state. Last validated state remains authoritative until a fresh definitive non-`Starting` readback confirms it. Definite provider/local failures remain errors.
- `component/src/tests.rs` exercises actual `set-device`, timers, fake HTTP, reports, the fixture-labelled 15.575831-second event schedule, response timeout, duplicates, expiry, late completion, contradictory `Starting`, unavailable-readback recovery, rejection, unreachable board, and timer registration failure; core detection also rejects `Starting` with `running:true` as confirmation.
- `component/tests/host_proof.rs` extends the existing pinned-runtime proof with a loopback Board Manager that crosses the two-second response boundary, serializes command results and reported state, reaches later running observation, and returns a definite stop rejection.
- `crates/autodarts-core/tests/fixtures/board-manager-jvl15-startup-synthetic-scenarios.json` is synthetic timing only and remains separate from supplied captures.

[[TASK 3/3]] Verification

Executed:

- `bash scripts/prepare-host.sh` — passed; pinned host revision `4f3c0a10b2accd022354a23b9f961b222c27eb56`.
- `cargo test -p autodarts-core -p niles-autodarts-component` — passed: 50 core and 43 component tests.
- `cargo build --locked -p niles-autodarts-component --target wasm32-wasip2 --release` — passed.
- `cargo test --manifest-path component/host-proof/Cargo.toml --offline --no-run` — passed after the proof dependency/fixture changes.
- `bash scripts/verify.sh --quick` — passed; log `.harness/artifacts/jvl15-verify-quick.log`.
- `bash scripts/verify.sh` — passed with `exit_status=0`; log `.harness/artifacts/jvl15-verify-full.log`. This final run included the pinned host proof, compiled delayed startup result chronology, exact fixture probe, checksums, actionlint, and `cmp AGENTS.md CLAUDE.md`.
- Parsed supplied-fixture equality — passed; log `.harness/artifacts/jvl15-fixture-equality.log`.
- No live device write, installation, release, push, LLM call, or live voice/audible test was performed.

Host limitation: pinned `set-device` remains synchronous `result<_, string>` with no structured progress or asynchronous terminal-result channel. Pending progress is therefore diagnostic/state evidence; expiry is not rewritten as a retroactive command result.
