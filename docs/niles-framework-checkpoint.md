# Archived Niles framework research checkpoint

This document preserves observations from an earlier Niles upstream investigation. It is historical context, not current add-on implementation status, workflow guidance or authorization. For the current contract and evidence limits, see the [canonical plan](plan.md), [core communication inventory](core-communication.md), and [announcement path](announcement-path.md).

## Historical product and evidence observations

At the time of that checkpoint, the add-on investigation described local Board Manager HTTP control/status and live dart/removal events. It recorded an `AnnouncementSession` and offline verification while treating live component event ingress and real speech as absent. Those statements describe that earlier state only; the current component lifecycle is documented in the canonical plan. The local-sum, non-authoritative-X01 boundary remains applicable.

## Framework research at inspected revisions

The investigation inspected Niles main at `2f4d54b739e001d2e4e1fe4e173bf4400dae476a` and checked decisive docs/WIT/host/speech files against development revision `71952dafda9db7844bc6a5cd596aabb4be5308a2`.

- Host-managed WebSockets were described in the inspected documentation as “Coming later”; that was not implemented at those inspected revisions.
- `notify.say` was the documented Piper/satellite notification route, but its six-per-hour policy made darts commentary unsuitable. It targeted a room, not arbitrary satellite IDs. No documented planned high-frequency speech interface was found in that research.
- The investigation preferred Niles-owned rooms/routing and normal device readings/actions. A separate WS-to-MQTT bridge was considered additional infrastructure rather than a transparent Wasm feature.
- A read-only Linear lookup found no WebSocket issue; NIL-28 Backlog covered other plugin follow-ups. No Linear writes were made.

These are revision-specific observations and do not describe the checked-in add-on WIT or current component implementation.

## Historical user direction and contribution workflow

The user direction recorded then was to save the checkpoint, verify repository access and inspect contribution/setup workflow before coding. WebSocket support was intended as a first Niles contribution; this was not authorization to publish an issue, push, deploy or implement before Niles workflow gates.

The upstream contribution rules observed at that time were: `development` was the base/PR target and `main` was released; new transports/features required an issue and agreed decision before code; non-Mark contributors filed in Triage; only Mark moved issues to Todo; work was limited to unassigned Todo issues not delegated to AI Harness; contributors assigned themselves and moved work to In Progress before coding; each Linear write required Johan's exact-payload approval; PRs linked `Fixes NIL-N`, required Mark review and both CI jobs, then were squash-merged into `development`; direct protected-branch pushes were disallowed. These rules applied to that historical upstream Niles repository and are not this add-on's current PR rules.

The checkpoint reported authenticated GitHub access with pull/push/triage but no maintain/admin; a local Niles checkout clean on `main`, 77 commits behind `origin/main`, and no local `origin/development`; and a Git author of Johan Volmers Leth with a GitHub noreply address. It advised against blindly resetting or switching that checkout. These were observations then, not repeated checks.

It also recorded Hermes skills trust behavior, repo-local `.agents/skills` and `.hermes/skills`, no trust enablement at that time, no fetched/created branches or installed setup, and Rust/cargo/node/gh availability with bun absent from PATH. A fresh worktree from `development` was recommended after setup review. The upstream repository's then-current instructions differed from older injected context, including a `CLAUDE.md` import of `AGENTS.md`; this remains historical repository-specific context only.
