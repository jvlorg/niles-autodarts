# Niles Autodarts

Standalone [Niles](https://github.com/MarkNygaard/niles) add-on for local Autodarts Board Manager status, detection control, and detection-derived commentary. The implementation targets Niles 0.5.0 and interface 0.5. The alpha candidate is local only; no release has been published or installed.

## Contract

- V1 uses only the configured local Board Manager. No cloud account, match feed, X01 authority, game creation, checkout, bust, or match-outcome claims.
- Detection on/off controls Board Manager detection. It is not hardware power, reset, or game creation.
- `announcements_enabled` and `speech_stream_enabled` both default to `false`. Announcement mode must be explicit when enabled.
- One configured room is the only speech target. Niles owns household membership, satellite selection, permissions, synthesis, arbitration, and final transport.
- `Each dart → Total` submits every eligible local dart immediately and then the cached total. The released Niles scheduler has one replaceable pending slot, so accepted requests may finish as `Replaced`; this mode does not guarantee transport delivery of every utterance.
- `Total only` submits only the eligible total at the first explicit partial/full removal marker.
- Empty `numThrows == 0` is the clean baseline. Clear, shrink, malformed input, corrections, reconnects, disablement, settings changes, overflow, and stale generations suppress ambiguous commentary until a new baseline.
- Accepted, pending, terminal host outcomes, polling errors, and locally discarded requests are distinct. No automatic retry is performed. Pending bookkeeping is bounded to 128 requests and terminal history to 32 records.
- Local totals are simple detection-derived sums, not authoritative game scores. Bull handling is synthetic-tested; live miss encoding remains unverified.

## Evidence and commands

Core parser/session tests and `cargo run -p autodarts-core --example announcement_replay` provide deterministic policy evidence. Component callback tests use isolated fake WIT imports and cover accepted/pending/terminal outcomes, polling recovery, lifecycle invalidation, oversized frames, and overflow recovery. The compiled host proof uses the pinned Niles checkout's real speech scheduler and an output recorder; installed-host compatibility, live Board Manager ordering/miss encoding, household routing, and audible playback remain open installation checks.

The sole host pin is `scripts/niles-host-revision` (`4f3c0a10b2accd022354a23b9f961b222c27eb56`). `bash scripts/prepare-host.sh` verifies a detached, clean checkout under `.cache/niles` and builds its CLI. `bash scripts/verify.sh --quick` runs local Rust checks, the replay, maintained probe, and guidance-file equality. Full verification additionally runs pinned WIT comparison, packaging, the exact-package host diagnostic, the compiled proof, checksum verification, and workflow syntax checks.

The alpha package route, after an approved publication, is [Niles Autodarts v0.1.0-alpha.1](https://github.com/jvlorg/niles-autodarts/releases/tag/v0.1.0-alpha.1): Niles Preview → add the exact release URL → inspect the manifest and permissions → explicitly approve. This URL is an installation route, not evidence that publication has occurred.

Keep credentials, household captures, private configuration, generated candidates, and host caches out of Git. Do not perform live device writes, audible tests, installations, deployments, pushes, or release publication without explicit approval.
