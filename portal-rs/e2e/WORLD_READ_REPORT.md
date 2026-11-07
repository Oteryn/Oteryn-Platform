# Native world read qualification

Branch: `codex/portal-rs-world-read`. Base: origin/main `3d6a1ae2`.
Owner's Game-coordinator update supersedes raw content reading and tile rendering.
No PR, workflow changes, deployment, Game writes, runtime fetch, world seeding,
Today producer or visibility bypass.

## Result

Implemented CATALOG_EXPORT_V1 verification and transactional idempotent **inactive**
candidate intake at boot. Verified canonical artifact bytes are retained as `bytea`,
not normalized through JSONB, so every allowed JSON string (including escaped
NUL) survives exactly. The real-PostgreSQL test proves this with a synthetic
Game-producer vector. The offline transport does not authorize native publication.
The reference producer and fixture were fetched read-only at Game
`d34807f5bd234ee8318e57ba454ba39be0df113d`; the copied contract digest is
`9bc87fba5b565e5c7d4d3f6ca7a9bd75d45d8110de64a2a50f8f74d9ba181cad`, matching the
LOCKED Game registry. The fixture's source revision remains its declared `a2a5da95…`.
No raw Game layout is coupled to the binary. Older raw parser APIs have no new
binary caller. Admin approvals remain independent and unmodified.

The registry-backed world picker returns 200 without a query when worlds exist;
empty or unavailable registries still refuse. Query statistics retain the existing
privacy gate. The harness UUIDv7 is corrected. Atlas aliases require every
explicit alias to enable its gate. Today documentation now says PROPOSED.

## Commands and results

Validation used a task-owned PostgreSQL **18.6** container, bound only to loopback,
with `max_locks_per_transaction=1024` and `max_connections=300`.
`PORTAL_TEST_DATABASE_URL` was supplied privately; no URL or keys are recorded.

| Command (from portal-rs unless noted) | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --locked --no-fail-fast` (`RUST_TEST_THREADS=4`) | PASS: 1,111 passed, 0 failed, 3 ignored across 82 test targets |
| `cargo test --locked -p oteryn-world --test catalog` | PASS: 1 PostgreSQL test, including reversible migration and no public activation |
| `cargo test --locked -p oteryn-world catalog::tests --lib` | PASS: 9 tests, including Game-produced negative/synthetic vectors and refusal cases |
| `cargo test --locked -p oteryn-stats world::tests --lib` | PASS: 4 tests |
| `node local.mjs` (from e2e) | PASS: migrated, booted, read inactive candidate, served real registry picker |
| `node run.mjs /tmp/portal-world-read-e2e-final/env.json test specs/world-read.spec.mjs specs/public.spec.mjs` (from e2e) | PASS: 12/12 across Polish/English, 1440px/390px |
| `python3 tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261107-portal-rs-world-read.md --require-checkpoint` (repo root) | PASS |
| `git diff --check` (repo root) | PASS |

Focused development checks were followed by the final full workspace run, which
also includes the strengthened escaped-NUL PostgreSQL regression.

The three ignored tests are existing opt-in browser unit tests. Affected real
browser paths are qualified separately below.

The first picker assertion expected `&lt;`; Askama correctly emits `&#60;`. That
test assertion was corrected and the affected tests passed. The superseded full
run was interrupted, then restarted on the final code. No product behavior was
changed to accommodate a test.

The first standalone public sweep failed four cases because the existing
registration-abuse policy defaults to disabled. The repository
`helpers.mjs::seedPasswordRange(password())` prepared its existing synthetic
registration fixture, as earlier account/identity specs normally do. No product
registration code or expected status was changed, and no world entities, visibility approvals, or Today snapshots were
seeded. The prepared sweep passed all 12 cases before the storage correction;
final-schema browser qualification passed all 12 cases as recorded above.

The task-owned harness processes were stopped, their disposable databases dropped,
the PostgreSQL container and its anonymous volume removed, and both harness
scratch directories (including generated certificates/keys and private env files)
deleted. Shared environment containers were left in their original stopped state.

## Remaining source blockers

- **Wiki home, search and entity lists: 503.** Every real export capability is
  unsupported/unknown and there are no entities: [input:9](../crates/world/tests/fixtures/catalog-d34807f5/unsupported-native-input.json#L9),
  [input:27](../crates/world/tests/fixtures/catalog-d34807f5/unsupported-native-input.json#L27).
  V1 generic `data` has no accepted per-capability payload contract; there are no
  approved native names, facts or relations to display. Publication additionally
  needs an authenticated origin/current epoch/profile gate:
  [contract:194](../crates/world/tests/fixtures/catalog-d34807f5/CONTRACT.md#L194),
  [contract:226](../crates/world/tests/fixtures/catalog-d34807f5/CONTRACT.md#L226).
- **Atlas: 503.** Needs a **Game-side atlas export**. At the owner-pinned Game SHA,
  [`content/world/assets/catalog.json:1`](https://github.com/Oteryn/Oteryn-Game/blob/d34807f5bd234ee8318e57ba454ba39be0df113d/content/world/assets/catalog.json#L1)
  has `assets:[]`; the v1 [capability vocabulary](../crates/world/tests/fixtures/catalog-d34807f5/CONTRACT.md#L54)
  contains no maps. Platform renders no substitute from raw terrain or B3 shards.
- **Atlas delivery: 503.** Item, creature, loot and quest export capabilities are
  unsupported; no active public native catalogue or accepted payload revisions.
- **Today data/widget: 503.** G-12 has no accepted Game producer and remains
  [PROPOSED](../../docs/contracts/OTERYN_V2_WORLD_DAILY_FAMILY_CONTRACT_PROPOSAL.md#L1).
  Gate, ingest rules and name-resolution rules are unchanged; only the harness UUID
  and inaccurate README status were fixed. No snapshots are fabricated.
- **World statistics with query: 503.** Existing unavailable privacy policy:
  [http.rs:27](../crates/stats/src/world/http.rs#L27). No-query picker is 200 when
  the Platform world registry is populated; it discloses no protected statistics.

G-2 visibility remains a Game blocker; existing admin `world_visibility` approvals
are not bypassed. Deaths, online, stats/guilds and wiki/places are out of scope and
retain their existing documented refusals.

## Changed files

- `docs/agents/tasks/active/OTERYN-20261107-portal-rs-world-read.md`
- `portal-rs/bin/oteryn-portal/src/main.rs`
- `portal-rs/crates/atlas/src/config.rs`
- `portal-rs/crates/atlas/src/delivery/http.rs`
- `portal-rs/crates/atlas/src/lib.rs`
- `portal-rs/crates/atlas/src/navigation.rs`
- `portal-rs/crates/atlas/src/spawns.rs`
- `portal-rs/crates/atlas/src/web.rs`
- `portal-rs/crates/community/src/today/README.md`
- `portal-rs/crates/stats/src/world/http.rs`
- `portal-rs/crates/stats/src/world/tests.rs`
- `portal-rs/crates/world/README.md`
- `portal-rs/crates/world/src/catalog/json.rs`
- `portal-rs/crates/world/src/catalog/mod.rs`
- `portal-rs/crates/world/src/catalog/tests.rs`
- `portal-rs/crates/world/src/lib.rs`
- `portal-rs/crates/world/tests/catalog.rs`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/CONTRACT.md`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/SOURCE.md`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/game-platform-catalog.json`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/game-platform-catalog.json.sha256`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/synthetic-control-character.json`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/synthetic-control-character.json.sha256`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/synthetic-supported.json`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/synthetic-supported.json.sha256`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/unsupported-native-input.json`
- `portal-rs/e2e/README.md`
- `portal-rs/e2e/WORLD_READ_REPORT.md`
- `portal-rs/e2e/local.mjs`
- `portal-rs/e2e/specs/public.spec.mjs`
- `portal-rs/e2e/specs/world-read.spec.mjs`
- `portal-rs/migrations/20261107120001_world_catalog_candidates.down.sql`
- `portal-rs/migrations/20261107120001_world_catalog_candidates.up.sql`
- `portal-rs/templates/stats/world/en.ftl`
- `portal-rs/templates/stats/world/index.html`
- `portal-rs/templates/stats/world/pl.ftl`
