# Niles Autodarts — canonical plan

## GOAL STATUS

Planning baseline recorded. Implementation has not started. Nothing is installed or live-proven.

This repository owns the standalone Autodarts add-on. Niles core remains responsible for household rooms, satellites and speech delivery; confirm its actual extension APIs before relying on them. Do not move the integration into Niles core or build a parallel speech service as an unapproved workaround.

## V1 — room-scoped score announcer and board on/off

### Product contract

1. Represent the dartboard as a Niles device that the user can assign to a room.
2. Support “Niles, turn on the dartboard” and the corresponding off command through normal Niles device control.
3. On/off means starting/stopping local Board Manager detection. It does not mean physical power control or selecting/starting an online game.
4. Show observed running state and availability. A command acknowledgement or timeout is not proof of successful startup.
5. Let the user select the announcement satellites in the board's room; do not automatically broadcast to every satellite or infer that every room satellite is selected.
6. Announce scores using Piper through the Niles-owned speech path to those selected satellites.
7. Provide an announcement enable/disable control.
8. Continue choosing and managing games through Autodarts itself for V1.

### Decisions still open

- Announce each dart, a completed turn total, or both? No default has been approved.
- Which feed supplies authoritative scores, corrections, busts and checkouts? Local throw detections are not automatically authoritative match results.
- What defines a confirmed score, and how should a subsequent correction be handled?
- Can the deployed Niles add-on contract request Piper speech to an explicit satellite audience and expose board control through normal voice/device tools?
- How should unavailable satellites, room reassignment, disabled announcements and overlapping speech be handled?
- Is more than one board required for V1? Do not bake a household-specific room or board identity into the implementation.

### Acceptance targets (not yet tested)

- Board registration and room assignment work through Niles, with stable identity across restart.
- Voice on/off resolves to the correct board; observed state confirms the result. Unreachable hardware is unavailable, not falsely off or successfully started.
- Satellite selection is persisted and scoped to the board's room; unselected satellites receive no announcements.
- Confirmed score evidence produces the agreed spoken content through the actual host/Piper/satellite path.
- Duplicate events and reconnect snapshots do not repeat announcements; stale queued scores are not spoken after recovery.
- Corrections and provisional detections follow the agreed announcement policy rather than guessing.
- Disabling announcements stops new score speech without implicitly stopping the board.
- Restart/disconnect recovery is covered by deterministic tests and later an explicitly approved live test.

## V2 — TV scoreboard

Goal: display the current scoreboard on the room's LG TV, using the LG TV add-on for power and app/page selection where supported.

Investigate supported TV launch targets, scoreboard rendering and transport, live updates, authentication and reconnect behavior. A web page is a candidate, not an approved implementation. Do not assume arbitrary casting, app launch or browser support. V1 must not depend on V2.

## V3 — game selection (tentative)

Explore selecting/starting games from Niles, including game variants, players, match options and confirmation semantics. Scope and API feasibility remain undecided. Keep game creation distinct from turning detection on.

## Existing expertise and investigation order

Local reference: `/home/jleth/projects/smart-hub/src/modules/autodarts/`.

- `client.rs`: HTTP diagnostics; PUT `/api/start`, `/api/stop`, `/api/reset`; WebSocket URL `/api/events`.
- `poller.rs`: polling and WebSocket ingestion; board, motion, camera and throw data.
- `mod.rs`: Smart Hub command mapping; not a Niles host adapter.
- `model.rs`: diagnostics shape and known local event topics.

These are source observations, not a fresh live API verification. Do not copy the old start-timeout-as-success behavior or household-specific device/room wiring without review. Reset exists in the reference but is not required by the agreed V1 scope.

Next investigation, before implementation:

1. Trace confirmed score/turn semantics in existing code and safe fixtures; identify any required match feed.
2. Inspect current Niles WIT and host implementations for room/device control, satellite selection and targeted speech. Pin the reviewed revision.
3. Resolve announcement granularity with Johan and record the chosen behavior here.
4. Define the smallest end-to-end add-on slice and deterministic test fixtures; then implement within approved coding scope.
5. Verify packaging and host loading separately from approved physical/audio testing.

## Execution and publication boundaries

- This initial bootstrap is documentation only; no runtime, manifest or release claims.
- Keep source-supported, unit-tested, host-tested, installed and live-proven states distinct.
- No audible tests, device/provider writes, live restarts or deployments without explicit approval.
- GitHub publication requires approval; the initial documentation push is explicitly approved. That does not grant indefinite future pushes.
- Linear is not part of this bootstrap. Any Niles maintainer issue must be shown to Johan and explicitly approved before posting to Niles Triage.
- Keep secrets, private captures and generated build artifacts out of Git. Inspect reuse licensing before copying source.
