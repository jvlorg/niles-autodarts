# Niles framework research checkpoint

> **Historical checkpoint.** The observations below describe an earlier framework investigation and are superseded for current add-on wiring by [implementation commit `c0e6b23629914e371c7e2525b4c9c1d972fcf3b9`](https://github.com/jvlorg/niles-autodarts/commit/c0e6b23629914e371c7e2525b4c9c1d972fcf3b9). For current source and evidence boundaries, see the [announcement path](announcement-path.md) and [canonical plan](plan.md).

Autodarts implementation is paused while preparing a first Niles contribution. Local implementation remains uncommitted and unpublished; only initial documentation was previously pushed. Do not discard pending work.

## Product and evidence

The product/evidence statement below is historical. V1 uses local Board Manager control/status and event policy; local sums are not game-authoritative X01 values. Current component event and room-bound speech wiring, plus its unproven installation/live/audible behavior, are documented in the [canonical plan](plan.md) and [announcement path](announcement-path.md).

## Framework research

Historical inspection: main 2f4d54b739e001d2e4e1fe4e173bf4400dae476a and decisive docs/WIT/host/speech files identical on development 71952dafda9db7844bc6a5cd596aabb4be5308a2.

- At that inspection point, host-managed WebSockets appeared under docs/integrations/host-api.md Coming later; current component wiring is described in [announcement-path](announcement-path.md).
- Current notify.say is the documented Piper/satellite speech route, but six-per-hour policy makes darts commentary unsuitable. Host targets a room, not arbitrary satellite IDs. No documented planned high-frequency speech interface found.
- Prefer Niles-owned rooms/routing and normal device readings/actions. A separate WS→MQTT bridge could solve ingress only, but is additional infrastructure, not a transparent Wasm feature.
- Read-only Linear lookup of NIL issues found no WebSocket issue. NIL-28 Backlog covers other plugin follow-ups, not WebSockets. No Linear writes made.

## Current user direction

Save checkpoint, verify repository access and inspect contribution/setup workflow before coding. WebSocket support is intended as first Niles contribution. This is not authorization to publish an issue, push, deploy or implement before Niles workflow gates.

## Verified contribution rules

Current upstream AGENTS.md and CONTRIBUTING.md: development is base/PR target, main is released. New transports/features require issue and agreed decision before code. Non-Mark contributors file in Triage; only Mark moves to Todo. Work only on unassigned Todo issue not delegated to AI Harness; assign self and move In Progress before coding. Each Linear write still requires Johan's exact-payload approval. PR links Fixes NIL-N; Mark review and both CI jobs required; squash into development. No direct protected-branch push.

GitHub API confirms authenticated account has pull/push/triage, no maintain/admin. Local Niles checkout observed clean on main, behind origin/main by 77; origin/development missing. Do not reset/switch existing checkout blindly. Git author configured Johan Volmers Leth, GitHub noreply JVLHome.

Hermes skills trust enables repo-local .agents/skills and .hermes/skills. Current upstream has shadcn and unslop. Trust has NOT yet been enabled. No branches fetched/created or setup installed in this investigation. Rust/cargo/node/gh found; bun not on PATH. Fresh worktree from development is recommended after setup review, preserving existing checkout. Repo-local instructions differ from old injected context: CLAUDE.md now imports AGENTS.md; do not enforce obsolete mirror rule against current upstream.
