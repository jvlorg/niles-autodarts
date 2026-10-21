# Niles Autodarts

Standalone [Niles](https://github.com/MarkNygaard/niles) add-on for local Autodarts Board Manager status, detection control and score announcements.

The component owns an `AnnouncementSession`, receives Board Manager `/api/events` over its declared WebSocket permission, and submits/polls speech requests through a configured-room Niles speech stream. Deterministic fixtures exercise component lifecycle and recording-host behavior. Installed-host compatibility, live event ordering/miss encoding and audible delivery remain unproven; room-bound requests do not establish selected-satellite control. See the [announcement path and evidence](docs/announcement-path.md) and [canonical plan](docs/plan.md).

The add-on parses local `{type,data}` events and computes detection-derived collection sums. They are not credited X01 scores, remaining scores, busts or checkout results. Bull numbers 25 and 50 are accepted; live miss encoding remains unresolved. Both Each dart → Total and Total only policy modes remain supported. Announcement settings default disabled. Detection on/off means Board Manager detection only, not physical hardware power, reset or game creation. V2 TV scoreboard and V3 game selection remain later roadmap scope.

## Verification

Run `bash scripts/verify.sh` for the maintained formatting, workspace build, strict Clippy, workspace tests and release Wasm build. The optional `--host-proof /absolute/path/to/niles` path depends on the developer-local Niles checkout documented in [announcement-path](docs/announcement-path.md); it is not a portable installation procedure and uses a recording speech host. `scripts/verify-host.py` is a separate synthetic HTTP dry-run diagnostic. None of these paths alone proves installed compatibility, live Board Manager semantics, household audience selection or audible output.

Keep credentials, household captures and local configuration out of Git. Live device operations, installation and audible testing require separate explicit authorization.
