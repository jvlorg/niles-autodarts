# Niles Autodarts — canonical plan

## Goal status

The local candidate implements bounded Board Manager JSON ingress, strict dart parsing, clean-baseline gating, one/two/three-dart local collection policy, explicit takeout handling, and exactly-once total selection. A third validated dart arms one logical one-second deadline; explicit takeout may complete the same round earlier. Deadline expiry is supplied by the component and never uses a clock in core.

The component uses `/api/events` for scoring ingress and an optional room-bound Niles speech stream. Speech intents are admitted into a bounded FIFO with one host request in flight. The next intent waits for the exact correlated terminal host outcome, including failure, cancellation, replacement, rate limiting, and no-satellite outcomes. Pending and polling errors retain the in-flight request. Stop, disablement, reconfiguration, reconnect, parse errors, overflow, and observed stopped input invalidate stale generations; ordinary takeout and round clear do not.

This is a local implementation candidate. Installed-host compatibility, general live Board Manager event ordering, audible delivery, household routing, and selected-satellite behavior remain unproven. The compiled proof requires the pinned host and free loopback port 3180. Local sums are detection-derived values, not X01 authority.

## V1 — local Board Manager score announcements and detection control

V1 uses only the configured local Board Manager. No cloud account, match feed, X01 authority, game selection, match outcome, bust, checkout, or special/game speech is claimed. Detection on/off is distinct from physical power, reset, and game creation.

The parser accepts bounded `{type,data}` envelopes, at most three darts, strict integer counts, valid segment ranges, and bull values. Multiplier zero is accepted only for the evidenced canonical outside miss shape: `number` 1 through 20, `bed` exactly `Outside`, `name` exactly `M<number>`, and multiplier `0`; it occupies a detected dart slot and scores zero. Ordinary nonzero scoring inputs do not require name or bed fields. Malformed or contradictory miss fields are rejected. Omitted `throws` is canonical only for `numThrows == 0`.

The reducer requires a clean empty baseline and accepts one append at a time. Each dart mode emits each accepted score, including detected zeroes; Total only emits the completed total. A third validated append arms one deadline token once. Duplicate snapshots do not rearm it. Explicit motion (`isTakeoutPartial` or `isTakeoutFull`) and exact state events (`Takeout started` / `Takeout finished`) are boundary evidence; a third-dart `status: Takeout` alone is not. Takeout and matching deadline expiry share one exactly-once completion transition. One or two detected darts complete only on explicit takeout. Zero detected darts stay silent; a detected all-zero collection may announce total zero.

Clear or shrink alone never announces. A clear-before-marker sequence retains its eligible collection. Corrections, count jumps, malformed input, stopped input, reconnect, disablement, and settings changes invalidate eligibility and require a new baseline. A completed total retains immutable round content while later rounds enter the FIFO.

`AnnouncementSession` owns validated settings, parsing, reducer state, deadline tokens, and JSON ingress. `pending_total_deadline()` exposes the logical token. `on_total_deadline()` is a no-op for stale, cancelled, completed, invalidated, or reset tokens. Request keys include connection epoch and collection serial; every delivery attempt is recorded before the injectable boundary. Adapter `Ok(())` means adapter acceptance only, never host submission, playback, or audibility.

Household membership, room audience resolution, speech authority, arbitration, synthesis, and final transport remain in Niles. V2 TV scoreboard and V3 game selection remain out of scope.

## Evidence and proof boundaries

The supplied installed alpha.2 observation is historical user evidence for revision `82a30c4155e6ba20806a00734042a1f4ea49c1a7`: audible hit and takeout-total behavior were observed, and captured outside misses exposed the narrow grammar above. That evidence does not make this candidate installed, released, generally compatible, routed, or audibly verified.

`board-manager-jvl14-three-round-reduced-capture.json` is a deterministic 40-envelope fixture projection kept separate from `board-manager-jvl14-synthetic-scenarios.json`; the raw capture source was unavailable in this worktree. The fixture replay carries the documented three-round shape through settings, JSON ingress, reducer policy, and recording delivery. Synthetic scenarios cover partial rounds, deadline-first ordering, takeout-first ordering, and detected-zero behavior. Both modes produce totals 87, 60, and 30; Each dart → Total additionally preserves the nine detected dart values. This projection is not live-provider evidence.

| Evidence path | What it establishes | What it does not establish |
|---|---|---|
| Core parser/session and replay | Strict parsing, miss grammar, baseline policy, deadline token transitions, exactly-once totals, both modes, and deterministic adapter requests | Component callbacks, host compatibility, live provider behavior, audience routing, or audible output |
| Component fake-import callbacks | WebSocket generation filtering, timer identity checks, FIFO admission, one in-flight request, terminal outcome correlation, polling recovery, queue bounds, and lifecycle cancellation | Installed-host compatibility, live provider behavior, household routing, or audibility |
| Compiled pinned-host proof | Real pinned runtime, timer bridge, speech scheduler, slow-output ordering, and recording transport observations | Installation, live Board Manager ordering, general compatibility, household routing, or human audibility |
| Package and verification scripts | Exact manifest/Wasm candidate, pinned WIT comparison, package validation, and checksums | Live device writes, installation, deployment, release, or audible acceptance |

Source-implemented, synthetic-tested, host-tested, installed, and live-proven behavior must remain distinct. No live writes, installation, deployment, release publication, household capture, or audible acceptance is part of this local implementation.

## Validation

`bash scripts/verify.sh --quick` is the scoped chain. `bash scripts/verify.sh` is the maintained final chain and includes pinned-host preparation, exact WIT comparison, proof checks, packaging, exact-asset diagnostics, checksums, and actionlint. The host proof requires free loopback port 3180. Do not change pinned WIT, host revision, permissions, package version, or manifest URL policy to make a proof pass.
