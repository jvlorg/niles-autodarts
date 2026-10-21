# WIT provenance

The checked-in file `niles-integration.wit` is copied from the frozen Niles integration 0.5 schema in the read-only Niles reference checkout.

- Niles reference checkout: `/home/jleth/projects/niles-nil43`, inspected commit `d9362f05f27bbe9e8ab83e530f352da08999a97b`.
- Source: `schemas/niles-integration-0.5.wit`, which defines interface 0.5 and its room-bound `speech-stream` API.
- Author contract cross-checked against `site/content/docs/writing-add-ons/manifest.mdx`, `host-api.mdx`, and `permissions.mdx` at that same checkout.
- The reference identifies PRs #482 (speech stream) and #484 (tracked notification requests) as review-ready work, not merged/released/installed capabilities. This component uses 0.5 and does not import notifications.
- Do not infer a released minimum Niles version supporting speech streams from this source snapshot; it is not recorded as an installable compatibility claim here.
