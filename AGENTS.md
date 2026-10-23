# Agent guidance

Read `docs/plan.md` before working. It is the canonical product scope and progress record; keep README claims consistent with it.

This repository contains a local implementation candidate for a standalone Niles add-on targeting the pinned Niles 0.5.0 contract. Do not claim that an installable release, installed-host compatibility, live device behavior, household routing, or audible playback exists unless the recorded evidence proves it. Do not invent host APIs. Keep provider parsing and local detection policy here; household membership, audience resolution, speech authority, arbitration, synthesis, and final transport remain in Niles.

Preserve the V1/V2/V3 boundaries and unresolved decisions. Do not assume per-dart versus turn-total announcements beyond the documented explicit modes. Detection on/off is distinct from hardware power, reset, and game creation. `Accepted` is an adapter submission result, not audible delivery.

Use the pinned host preparation and exact-asset verification scripts. Keep credentials, household captures, local configuration, generated candidates, and caches out of Git. Use deterministic synthetic fixtures. Live device writes, audible tests, installations, deployments, releases, and pushes need explicit approval. Stage explicit files only.

`AGENTS.md` and `CLAUDE.md` must remain identical.
