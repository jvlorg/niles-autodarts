# Niles Autodarts

Standalone [Niles](https://github.com/MarkNygaard/niles) add-on for local Autodarts Board Manager status, detection control, and detection-derived commentary. The implementation targets Niles 0.5.0 and interface 0.5. The alpha.1 installation and Board Manager event/room-stream connections are user-reported observations; the alpha.2 candidate remains local and is not installed or published here.

## Contract

- V1 uses only the configured local Board Manager. No cloud account, match feed, X01 authority, game creation, checkout, bust, or match-outcome claims.
- Detection on/off controls Board Manager detection. It is not hardware power, reset, or game creation.
- `announcements_enabled` and `speech_stream_enabled` both default to `false`. Announcement mode must be explicit when enabled.
- One configured room is the only speech target. Niles owns household membership, satellite selection, permissions, synthesis, arbitration, and final transport.
- `Each dart → Total` submits every eligible local dart immediately and then the cached total. The released Niles scheduler has one replaceable pending slot, so accepted requests may finish as `Replaced`; this mode does not guarantee transport delivery of every utterance.
- `Total only` submits only the eligible total at the first explicit partial/full removal marker.
- An integer `numThrows: 0` with `throws` absent or `throws: []` is the clean baseline. Explicit `throws: null`, nonzero/unknown counts without an array, zero with nonempty throws, clear/shrink, malformed input, corrections, reconnects, disablement, settings changes, overflow, and stale generations suppress ambiguous commentary until a new baseline.
- Accepted, pending, terminal host outcomes, polling errors, and locally discarded requests are distinct. No utterance is retried. Session reopening can wait for asynchronous host close; subsequent callbacks retry only that unavailable open. Pending bookkeeping is bounded to 128 requests and terminal history to 32 records.
- Local totals are simple detection-derived sums, not authoritative game scores. Bull handling is synthetic-tested; live miss encoding remains unverified.

## Evidence and commands

The shared reduced 14-envelope capture in `crates/autodarts-core/tests/fixtures/board-manager-1.0.7-reduced-capture.json` preserves the reported startup ordering without an invented baseline. Core replay, real Guest callbacks, and the compiled proof append explicitly synthetic 3/7/7 rounds; they establish policy and runtime behavior, not installed-host compatibility or human-audible output. Rendered diagnostics distinguish readiness, ingress decisions, adapter acceptance, pending requests, polling errors, and host terminal outcomes.

The sole host pin is `scripts/niles-host-revision` (`4f3c0a10b2accd022354a23b9f961b222c27eb56`). `bash scripts/prepare-host.sh` verifies a detached, clean checkout under `.cache/niles` and builds its CLI. `bash scripts/verify.sh --quick` runs local Rust checks, the replay, maintained probe, and guidance-file equality. Full verification additionally runs pinned WIT comparison, packaging, the exact-package host diagnostic, the compiled proof, checksum verification, and workflow syntax checks.

Use `bash scripts/package.sh --tag v0.1.0-alpha.2 --output dist/v0.1.0-alpha.2` from a clean tracked source commit and a fresh output directory. The package contains the exact checked manifest/Wasm, `SHA256SUMS`, and `provenance.json`. Full verification also needs `uv`, `actionlint`, the Wasm target and the pinned host's build prerequisites. The compiled synthetic fixture requires free loopback port 3180; the candidate's declared WebSocket permissions do not allow an ephemeral port. [Announcement-path evidence](docs/announcement-path.md) records the exercised scenarios and remaining proof boundaries.

Before publication, a repository administrator must configure required reviewers on the `alpha-release` environment, then set repository variable `ALPHA_RELEASE_APPROVAL_CONFIGURED=true`. The workflow fails closed without that acknowledgment; the variable alone does not configure protection. Manual `publish=false` validates without entering publication. No protected-environment approval or publication is implied by local workflow checks.

The alpha.2 package route, after an approved publication, is [Niles Autodarts v0.1.0-alpha.2](https://github.com/jvlorg/niles-autodarts/releases/tag/v0.1.0-alpha.2): Niles Preview → add the exact release URL → inspect the manifest and permissions → explicitly approve. The historical alpha.1 installation observation is not evidence of alpha.2 publication, routing, or audibility.

Keep credentials, household captures, private configuration, generated candidates, and host caches out of Git. Do not perform live device writes, audible tests, installations, deployments, pushes, or release publication without explicit approval.
