# CATALOG_EXPORT_V1 compatibility source pin

Read-only source: `Oteryn/Oteryn-Game` `main`, full revision
`669d5af5261a60d347167b7c880a4498aebe72ed` (squash merge of Game PR #3025, the
CATALOG lane previously pinned at `fd4be4e214ef33ad65d3ce465dab7160377e62f5`).

Exact copied source:

- `tools/game-platform-catalog/fixtures/unsupported-native-input.json`.

Referenced at the same revision, not vendored (read them through the pinned links):

- [`docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md`](https://github.com/Oteryn/Oteryn-Game/blob/669d5af5261a60d347167b7c880a4498aebe72ed/docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md),
  SHA-256 `9bc87fba5b565e5c7d4d3f6ca7a9bd75d45d8110de64a2a50f8f74d9ba181cad`.
- [`docs/reference/CATALOG-CONTRACT-NOTES.md`](https://github.com/Oteryn/Oteryn-Game/blob/669d5af5261a60d347167b7c880a4498aebe72ed/docs/reference/CATALOG-CONTRACT-NOTES.md),
  SHA-256 `495ab72d4261fe7ae74d6eed56f61cef7d0ddf2aae7b7e85950a1141761aabdd`.
- `tools/game-platform-catalog/producer.py`, retained as test tooling in
  `portal-rs/e2e/support/catalog-producer.py`. SHA-256:
  `865afe56516de5fe23d5420ca788a25e067033f748334aaa68afc8432b6ee9ae`.

The contract and negative source input are byte-identical to the preceding
`fd4be4e2` lane pin. The merged producer adds one fail-closed check: a nested
`visibility` member anywhere in record `data` must be the string `public`
(notes lines 92–101); the Rust verifier already enforces the same rule. The notes
otherwise add Quest provenance, CI artifact-build and a design-only `area` 1.1.0
proposal. The negative artifact bytes and SHA-256 are unchanged. The G-2 shared public view omits hidden
identities, hidden NPC shops and secret achievements. Optional explicit
`visibility` may only be the string `public`; absence is the normal wire form,
not a Platform-invented visibility default.

The notes lines 54–78 explicitly keep all eight native capabilities
unsupported/unknown pending capability-payload promotion. Its native builder
cannot generate supported public data at this revision. No populated native
fixture or positive native browser result is claimed. Areas are scheduled for an
additive later export revision.

No generated artifact or sidecar is committed. From `portal-rs/`, generate the
pinned negative reference artifact in a private scratch directory:

```sh
mkdir -p -m 700 /tmp/portal-catalog-negative
python3 e2e/support/catalog-producer.py produce \
  crates/world/tests/fixtures/catalog-669d5af5/unsupported-native-input.json \
  /tmp/portal-catalog-negative/game-platform-catalog.json
```

Artifact SHA-256:
`4fe8ce4488c21396b588dc6b04179cde93c7a9b22b87427b6e4af3cda6e60d7d`.
The source input deliberately claims revision
`a2a5da955dd8f580c9e768c8ac6a741db388cb22`, distinct from the producer evidence
pin. It is a negative compatibility fixture, not an export of Game content.

Rust `tests/support/catalog.rs` generates in-memory test-only supported vectors.
The original Item input is the reference producer's `test_producer.py::valid_source()`;
its byte hash is pinned. Other test vectors prove category/visibility/refusal
behavior and never enter the production binary or imply native availability.
Production artifacts must come from an admin-configured read-only path/URL plus
independently chosen SHA-256 and provenance fences. Game `main` CI builds the
real export as the `game-platform-catalog` artifact (notes lines 111–126); at this
revision it still declares every capability unsupported/unknown, so the wiki
correctly stays 503 when pointed at it.

Observed CI evidence at 669d5af5 (reported by the programme coordinator, not
re-downloaded here): `rust.yml` run 42409329543, artifact `game-platform-catalog`
(id 13589446076), `game-platform-catalog.json.sha256` =
`892caf99952c3bc8b920abdd7f9c8b5aca1d5630f0c6f916ef2e11290247139d`;
`capability_manifest` has 8 entries, all `unsupported`, and
`completeness_manifest` has 8 entries, all `unknown`. This hash is provenance
only; operators still choose their own SHA-256 fence for a configured artifact.
