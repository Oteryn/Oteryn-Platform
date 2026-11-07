# Production catalogue publication — resolved G-2 continuation

Branch `codex/portal-rs-world-read`; this continuation follows `6d6176db`.
No PR, workflow edit, deployment, Game write, raw-content fallback, atlas rendering,
Today producer, committed generated artifact or production test adapter.

## Source and G-2

The read-only Game lane `claude/happy-davinci-ijz3wy-catalog` resolves to
`fd4be4e214ef33ad65d3ce465dab7160377e62f5`.
Fixtures are marked **lane branch, re-pin on merge** in
[SOURCE.md](../crates/world/tests/fixtures/catalog-fd4be4e2/SOURCE.md).
Contract, negative source input and reference producer are byte-identical to the
prior pin. Lane notes are copied exactly; no generated artifact is committed.

G-2 is resolved. The producer omits hidden identities, hidden NPC shop relations
and secret achievements ([lane notes:67–74](../crates/world/tests/fixtures/catalog-fd4be4e2/CATALOG-CONTRACT-NOTES.md#L67)).
Normal exported records omit `visibility`. Explicit `visibility` accepts only the
string `public`; `hidden`, another string, null and other types reject the **whole
artifact**. This also fences nested payloads, relations and tombstones. Platform
never invents a visibility default. `world_visibility` remains independent.

The conditional request for a real supported positive fixture does **not** apply
at this revision: Game explicitly emits every native capability unsupported/unknown
and no entities/relations/tombstones ([lane notes:39–63](../crates/world/tests/fixtures/catalog-fd4be4e2/CATALOG-CONTRACT-NOTES.md#L39),
[implementation:70–80](https://github.com/Oteryn/Oteryn-Game/blob/fd4be4e214ef33ad65d3ce465dab7160377e62f5/crates/gameplay/content/src/content/project/public_view.rs#L70)).
Its internal public candidate view is not a supported export. No populated native
artifact or native positive browser result is fabricated. SOURCE gives the exact
negative reference generation command and byte hash.

## Production behavior

- Existing exact byte SHA-256, semantic digest, content revision and provenance
  fences, canonical wire/schema checks and resource limits remain mandatory.
- `OTERYN_WORLD_CATALOG_PUBLICATION_ENABLED=true` enables publication; default OFF.
  Verified cache records from `supported + complete` categories are projected
  transactionally and idempotently to derived `world_entities`. Loot relation
  carriers support the current generic loot record view; no drop formulas or
  endpoint graph UI are invented. `npc_shop` has no current wiki renderer.
- A dedicated wiki publication pointer keeps the legacy/map pointer untouched.
  Unsupported/partial/unknown category pages remain 503. Home/search aggregate
  complete categories and remain 503 when none exists. Categories lacking required
  wiki names or quest classification also refuse, without deriving names from keys.
  A complete empty category
  may return an empty 200; absent records cannot be restored by an old approval.
- Every public read joins `world_visibility`; intake/publication insert no allow
  rows. The existing staff `/admin/world` action now accepts native active records
  with its RBAC, MFA, CSRF and transactional audit checks. Native versions cannot
  use legacy/map publication or Platform-authored replacement names; those form
  controls are hidden and server-side actions refuse them.
- Serve starts refresh in a background task. Each process gates catalogue reads
  until its own refresh succeeds, even with an old persisted pointer. Missing,
  mismatched or unreachable configured sources cannot fall back to another native
  revision. `/healthz`, `/readyz` and listening sockets do not await the URL's
  30-second deadline. File I/O and CPU verification run on blocking workers.
- Migration `20261107120003_world_catalog_publication` adds only reversible
  `world_*` metadata/pointer tables. It does not change the existing migrations.

## Verification

Repo Rust 1.97.0. Disposable PostgreSQL **18.6**:
`max_locks_per_transaction=1024`, `max_connections=300` (read back from server).
Test database URL is task-local; no credentials/keys are recorded here.

| Exact command (from portal-rs unless stated) | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | PASS |
| `RUST_TEST_THREADS=4 cargo test --locked --no-fail-fast` with `PORTAL_TEST_DATABASE_URL` | PASS: 1,117 passed, 0 failed, 3 ignored across 82 targets |
| `cargo test --locked -p oteryn-world --test catalog` | PASS: transactional cache, rollback/repair/concurrency and all three migration pairs reversible |
| `cargo test --locked -p oteryn-world -p oteryn-wiki postgres_ --lib` | PASS during focused development: 9 tests, including production publication and audited staff gate |
| `node local.mjs` with pinned negative fixture | PASS: real binary/production publication switch, generated negative artifact in private scratch |
| `node run.mjs /tmp/portal-world-read-u4-qualified-e2e/env.json test specs/world-read.spec.mjs specs/public.spec.mjs` (e2e/) | PASS: 12/12, Polish/English at 1440px/390px |
| `node local.mjs` with `OTERYN_WORLD_CATALOG_URL=http://127.0.0.1:19998/catalog`, `PORTAL_E2E_CATALOG_EXPECT_UNAVAILABLE=1` and a test-only HTTP server delaying its response 35 seconds | PASS: `/healthz` 200 in 0.066s, `/readyz` 200 in 0.092s, `/en/wiki` 503 in 0.138s after the stalled artifact GET |
| `node run.mjs /tmp/portal-world-read-u4-slow-final/env.json test specs/world-read.spec.mjs` (e2e/) | PASS: 8/8, unavailable URL, publication ON, zero candidates/publication |
| `python3 tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261107-portal-rs-world-read.md --require-checkpoint` (root) | PASS |

The public sweep uses the existing `seedPasswordRange(password())` test helper
before standalone invocation, as documented in the E2E README. PostgreSQL tests
prove 200 for all seven current wiki categories using **test-only synthetic input
through the production publisher**, plus independent allow/deny, mixed partial
coverage, empty complete replacement, source mismatch, readiness gating,
idempotence and publication rollback. No alternate test publisher remains.

Intermediate checks caught duplicate fixture-module loading (centralized under
`cfg(test)`) and a test exceeding the real wiki rate limit (reset only the isolated
test schema's limiter between synthetic category cases). The first broad run also
had two binary startup failures while the E2E launcher occupied private listener
ports; default startup reported `Address already in use`. The launcher was stopped
and the final full Rust qualification ran in isolation, before browser services
were restarted. Final results above supersede those intermediate runs; product
limits and startup-test requirements are unchanged.

## Remaining refusals and exact Game gaps

| Pages | Expected status and blocker |
| --- | --- |
| Wiki home/search and current entity categories with the real pinned lane | **503**: all eight native capabilities remain unsupported/unknown and empty; owning source/payload promotion is still needed. [Lane notes:39–63](../crates/world/tests/fixtures/catalog-fd4be4e2/CATALOG-CONTRACT-NOTES.md#L39); [native_input:70–80](https://github.com/Oteryn/Oteryn-Game/blob/fd4be4e214ef33ad65d3ce465dab7160377e62f5/crates/gameplay/content/src/content/project/public_view.rs#L70). |
| `/wiki/areas` | **503**: owner schedules additive `area` in a later export revision. V1's closed vocabulary has no area capability: [contract:54](../crates/world/tests/fixtures/catalog-fd4be4e2/CONTRACT.md#L54). The served source also lacks Area declarations: [lane notes:53–63](../crates/world/tests/fixtures/catalog-fd4be4e2/CATALOG-CONTRACT-NOTES.md#L53). |
| `/atlas`, `/atlas/delivery` | **503**: needs a Game-side atlas export; this task does not render or activate maps. Asset evidence at the earlier merged source remains [`catalog.json:1`](https://github.com/Oteryn/Oteryn-Game/blob/d34807f5bd234ee8318e57ba454ba39be0df113d/content/world/assets/catalog.json#L1); v1 has no map capability. No new Atlas changes in this continuation. |
| `/today/data`, `/today/widget` | **503**: G-12 remains [PROPOSED](../../docs/contracts/OTERYN_V2_WORLD_DAILY_FAMILY_CONTRACT_PROPOSAL.md#L1), no accepted Game producer. No daily seeding added. |
| Queried `/stats/world` | **503**: existing privacy gate; unrelated to catalogue readiness. The no-query registered-world picker remains 200. |

Required wiki categories are exactly `item`, `creature`, `npc`, `spell`, `quest`,
`achievement`, `creature_loot`. `npc_shop` is separate from basic NPC names/lists;
`area` needs the later export revision. See
[CATALOG_WIKI_REQUIREMENTS.md](../crates/world/CATALOG_WIKI_REQUIREMENTS.md) for the
current renderer fields and scope. Deaths, online, stats/guilds and wiki/places
remain outside this task.

## Changed files in this continuation

Production: portal serve bootstrap; world catalogue verifier/cache/new publisher;
world admin service/template; wiki entity provider. Tests: catalogue unit/cache
integration, world admin and wiki PostgreSQL cases, pinned lane evidence directory,
E2E local source handling and world-read spec. Documentation: world README,
wiki requirements, E2E README/report and the active task packet. New reversible
migration: `20261107120003_world_catalog_publication.{up,down}.sql`.

Cleanup complete: task portal/worker/producers and slow URL server stopped;
disposable databases, labelled PostgreSQL container and its anonymous volume
removed; normal/slow scratch certificates, keys and env files removed. Shared
environment containers remain in their initial stopped state. Build caches and
non-secret validation logs are intentionally retained.

Exact file inventory relative to `6d6176db` (fixture renames shown at destination):

- `docs/agents/tasks/active/OTERYN-20261107-portal-rs-world-read.md`
- `portal-rs/bin/oteryn-portal/src/main.rs`
- `portal-rs/crates/wiki/src/entities.rs`
- `portal-rs/crates/wiki/src/entities/tests.rs`
- `portal-rs/crates/world/CATALOG_WIKI_REQUIREMENTS.md`
- `portal-rs/crates/world/README.md`
- `portal-rs/crates/world/src/admin.rs`
- `portal-rs/crates/world/src/admin_tests.rs`
- `portal-rs/crates/world/src/catalog/cache.rs`
- `portal-rs/crates/world/src/catalog/mod.rs`
- `portal-rs/crates/world/src/catalog/publication.rs`
- `portal-rs/crates/world/src/catalog/tests.rs`
- `portal-rs/crates/world/src/lib.rs`
- `portal-rs/crates/world/tests/catalog.rs`
- `portal-rs/crates/world/tests/fixtures/catalog-d34807f5/SOURCE.md`
- `portal-rs/crates/world/tests/fixtures/catalog-fd4be4e2/CATALOG-CONTRACT-NOTES.md`
- `portal-rs/crates/world/tests/fixtures/catalog-fd4be4e2/CONTRACT.md`
- `portal-rs/crates/world/tests/fixtures/catalog-fd4be4e2/SOURCE.md`
- `portal-rs/crates/world/tests/fixtures/catalog-fd4be4e2/unsupported-native-input.json`
- `portal-rs/crates/world/tests/support/catalog.rs`
- `portal-rs/e2e/README.md`
- `portal-rs/e2e/WORLD_READ_REPORT.md`
- `portal-rs/e2e/local.mjs`
- `portal-rs/e2e/specs/world-read.spec.mjs`
- `portal-rs/migrations/20261107120003_world_catalog_publication.down.sql`
- `portal-rs/migrations/20261107120003_world_catalog_publication.up.sql`
- `portal-rs/templates/admin/world/index.html`
