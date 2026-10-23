# Implementation evidence

## Scope delivered

- Pinned Niles source revision: `4f3c0a10b2accd022354a23b9f961b222c27eb56`, prepared under `.cache/niles` by `scripts/prepare-host.sh`.
- Released 0.5 WIT synchronized byte-for-byte with the pinned checkout; digest recorded in `component/wit/PROVENANCE.md`.
- Component speech lifecycle now tracks host request IDs, pending requests, every released terminal outcome, polling failures, bounded history, and stale-generation discard.
- Core delivery terminology distinguishes adapter acceptance from transport delivery.
- Deterministic core replay covers both announcement modes over three collections.
- Component fake-import tests cover lifecycle transitions, status correlation, polling recovery, overflow, reset, and open/submit rejection.
- Exact candidate packaging, host diagnostic, verification modes, and approval-gated alpha workflow are checked in.

## Verification run

- `cargo +1.99.0 check --workspace`: passed.
- `cargo +1.99.0 test -p niles-autodarts-component --lib`: passed, 15 tests.
- `cargo +1.99.0 check -p autodarts-core --examples`: passed.
- `cargo +1.99.0 run -p autodarts-core --example announcement_replay`: passed; both modes emitted expected synthetic policy submissions.
- `PATH=/home/harness/.local/bin:$PATH bash scripts/verify.sh --quick`: passed. Workspace tests: 25 core tests and 15 component tests. Wasm release build passed. Maintained loopback probe: 8 tests passed. `cmp AGENTS.md CLAUDE.md` passed.

## Full-chain status

The full chain was not claimed as passed. The compiled host-proof check is blocked before compilation by `openssl-sys v0.9.117`: this environment lacks the OpenSSL development installation and `pkg-config` metadata required by the pinned Niles dependency graph. The host proof also requires a loopback port covered by the released manifest permission contract; the component currently accepts only the declared 80/3180 HTTP grants, so a dynamic port-zero proof cannot be represented without inventing a host permission.

No live Board Manager, device write, household capture, audible test, install, deployment, push, or release publication was performed.
