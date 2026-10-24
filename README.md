# Niles Autodarts

Standalone [Niles](https://github.com/MarkNygaard/niles) add-on candidate for local Autodarts Board Manager status, detection control, and detection-derived commentary. The implementation targets Niles 0.5.0 and interface 0.5. The alpha.2 candidate remains local; this change is not a release or installation claim.

## Contract

- V1 uses only the configured local Board Manager. No cloud account, match feed, X01 authority, game creation, checkout, bust, or match-outcome claims.
- Detection on/off controls Board Manager detection. It is not hardware power, reset, or game creation.
- `announcements_enabled` and `speech_stream_enabled` default to `false`. Announcement mode must be explicit when enabled.
- One configured room is the speech target. Niles owns household membership, satellite selection, permissions, synthesis, arbitration, and final transport.
- A third validated detected dart arms a fixed one-second total trigger. Explicit takeout may complete it earlier. One- and two-dart rounds complete only on explicit takeout. Empty snapshots do not infer missing darts.
- `Each dart → Total` admits each validated local dart, including detected zeroes, then its immutable total. `Total only` admits only the total. A bounded FIFO and one in-flight host request preserve order; the next request waits for the exact prior terminal host outcome.
- Host `pending`, all terminal outcomes, polling errors, adapter acceptance, and locally discarded requests are distinct. No request is automatically replayed. `delivered` is host transport evidence, not human audibility.
- Multiplier-zero misses are accepted only as canonical `M1` through `M20` with matching number, `Outside` bed, and zero multiplier. Ordinary scoring retains existing validation. Local totals are detection-derived sums, not authoritative game scores.
- Clean `numThrows == 0` is the only baseline. Corrections, malformed input, stopped input, reconnects, disablement, settings changes, overflow, and stale generations invalidate commentary until a new baseline. Ordinary clear/takeout does not cancel already-triggered FIFO work.

## Evidence and commands

- Core parser/session tests and `cargo run -p autodarts-core --example announcement_replay` provide deterministic policy evidence. Component callback tests use isolated fake WIT imports and cover timer identity, FIFO admission, correlated terminal outcomes, polling recovery, lifecycle invalidation, bounded overflow, and the exact 40-envelope JVL-14 reduced capture. The compiled proof uses the pinned Niles checkout's real scheduler and a recording output. Installed-host compatibility, general live Board Manager ordering, household routing, and audible playback remain open.

The supplied installed alpha.2 observation at `82a30c4155e6ba20806a00734042a1f4ea49c1a7` is historical user evidence for audible hit and takeout-total behavior plus captured outside-miss failures. It does not establish that this candidate is installed, released, generally compatible, routed, or audibly verified.

- `crates/autodarts-core/tests/fixtures/board-manager-jvl14-three-round-reduced-capture.json` is the supplied JVL-14 reduced receive-only capture, verified equal as parsed ordered JSON including all 40 envelopes, fields, and omissions. `board-manager-jvl14-synthetic-scenarios.json` is separately labelled synthetic input for deadline/takeout races and edge cases. The exact capture replay produces dart values `60`, `20`, `7`, `40`, `20`, `0`, `12`, `18`, `0` and totals `87`, `60`, and `30`; neither fixture establishes live behavior.

`bash scripts/verify.sh --quick` runs the scoped local chain. `bash scripts/verify.sh` runs the maintained final chain, including pinned-host preparation, exact WIT comparison, packaging, exact-asset diagnostics, compiled proof, checksums, and actionlint. The compiled proof requires free loopback port 3180. Do not change pinned WIT, host revision, permissions, package version, or manifest URL policy to make proof pass.

Before release publication, a repository administrator must configure required reviewers on the `alpha-release` environment, then set repository variable `ALPHA_RELEASE_APPROVAL_CONFIGURED=true`. The workflow fails closed without that acknowledgment; the variable alone does not configure protection. Manual `publish=false` validates without entering publication. No protected-environment approval or publication is implied by local workflow checks.

Keep credentials, household captures, private configuration, generated candidates, and host caches out of Git. Do not perform live device writes, audible tests, installations, deployments, pushes, or release publication without explicit approval. V2 TV scoreboard and V3 game selection remain out of scope.
