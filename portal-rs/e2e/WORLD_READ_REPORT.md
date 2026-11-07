# World read — Update 5 review closeout

Branch: `codex/portal-rs-world-read`, predecessor `2a224efb`.
Scope is the owner's five review items. No PR, workflow edits or deployment.

## 1. Bounded recovery and stored readiness

Serve refresh remains off startup/readiness. Before producer I/O, a lookup checks
for a publication matching the configured snapshot, SHA-256, source revision,
authority epoch, ruleset and profile. A matching publication becomes ready without
refetch, even when its configured producer is unavailable. Unpublished candidates,
wrong hashes/fences, and publication OFF cannot borrow that readiness.

Transient `Unavailable` failures retry with exponential backoff and independent
75–100% jitter, including at the cap. Other validation/provenance/integrity/G-2
refusals stop immediately. Each whole attempt is capped at 60 seconds; URL
transport retains its 30-second deadline. Retry configuration:

| Variable | Default | Bounds |
| --- | --- | --- |
| `OTERYN_WORLD_CATALOG_RETRY_INITIAL_SECONDS` | 5 | 1–300 |
| `OTERYN_WORLD_CATALOG_RETRY_MAX_SECONDS` | 300 | initial–3600 |
| `OTERYN_WORLD_CATALOG_RETRY_MAX_ATTEMPTS` | 12 | 1–64, includes the first immediate attempt |

The finite attempt budget is intentional: exhaustion returns an error and leaves
the source closed. Operators can restart after an outage exceeding that budget.
Invalid policy configuration fails closed. Unit tests cover interval growth,
jitter and bounds. Real PostgreSQL/HTTP tests prove unavailable-then-available
recovery without restart, stored readiness with **zero** producer requests,
wrong-fence/hash/publication-OFF refusal, retry exhaustion and no integrity retry.

## 2. Loot publication-key collision

Artifact validation rejects a loot entity `content_key` equal to a loot relation
`relation_key`, before any persistence or SQL publication. The entire artifact
fails with `INVALID_INPUT: loot entity/relation publication key collision`.
A regression recomputes valid artifact digests to prove this semantic rejection;
equal keys mapping to different published kinds remain valid.

## 3. Safe publication rollback

Only the down migration changes; the applied up migration/checksum is unchanged.
While catalogue ownership metadata still exists, rollback removes catalogue
`world_entities`, catalogue-dependent puzzle answers/definitions, and the exact
catalogue-created `world_versions`. It also clears only catalogue references from
the legacy active/previous pointer. Legacy versions, entities, puzzle rows/answers,
identity approvals and legacy pointer values are preserved.

The existing puzzle immutability trigger is suspended only inside the rollback
transaction/table lock for those scoped deletions, then restored. The regression
populates both legacy and catalogue versions plus dependent puzzle answers, runs
the rollback, verifies only legacy rows survive, and proves the legacy puzzle
immutability guard still refuses deletion. This prevents orphaned ready catalogue
versions from being activated through legacy administration after metadata removal.

## 4. Identity-wide approval is intentional

The original world-store schema explicitly defines visibility independently of
versions and imported data. Approval is for stable `(kind, key)` identity, rather
than a review of a particular payload revision. Thus an exact legacy/canonical
native identity approval intentionally carries into the catalogue; rebuilding the
cache must not create another approval authority or silently revoke that decision.

The new PostgreSQL regression approves a legacy Item identity, publishes the
verified catalogue record for that same identity, verifies list/detail return the
new catalogue facts, then revokes through the existing service and verifies both
legacy and catalogue reads close. G-2, exact source fences and complete/presentable
categories remain mandatory. Omitted/hidden/unsupported data cannot be resurrected
by an old approval. This policy is explicit in
[CATALOG_WIKI_REQUIREMENTS.md](../crates/world/CATALOG_WIKI_REQUIREMENTS.md).

## 5. World picker remains required

Retained. The corrected brief explicitly requires a real picker/default view for
`/stats/world` without a query (scope item 4), and the coordinator's first update
says the picker is unchanged and still in scope. Update 5 changes no stats code.
The existing world-read E2E continues to require 200 for the registry picker and
503 for queried private metrics.

## Commands and results

Run from `portal-rs/` unless stated. Rust 1.97.0. Disposable PostgreSQL **18.6**
with `max_locks_per_transaction=1024`, `max_connections=300`, read back from server.
`PORTAL_TEST_DATABASE_URL` points only to that disposable local instance.

| Exact command | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --locked -p oteryn-world --test catalog --test catalog_refresh --lib` | PASS: 42 tests, PostgreSQL enabled |
| `RUST_TEST_THREADS=4 cargo test --locked --no-fail-fast` | PASS: 1,123 passed, 0 failed, 3 ignored across 83 targets |
| `node local.mjs` (e2e/) | PASS: disposable services started and `/readyz` returned 200 |
| `node run.mjs /tmp/portal-world-read-u5-e2e/env.json test specs/world-read.spec.mjs` (e2e/) | PASS: 8/8, PL/EN at 1440px/390px |
| `python3 tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261107-portal-rs-world-read.md --require-checkpoint` (root) | PASS during development |

Full Rust qualification runs before E2E services start, avoiding private listener
port collisions observed and corrected during the previous update. Required
checks are not skipped or weakened.

## Unchanged source gaps and refusals

Game pin is `669d5af5261a60d347167b7c880a4498aebe72ed`, Game `main` after the
CATALOG lane (#3025) merged; it replaces the earlier lane pin `fd4be4e2`.
No new Game access or artifact generation is needed by this
review update. The committed negative reference input and test-only supported
vectors remain unchanged.

- Wiki/search with merged Game `main` remain **503**: all eight native capabilities
  are explicitly unsupported/unknown and empty. Evidence:
  [contract notes:55–79](https://github.com/Oteryn/Oteryn-Game/blob/669d5af5261a60d347167b7c880a4498aebe72ed/docs/reference/CATALOG-CONTRACT-NOTES.md#L55).
- `/wiki/areas` remains **503**, pending the scheduled later additive `area`
  category; v1 has no such capability:
  [contract:54](https://github.com/Oteryn/Oteryn-Game/blob/669d5af5261a60d347167b7c880a4498aebe72ed/docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md#L54).
- Atlas/delivery remain **503**, requiring a Game-side atlas export; no Atlas
  code changes in Update 5.
- Today data/widget remain **503**, with G-12 still
  [PROPOSED](../../docs/contracts/OTERYN_V2_WORLD_DAILY_FAMILY_CONTRACT_PROPOSAL.md#L1)
  and no accepted producer. No Today changes or seeding.
- Queried world statistics retain the existing privacy refusal. The no-query
  registered-world picker remains 200.

Wiki capability scope remains `item`, `creature`, `npc`, `spell`, `quest`,
`achievement`, `creature_loot`; `npc_shop` has no current renderer. G-2 is resolved
and unchanged. Deaths, online, stats/guilds and wiki/places remain outside scope.

## Changed files and cleanup

Implementation: `crates/world/src/catalog/{mod.rs,refresh.rs}` and the existing
publication down migration. Tests: catalogue unit tests, cache/rollback integration
and new `tests/catalog_refresh.rs`. Documentation: world README, wiki requirements,
this report and the packet checkpoint. No stats or workflow source changes.

Cleanup complete: task portal/worker/producers stopped; disposable databases,
labelled PostgreSQL container and its anonymous volume removed; scratch
certificates, keys, auth directory and env file removed. Shared environment
containers remain in their original stopped state. Build caches and non-secret
validation logs are intentionally retained.

Exact Update 5 file inventory:

- `docs/agents/tasks/active/OTERYN-20261107-portal-rs-world-read.md`
- `portal-rs/crates/world/CATALOG_WIKI_REQUIREMENTS.md`
- `portal-rs/crates/world/README.md`
- `portal-rs/crates/world/src/catalog/mod.rs`
- `portal-rs/crates/world/src/catalog/refresh.rs`
- `portal-rs/crates/world/src/catalog/tests.rs`
- `portal-rs/crates/world/tests/catalog.rs`
- `portal-rs/crates/world/tests/catalog_refresh.rs`
- `portal-rs/e2e/WORLD_READ_REPORT.md`
- `portal-rs/migrations/20261107120003_world_catalog_publication.down.sql`
