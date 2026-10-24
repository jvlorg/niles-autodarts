# Announcement-path feasibility

Status: the checked-in component has Board Manager WebSocket ingress, a room-bound Niles speech-stream path, one-shot total deadlines, and correlated host-outcome bookkeeping. It is a local implementation candidate, not installed or audible proof. Niles retains household and speech authority.

## JVL-14 capture and synthetic provenance

`crates/autodarts-core/tests/fixtures/board-manager-jvl14-three-round-reduced-capture.json` is a deterministic 40-envelope fixture projection; the raw capture source was unavailable in this worktree. It is replayed without a prepended baseline; its running empty `Started` state is the usable baseline. The documented round shape is `[60,20,7]`, `[40,20,0]`, and `[12,18,0]`, producing totals 87, 60, and 30. The outside misses are accepted only when their number, `M<number>` name, `Outside` bed, and multiplier zero agree. This projection is not live-provider evidence.

`board-manager-jvl14-synthetic-scenarios.json` is explicitly synthetic. It contains partial-round takeout, timer-first, takeout-first, and detected-zero schedules. It is not a reconstruction of provider timing or live device behavior. The older JVL-13 fixtures remain separate maintained evidence.

## Core policy

`AnnouncementSession` parses bounded JSON and owns `LocalJourney`. A clean count-zero state establishes the baseline. Each accepted append is validated as a prefix extension of the previous snapshot. Each dart mode records each accepted score; Total only records the completed sum.

The third accepted dart creates one `RoundToken` and exposes it through `pending_total_deadline()`. Core never reads a clock. The component registers a one-second `after` timer and later supplies the token through `on_total_deadline()`. Exact motion takeout, state events `Takeout started` / `Takeout finished`, or status `Takeout in progress` can complete earlier, after validating the entire snapshot and collection transition. The third-dart `status: Takeout` alone does not. Deadline expiry and takeout consume the same eligibility and total attempt. A clear-bearing finish preserves the next round's baseline; duplicate motion flags remain latched until explicit neutral motion, independently of deadline completion. One- and two-dart rounds require explicit takeout; empty snapshots do not manufacture darts or totals.

A clear or shrink retains an eligible cached collection until its boundary. Corrections, malformed frames, stopped input, reconnect, disablement, reconfiguration, queue overflow, and stale generations invalidate it. A completed round's request text and key are immutable when later state arrives.

## Component speech lifecycle

A core delivery call admits an immutable intent into a bounded FIFO of 32 total local intents, including the in-flight request. The component submits only when no request is in flight. The next intent waits for the exact host request ID's terminal outcome. `pending` and omitted status do not unlock it. Poll errors preserve the in-flight item and queue. Every terminal outcome—delivered, busy, quiet, expired, failed, cancelled, replaced, rate-limited, or no-satellite—retires one request and pumps the next. Submission failure is recorded and is not retried.

A dedicated one-second speech-progress timer retries status collection while one request is in flight. It is progress polling, not a speech-duration estimate. Periodic status polling may collect the same outcomes through the idempotent collector. Late timer IDs are ignored unless their complete generation or round ID matches current state. Cancelling a timer is not the safety mechanism; token equality and consumed state are.

Stale callbacks cannot consume the current timer registration, and generation identity survives component stop/restart. Queue overflow and either timer-registration failure invalidate pending/queued speech and core eligibility. Diagnostics distinguish queued intents from host-pending requests. Invalidating ingress is processed before collecting terminal outcomes, so an already-completed request cannot cause one last stale queued submission.

Ordinary clear and takeout do not close the speech stream or discard the FIFO. Observed stopped/disconnected WebSocket or HTTP state, acknowledged detection-stop control, disablement, settings changes, parse errors, overflow, and reconnect close pending host work, discard queued stale intents, advance the speech generation, and require a fresh baseline. An acknowledged stop cancels commentary even if its independent state reread still reports running; failed commands alone do not establish stop. The pinned close contract cannot promise reversal of external transport already started; no stronger audible-stop claim is made.

`Ok(())` at the core adapter means adapter acceptance. Host submission, terminal transport result, and human audibility are separate observations.

## Verification

Run the local replay with:

```text
cargo run -p autodarts-core --example announcement_replay
cargo test --workspace
```

The scoped maintained chain is `bash scripts/verify.sh --quick`. Final certification is `bash scripts/verify.sh`; it prepares the pinned host, compares the exact WIT, builds and checks the component, runs the compiled proof, packages the exact candidate, verifies checksums, and runs actionlint. The compiled proof requires free loopback port 3180 and uses the pinned host checkout; it does not use host-private APIs.

The compiled proof records scheduler and recording-output observations for ordered speech and both modes. Recording output is not audible acceptance. The proof does not establish installed-host compatibility, general live Board Manager ordering, live miss encoding beyond the supplied narrow shape, household routing, selected-satellite behavior, or human audibility. No live writes, installation, deployment, release publication, or push occur in this implementation stage.

The maintained compiled scenarios exercise the real one-second deadline without takeout, final-dart preparation and active-transport gates, rapid subsequent rounds, and observed-stop/fresh-baseline restart in both modes. The recording future can be cancelled after transport entry; that is not proof of stopping external audible effects. The older JVL-13 capture and its separately scheduled synthetic continuation remain covered. Exact equality of the JVL-14 projection to the unavailable original capture remains unverified.
