# CATALOG_EXPORT_V1 compatibility source pin

Read-only source: `Oteryn/Oteryn-Game`, branch
`claude/happy-davinci-ijz3wy-catalog`, full revision
`fd4be4e214ef33ad65d3ce465dab7160377e62f5`.
**Lane branch, re-pin on merge.** This is not a merged Game release.

Exact copied sources:

- `tools/game-platform-catalog/fixtures/unsupported-native-input.json`.
- `docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md` (`CONTRACT.md`).
- `docs/reference/CATALOG-CONTRACT-NOTES.md`.
- `tools/game-platform-catalog/producer.py`, retained as test tooling in
  `portal-rs/e2e/support/catalog-producer.py`. SHA-256:
  `c3a69662610d8e52430f24a04e0878ea3055dc62675283626a708e24f41dcc7e`.

The contract, negative source input and reference producer are byte-identical to
our preceding d34807f5 pin. The lane's G-2 shared public view omits hidden
identities, hidden NPC shops and secret achievements. Optional explicit
`visibility` may only be the string `public`; absence is the normal wire form,
not a Platform-invented visibility default.

The lane notes lines 38–62 explicitly keep all eight native capabilities
unsupported/unknown pending capability-payload promotion. Its native builder
cannot generate supported public data at this revision. No populated native
fixture or positive native browser result is claimed. Areas are scheduled for an
additive later export revision.

No generated artifact or sidecar is committed. From `portal-rs/`, generate the
pinned negative reference artifact in a private scratch directory:

```sh
mkdir -p -m 700 /tmp/portal-catalog-negative
python3 e2e/support/catalog-producer.py produce \
  crates/world/tests/fixtures/catalog-fd4be4e2/unsupported-native-input.json \
  /tmp/portal-catalog-negative/game-platform-catalog.json
```

Artifact SHA-256:
`4fe8ce4488c21396b588dc6b04179cde93c7a9b22b87427b6e4af3cda6e60d7d`.
The source input deliberately claims revision
`a2a5da955dd8f580c9e768c8ac6a741db388cb22`, distinct from the producer evidence
pin. It is a negative compatibility fixture, not an export of the lane content.

Rust `tests/support/catalog.rs` generates in-memory test-only supported vectors.
The original Item input is the reference producer's `test_producer.py::valid_source()`;
its byte hash is pinned. Other test vectors prove category/visibility/refusal
behavior and never enter the production binary or imply native availability.
Production artifacts must come from an admin-configured read-only path/URL plus
independently chosen SHA-256 and provenance fences. Update this pin and producer
from the merged revision after CATALOG lands.
