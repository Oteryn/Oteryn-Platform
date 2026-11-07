# CATALOG_EXPORT_V1 compatibility source pin

Read-only source: `Oteryn/Oteryn-Game@d34807f5bd234ee8318e57ba454ba39be0df113d`.

- Exact copied input: `tools/game-platform-catalog/fixtures/unsupported-native-input.json`.
- Exact copied contract: `docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md` (`CONTRACT.md`).
- Exact reference producer: `tools/game-platform-catalog/producer.py`, retained as
  test tooling in `portal-rs/e2e/support/catalog-producer.py`.
- Reference producer SHA-256: `c3a69662610d8e52430f24a04e0878ea3055dc62675283626a708e24f41dcc7e`.

No generated artifact or sidecar is committed. The harness generates the negative
artifact under its private scratch directory using the pinned producer. Rust test
code in `tests/support/catalog.rs` generates in-memory negative and synthetic
supported vectors, pinned to the reference producer's known byte hashes. The
supported Item input is the exact `test_producer.py::valid_source()` example;
the escaped-NUL variant changes only its synthetic display name.

The negative input's source revision is
`a2a5da955dd8f580c9e768c8ac6a741db388cb22`, intentionally distinct from this
producer/contract evidence pin. All categories are unsupported/unknown. No vector
here establishes a real native capability or production activation authority.

When CATALOG merges, update this single evidence directory/pin and test producer
from the merged SHA; real CI artifacts are supplied by a configured read-only
path/URL with an independently chosen expected SHA-256 and content revision.
