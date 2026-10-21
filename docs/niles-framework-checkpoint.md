# Niles framework research checkpoint

> Historical research snapshot. This page records an earlier Niles host-contract review and the project state at that time. It is not current implementation guidance or evidence of installed-host compatibility. For the checked-in component and current evidence limits, see the [canonical plan](plan.md), [core communication inventory](core-communication.md), and [pinned WIT provenance](../component/wit/PROVENANCE.md).

## Earlier host-contract findings

The review inspected Niles main at `2f4d54b739e001d2e4e1fe4e173bf4400dae476a` and compared decisive documentation, WIT, host, and speech files with development commit `71952dafda9db7844bc6a5cd596aabb4be5308a2`.

- Host-managed WebSockets appeared under `docs/integrations/host-api.md` as coming later; they were not implemented in that inspected snapshot.
- `notify.say` was the documented Piper/satellite speech route, but its six-per-hour policy did not fit dart commentary. The host targeted a room rather than arbitrary satellite IDs, and no planned high-frequency speech interface was found in the reviewed docs.
- The review favored Niles-owned rooms/routing and normal device readings/actions. A separate WebSocket-to-MQTT bridge could address ingress, but would add infrastructure rather than provide a transparent Wasm feature.
- A read-only Linear lookup at that time found no WebSocket issue; NIL-28 covered other plugin follow-ups. No Linear writes were made.

These observations explain why the earlier design lacked supported event ingress and speech binding. They do not describe the current pinned 0.5.0 WIT contract, establish that its upstream changes were merged or released, or prove compatibility with an installed Niles host. Current implementation and proof limits are maintained in the [canonical plan](plan.md#verification-and-proof-boundaries).

## Earlier contribution and workspace observations

The original checkpoint also recorded a then-current contribution workflow: development was the PR target, new transports/features required an issue and an agreed decision before coding, and protected branches were not to be pushed directly. It recorded a local Niles checkout on `main`, 77 commits behind `origin/main`, no local `origin/development`, and repository-specific setup observations. These were point-in-time notes, not instructions for this add-on's current work or claims about present upstream state.
