# WIT provenance

The checked-in `niles-integration.wit` is the released Niles 0.5 contract.

- Niles source checkout: `.cache/niles`, pinned by `scripts/niles-host-revision` to `4f3c0a10b2accd022354a23b9f961b222c27eb56`.
- Source path: `schemas/niles-integration-0.5.wit`.
- Comparison: `cmp component/wit/niles-integration.wit .cache/niles/schemas/niles-integration-0.5.wit`.
- Digest at the pinned revision: `acb5a8c1fcc84810827fcc2612d3e055157ac64e0cfc0801fd4b29433efa6b70` for both files.
- The WIT files are byte-for-byte equivalent at the pinned revision; the component retains interface `0.5` and adds no private host API.
- Full verification repeats the comparison after `scripts/prepare-host.sh`; a mismatch fails before packaging or host proof.
