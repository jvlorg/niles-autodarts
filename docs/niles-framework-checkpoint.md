# Niles framework research checkpoint

Historical checkpoint: this document records research from the source revisions and date context below. Its statements about absent imports and unwired delivery are checkpoint findings, not current component status. For current implementation and proof boundaries, see the [published implementation revision](https://github.com/jvlorg/niles-autodarts/tree/c0e6b23629914e371c7e2525b4c9c1d972fcf3b9) and [canonical plan](plan.md).

## Product and evidence

V1 uses local Board Manager HTTP control/status and dart/removal events. Each dart → Total speaks numeric darts as detected and a cached total once on first explicit partial/full removal; Total only omits individual speech. Local sums are not game-authoritative X01 values. At this checkpoint, `AnnouncementSession` existed and offline verification had passed in that conversation; component event ingress and real speech imports were absent from the inspected contract. No deployment/audio proof was recorded.

## Framework research

Inspected main 2f4d54b739e001d2e4e1fe4e173bf4400dae476a and checked decisive docs/WIT/host/speech files identical on development 71952dafda9db7844bc6a5cd596aabb4be5308a2.

- Host-managed WebSockets explicitly appeared under docs/integrations/host-api.md Coming later in the inspected source revision; not implemented in that checkpoint.
- `notify.say` was the documented Piper/satellite speech route in the inspected revision, but its notification policy made darts commentary unsuitable. That host path targeted a room, not arbitrary satellite IDs; no documented planned high-frequency speech interface was found at that checkpoint.
- Prefer Niles-owned rooms/routing and normal device readings/actions. A separate WS→MQTT bridge could solve ingress only, but is additional infrastructure, not a transparent Wasm feature.
- Read-only Linear lookup of NIL issues found no WebSocket issue. NIL-28 Backlog covers other plugin follow-ups, not WebSockets. No Linear writes made.

## User direction at the checkpoint

The direction was to save the checkpoint, verify repository access and inspect contribution/setup workflow before coding. WebSocket support was intended as the first Niles contribution. That direction did not authorize publishing an issue, pushing, deploying or implementing before Niles workflow gates.

## Contribution rules observed at the checkpoint

The inspected upstream AGENTS.md and CONTRIBUTING.md recorded these rules for Niles contributions: development is base/PR target, main is released. New transports/features require issue and agreed decision before code. Non-Mark contributors file in Triage; only Mark moves to Todo. Work only on unassigned Todo issue not delegated to AI Harness; assign self and move In Progress before coding. Each Linear write requires Johan's exact-payload approval. PR links Fixes NIL-N; Mark review and both CI jobs required; squash into development. No direct protected-branch push.

The GitHub API check reported that the authenticated account had pull/push/triage, no maintain/admin. The local Niles checkout was observed clean on main, behind origin/main by 77; origin/development was missing. The checkpoint cautioned against resetting/switching the existing checkout blindly. Git author was configured as Johan Volmers Leth, GitHub noreply JVLHome.

The checkpoint recorded that Hermes skills trust enables repo-local .agents/skills and .hermes/skills, and that the inspected upstream had shadcn and unslop. Trust had NOT yet been enabled. No branches were fetched/created or setup installed in that investigation. Rust/cargo/node/gh were found; bun was not on PATH. A fresh worktree from development was recommended after setup review, preserving the existing checkout. The inspected Niles instructions differed from the old injected context: its CLAUDE.md imported AGENTS.md. That observation concerned the upstream Niles repository; this add-on's AGENTS.md/CLAUDE.md mirror requirement remains in force.
