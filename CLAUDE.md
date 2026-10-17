# Agent guidance

Read `docs/plan.md` before working. It is the canonical product scope and progress record; keep README claims consistent with it.

This is a standalone Niles add-on, currently planning-only. Do not invent host APIs or claim an installable implementation exists. Inspect the actual Niles contract before wiring code. Keep provider behavior here and household/speech authority in Niles.

Preserve the V1/V2/V3 boundaries and unresolved decisions. Do not assume per-dart versus turn-total announcements. Detection on/off is distinct from hardware power and game creation.

Keep credentials, household captures and local configuration out of Git. Use deterministic synthetic fixtures. Live device writes, audible tests, deployments and pushes need explicit approval. Stage explicit files only.

`AGENTS.md` and `CLAUDE.md` must remain identical.
