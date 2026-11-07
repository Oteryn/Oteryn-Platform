# Native catalogue consumer — Updates 2/3

Branch: `codex/portal-rs-world-read`; prior published checkpoint `faa77e57`.
No PR, deployment, workflow edits, raw Game content fallback, native adapter on
Platform, or Today producer. Atlas has no changes beyond the existing env aliases.

## Implemented

- Verify CATALOG_EXPORT_V1 canonical encoding, byte SHA-256, semantic digest,
  exact content/source revision and provenance fence, category support/completeness,
  stable identities, relation endpoints, tombstones and hard resource limits.
- Read exactly one configured read-only file or URL with an independently supplied
  expected SHA-256. HTTPS is required except on loopback. No redirects, proxy or
  ambient credentials; 30-second URL deadline and 256 MiB body bound. URL/path
  values are excluded from `Source` debug output.
- Retain the immutable verified artifact as exact bytes, and rebuild supported
  entity/relation/tombstone carrier records transactionally in `world_*` only.
  The candidate row serializes concurrent refreshes. Cached reads check complete
  capability coverage and detect missing/corrupted records. Rollback, concurrent
  refresh and repair have real PostgreSQL regression coverage.
- Unsupported, partial and unknown categories cannot pass a complete-capability
  read or the test publication setup. No production activation switch is exposed.
- A test-only Item publication setup consumes the real cache API and the Game
  reference producer's synthetic vector, proves wiki home/list/detail/search 200
  in both languages, and exercises independent admin allow/deny. It is compiled
  only under `cfg(test)`. The real unsupported fixture never activates.
- Generated artifacts/sidecars are removed from the tree. The harness generates
  only the pinned negative artifact in private scratch. Rust synthetic exports
  exist only in test code. Source pins and provenance are in
  [SOURCE.md](../crates/world/tests/fixtures/catalog-d34807f5/SOURCE.md).
- SQLx now watches migration directory additions. Seven older test bootstraps now
  use the production migration runner instead of executing down scripts as setup.

## Required G-2 contract input — not claimed complete

The owner accepted G-2 (`public|hidden`), but the exact export field path and
omission semantics are not present in the pinned merged v1 record schema. Both
branch/ref API reads of `claude/happy-davinci-ijz3wy-catalog` returned 404. An async
clarification requested the reachable producer/schema SHA and exact encoding.
No field location or default-public omission policy is invented. Production
publication remains closed, so no cached hidden or unsupported entry can become
public through an admin allow row. Explicit marked-entry G-2 decoding and its
positive/hidden/omitted-field regression must be completed against that source.

## Wiki adapter scope

Exactly seven existing v1 categories map to current wiki routes: `item`,
`creature`, `npc`, `spell`, `quest`, `achievement`, `creature_loot`.
`npc_shop` has no current wiki renderer and does not block a basic NPC name/list.
`/wiki/areas` has no v1 category and needs a contract addition. Detailed minimum
presentation fields, optional numeric facts/filters and source references are in
[CATALOG_WIKI_REQUIREMENTS.md](../crates/world/CATALOG_WIKI_REQUIREMENTS.md).
These are renderer requirements, not an invented Game payload contract.

## Validation

PostgreSQL 18.6, `max_locks_per_transaction=1024`, `max_connections=300`;
private test URL, no credentials recorded. Repo Rust toolchain 1.97.0.

| Command (portal-rs unless noted) | Current result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --locked --no-fail-fast` (`RUST_TEST_THREADS=4`) | PASS: 1,115 passed, 0 failed, 3 ignored (82 test targets) |
| `cargo test --locked --workspace --test catalog` | PASS during focused development; full run covers strengthened concurrency test |
| `cargo test --locked --workspace postgres_native_export_cache --lib` | PASS: synthetic supported export -> cache -> wiki/search 200; real unsupported fixture refuses |
| `cargo test --locked --workspace catalog::tests --lib` | PASS: 12 catalogue tests plus other matching workspace tests |
| `node local.mjs` | PASS: scratch-generated pinned negative export; no committed artifact |
| `node run.mjs /tmp/portal-world-read-u3-e2e/env.json test specs/world-read.spec.mjs specs/public.spec.mjs` | PASS: 12/12, Polish/English at 1440px/390px |
| `python3 tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261107-portal-rs-world-read.md --require-checkpoint` (root) | PASS during development |

Intermediate validation found a test-helper lint, full disk from superseded
symbol-heavy test executables, and old fixtures executing down migrations. Test
lints/bootstrap were corrected. Only task-log-referenced, task-built, inactive
ELF executables were removed; source, libraries and shared services were retained.
Final verification is repeated rather than treating those failed runs as passes.

## Pages that remain 503

- Wiki entities/home/search: the merged reference input has all categories
  unsupported/unknown and zero entities ([input:9](../crates/world/tests/fixtures/catalog-d34807f5/unsupported-native-input.json#L9)).
  The CATALOG native adapter is not merged; accepted per-category payload/G-2
  export mapping and publication evidence are not available at this pin.
- Atlas: needs a Game-side atlas export; the owner schedules its Rust rewrite at
  programme end. No substitute rendering is performed.
- Atlas delivery: item/creature/loot/quest capabilities and placements are not
  available as an active public native catalogue.
- Today data/widget: G-12 is still [PROPOSED](../../docs/contracts/OTERYN_V2_WORLD_DAILY_FAMILY_CONTRACT_PROPOSAL.md#L1),
  with no accepted producer. No synthetic daily publication is created.
- Queried world statistics: existing privacy gate. The no-query registry picker
  is 200 when registered worlds exist; empty/unavailable registry still refuses.

Deaths, online, stats/guilds and wiki/places remain outside this task.

Task-owned portal/worker/producers, disposable databases, PostgreSQL container
and anonymous volume, scratch certificates/keys and private env file were cleaned
up. Shared environment containers remained in their initial stopped state.

## Changed files


- `docs/agents/tasks/active/OTERYN-20261107-portal-rs-world-read.md`
- `portal-rs/Cargo.lock`
- `portal-rs/bin/oteryn-portal/src/main.rs`
- `portal-rs/crates/admin/src/commerce/tests.rs`
- `portal-rs/crates/admin/src/console/dashboard_tests.rs`
- `portal-rs/crates/admin/src/web_tests.rs`
- `portal-rs/crates/atlas/src/config.rs`
- `portal-rs/crates/atlas/src/delivery/http.rs`
- `portal-rs/crates/atlas/src/lib.rs`
- `portal-rs/crates/atlas/src/navigation.rs`
- `portal-rs/crates/atlas/src/spawns.rs`
- `portal-rs/crates/atlas/src/web.rs`
- `portal-rs/crates/community/src/today/README.md`
- `portal-rs/crates/content/src/announcements_tests.rs`
- `portal-rs/crates/content/src/events/tests.rs`
- `portal-rs/crates/content/src/media_tests.rs`
- `portal-rs/crates/content/src/news_tests.rs`
- `portal-rs/crates/db/build.rs`
- `portal-rs/crates/stats/src/world/http.rs`
- `portal-rs/crates/stats/src/world/tests.rs`
- `portal-rs/crates/wiki/Cargo.toml`
- `portal-rs/crates/wiki/src/entities/tests.rs`
- `portal-rs/crates/world/CATALOG_WIKI_REQUIREMENTS.md`
- `portal-rs/crates/world/Cargo.toml`
- `portal-rs/crates/world/README.md`
- `portal-rs/crates/world/src/catalog/cache.rs`
- `portal-rs/crates/world/src/catalog/json.rs`
- `portal-rs/crates/world/src/catalog/mod.rs`
- `portal-rs/crates/world/src/catalog/tests.rs`
- `portal-rs/crates/world/src/catalog/transport.rs`
- `portal-rs/crates/world/src/lib.rs`
- `portal-rs/crates/world/tests/catalog.rs`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/CONTRACT.md`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/SOURCE.md`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/unsupported-native-input.json`
- `portal-rs/crates/world/tests/support/catalog.rs`
- `portal-rs/e2e/README.md`
- `portal-rs/e2e/WORLD_READ_REPORT.md`
- `portal-rs/e2e/local.mjs`
- `portal-rs/e2e/specs/public.spec.mjs`
- `portal-rs/e2e/specs/world-read.spec.mjs`
- `portal-rs/e2e/support/catalog-producer.py`
- `portal-rs/migrations/20261107120001_world_catalog_candidates.down.sql`
- `portal-rs/migrations/20261107120001_world_catalog_candidates.up.sql`
- `portal-rs/migrations/20261107120002_world_catalog_records.down.sql`
- `portal-rs/migrations/20261107120002_world_catalog_records.up.sql`
- `portal-rs/templates/stats/world/en.ftl`
- `portal-rs/templates/stats/world/index.html`
- `portal-rs/templates/stats/world/pl.ftl`
