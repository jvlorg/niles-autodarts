# Niles framework research checkpoint

Historical checkpoint: the source investigation below concerned Niles revisions `2f4d54b739e001d2e4e1fe4e173bf4400dae476a` and `71952dafda9db7844bc6a5cd596aabb4be5308a2`. It records the earlier implementation direction, not current component capability. The checked-in WebSocket and room-bound speech wiring supersedes its direction to implement those imports; see the [canonical plan](plan.md) for current status.

At that checkpoint, Autodarts implementation was paused while preparing a first Niles contribution. Local implementation was described as uncommitted and unpublished, with only initial documentation previously pushed. This note preserves that historical state; it is not current worktree guidance.

## Product and evidence at the checkpoint

The checkpoint described V1 as using local Board Manager HTTP control/status and live dart/removal events. Each dart → Total spoke numeric darts as detected and a cached total once on first explicit partial/full removal; Total only omitted individual speech. Local sums were not game-authoritative X01 values. It recorded an injectable AnnouncementSession and offline verification in that conversation, with live component event ingress and real speech then absent. No deployment/audio proof was recorded. The source status has since changed; consult the canonical plan.

## Framework research at the checkpoint

The checkpoint inspected main 2f4d54b739e001d2e4e1fe4e173bf4400dae476a and checked decisive docs/WIT/host/speech files identical on development 71952dafda9db7844bc6a5cd596aabb4be5308a2.

- Host-managed WebSockets appeared under docs/integrations/host-api.md Coming later and were not implemented in the inspected snapshot.
- In the inspected snapshot, notify.say was the documented Piper/satellite speech route, but its six-per-hour policy made darts commentary unsuitable. Host targeting was a room, not arbitrary satellite IDs. No documented planned high-frequency speech interface was found.
- The checkpoint recommended Niles-owned rooms/routing and normal device readings/actions. It considered a separate WS→MQTT bridge an ingress-only additional infrastructure option, not a transparent Wasm feature.
- A read-only Linear lookup recorded no WebSocket issue; NIL-28 Backlog covered other plugin follow-ups, not WebSockets. No Linear writes were made.

## User direction recorded at the checkpoint

The recorded direction was to save the checkpoint, verify repository access and inspect contribution/setup workflow before coding. WebSocket support was intended as the first Niles contribution. The checkpoint did not authorize publishing an issue, pushing, deploying or implementing before the Niles workflow gates.

## Contribution rules recorded at the checkpoint

The upstream AGENTS.md and CONTRIBUTING.md inspected then stated that development was the base/PR target and main was released. New transports/features required an issue and agreed decision before code. Non-Mark contributors were to file in Triage; only Mark moved issues to Todo. The inspected process required work on an unassigned Todo issue not delegated to AI Harness, self-assignment and movement to In Progress before coding. Each Linear write required Johan's exact-payload approval. PRs linked Fixes NIL-N, required Mark review and both CI jobs, and were squashed into development. Direct protected-branch pushes were disallowed.

The GitHub API check recorded that the authenticated account had pull/push/triage access, without maintain/admin. The local Niles checkout was observed clean on main, 77 commits behind origin/main, with origin/development missing. The checkpoint advised against blindly resetting or switching that checkout. Git author was configured as Johan Volmers Leth, GitHub noreply JVLHome.

The checkpoint found that Hermes skills trust enabled repo-local .agents/skills and .hermes/skills; inspected upstream contained shadcn and unslop. Trust had not yet been enabled, and no branches were fetched/created or setup installed during that investigation. Rust/cargo/node/gh were found; bun was not on PATH. It recommended a fresh worktree from development after setup review, preserving the existing checkout. Repo-local instructions differed from the old injected context: CLAUDE.md imported AGENTS.md, so the checkpoint advised not enforcing the obsolete mirror rule against that upstream.
