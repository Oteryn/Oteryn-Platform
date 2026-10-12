# Platform PostgreSQL migration plan

- Status: plan. It authorizes no packet by itself; each packet below needs its own task record, PR and evidence.
- Decision: ADR 0046 (`docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md`)
- Governing Issue: #1474
- Input: `docs/architecture/reviews/OTERYN_PLATFORM_ARCHITECTURE_AND_DATABASE_REVIEW_2026-10-12.md` (PR #1560), findings F-01, F-02, F-04, F-05, F-06, F-12 and F-13
- Baseline: `main` at `b2e86fae` on 2026-10-12
- Owner answers: Q1 to Q6 answered on 2026-10-12 (§6)

## 1. Scope and fixed rules

The plan moves the Platform's own database, `oteryn_platform`, from MariaDB 11.8 to PostgreSQL 17. It runs in a Platform-owned instance that is separate from Game's instance (ADR 0046 §4).

These rules hold in every packet:

- Game-ADR-0004 invariants: separate logical databases (`oteryn_platform`, `oteryn_game`), roles, credentials and migration histories, and no shared-table writers. No packet reads or writes the Game database.
- The repository holds no secret, certificate, private key, password, dump or backup. Packets commit templates with placeholders only. Section 5 lists what the owner or operator supplies.
- Packets run in the order P1 to P7. P1 stage A can run alongside P2. Every other packet starts only after its predecessors have merged.
- From P1 stage B until P7, code and migrations must pass on both engines.
- The two legacy-engine `mysql` connections in `config/database.php` are out of scope. They retire under #1474. `pdo_mysql` stays in the images until then.
- No data is carried over. The Game node is not deployed anywhere and all staging data is test data, so PostgreSQL starts from a clean slate and the native-evidence witness directory is reset together with the database (P3).
- MariaDB stays deployed and authoritative until the P6 cutover. Rollback, and a clean-slate re-run of P3, are possible only before the first witnessed write on PostgreSQL. That write comes at the start of P6 step 4. First the P6a provenance-only init creates the witness-store id, without advancing any floor. Then the first `game-auth:native-admission:publish-trust` attempt runs, and trust publication advances the retained trust-state witness before it writes to the database. From that init on, the PostgreSQL witness is kept and state is only reconciled forward. The MariaDB volume is deleted right after a successful cutover smoke test, with no retention window and no final dump.
- Staging backups stay on the NAS, with provisional RPO 24 hours and RTO 4 hours. An encrypted off-NAS copy is required before any production or real-player use (ADR 0046 §7).
- `docs/agents/CI_WORKFLOW_LIFECYCLE.json` has 53 registered workflows against a budget of 53. Packets extend existing workflows and do not add new ones.

## 2. Baseline inventory

| Area | What depends on MariaDB today | Evidence |
|---|---|---|
| Schema | 68 migrations creating 104 tables, including 289 unsigned, `foreignId` or `increments` columns, 97 unique indexes, 27 `json` columns, 4 `enum` columns, 10 `->change()` and 22 `->after()` | `database/migrations/` |
| Driver-gated code | `innodb_lock_wait_timeout` session handling; `getDriverName() === 'mysql'` branches; the ADR 0044 store identity (`ALLOWED_HOSTS = ['mariadb']`, driver `mysql`); `LOCK TABLES` in one migration; an InnoDB snapshot-isolation assumption | `app/GameAuth/NativeLogin/NativeAdmissionAttempts.php:125-141`, `app/GameAuth/Worlds/DisposableNativeStore.php`, `app/GameAuth/Worlds/PreproductionNativeRouteStore.php:26,86`, `database/migrations/2026_10_08_090000_add_native_ids_to_character_bazaar_and_profile_preferences.php:42-53`, `app/GameAuth/NativeAccountCharacters/NativeAccountCharactersReadModel.php:56-58` |
| Error handling | About 20 files catch `QueryException` or vendor error codes | `rg -l "QueryException\|1062\|1205\|1213\|40001" app` |
| Raw SQL and text matching | 18 files with raw SQL; about 45 `LIKE`/`LOWER(` sites that rely on `utf8mb4_unicode_ci`, which is case- and accent-insensitive | `rg -l "DB::raw\|whereRaw\|selectRaw\|orderByRaw" app database`; `rg -n "LOWER\(\| like " app` |
| `insertOrIgnore` | 5 app call sites and 10 migration call sites | `rg -n insertOrIgnore app database` |
| Config | `mysql` connection, default `sqlite` when `DB_CONNECTION` is unset | `config/database.php` |
| Image | `docker-php-ext-install ... pdo_mysql` with no `pdo_pgsql` | `deploy/synology/docker/platform.Dockerfile:24` |
| Deploy | `mariadb` service, `mariadb_data` volume and an init script that grants `ALL PRIVILEGES` to the runtime user; readiness, dump, restore, preflight and health logic call `mariadb`/`mariadb-dump` as `root`; migrations run inside the runtime container | `deploy/synology/compose.yml`, `deploy/synology/mariadb/init/10-platform-database.sh:27`, `deploy/synology/scripts/{deploy,lib,health-check,deploy-impact,recover-schema,prepare-fresh-schema-baseline,production-target-preflight,marketplace-staging,rollback}.sh`, `deploy.sh:765` |
| Tests | Default suite on in-memory SQLite (`phpunit.xml:26`). Real-MariaDB suites cover payments, game tickets, topology and legacy transfer. There are no wallet or bid concurrency tests on a real database (F-05). | `tests/Feature/**/*MariaDbTest.php`, `.github/workflows/game-auth-ticket-concurrency.yml` |
| CI | 22 workflows reference `mariadb` (service containers, Synology operations) | `rg -l mariadb .github/workflows` |
| Docs | 48 non-archived Markdown files under `docs/` mention MariaDB | `rg -il mariadb docs` (excluding `tasks/archive` and `evidence/`) |

## 3. Packet overview

| Packet | Title | Depends on | Exit gate | Owner or operator input |
|---|---|---|---|---|
| P1 | CI probe on PostgreSQL and real concurrency tests (F-05) | none | Stage A: non-blocking job reports. Stage B: job green and blocking. | Branch-ruleset edit to add the required check (admin) |
| P2 | Code compatibility sweep | P1 stage A | Full suite green on PostgreSQL and MariaDB | none |
| P3 | Clean-slate bootstrap on PostgreSQL | P2 | Scripted bootstrap from an empty database and empty witness directory passes in CI, refusal paths tested | none |
| P4 | Synology Compose: PostgreSQL service, TLS and role split (F-06) | P2, P3 | PostgreSQL running idle beside MariaDB; role and TLS checks pass; ADR 0044 amendment accepted | Role secrets, ADR 0044 amendment acceptance |
| P5 | Backup and PITR on the NAS: base backup, WAL archive and restore drill (F-01) | P4 | Archiving active, one dated restore drill passed against provisional RPO 24 h and RTO 4 h | NAS space for the spool |
| P6 | Staging cutover | P1 stage B, P3, P4, P5 | Platform on PostgreSQL; smoke tests passed; cutover declared done; MariaDB volume deleted; dated evidence | Go decision, maintenance window, new WorldId and OAuth client ids |
| P7 | Remove MariaDB code paths and reconcile docs | P6 declared done | No Platform-DB MariaDB path left; docs and registries updated | Removal of the retired MariaDB secrets |

## 4. Packets

### P1 — CI probe on PostgreSQL and real concurrency tests

**Goal.** Run the full Platform test suite against a `postgres:17` service container, first as a report and then as a blocking check. Close F-05 with real multi-process concurrency tests for the wallet and Bazaar bids on PostgreSQL.

**Owned paths.**

- `.github/workflows/ci.yml`: a new `platform-postgres` job, extending the existing workflow because the workflow budget is full
- `phpunit.pgsql.xml` (new; the same suites as `phpunit.xml` with `DB_CONNECTION=pgsql`)
- `config/database.php`, adding a `pgsql` connection only: host, port, database, username and password from the environment, `sslmode`, `sslrootcert`, `search_path=public`, timezone UTC
- `tests/Feature/Wallet/Concurrency/**` (new)
- `tests/Feature/Marketplace/Concurrency/**` (new files only)
- `tests/Support/Concurrency/**` (new multi-process harness, only if no reusable one exists)
- `docs/agents/BUILD_TEST_MATRIX.md`, `docs/architecture/TEST_STRATEGY.md`: the PostgreSQL row

**Work.**

1. Stage A, reporting. The job runs the whole suite on PostgreSQL with `continue-on-error: true` and uploads the failure list as an artifact. The list is P2's work queue. It is not a required check yet.
2. Concurrency tests (F-05). Each test runs N=8 separate PHP processes against one PostgreSQL database. They cover:
   - `WalletMutator` credit and debit races: no lost update, no negative balance, and the ledger sum equals the balance for every account;
   - `PlaceAuctionBid` races: exactly one leading bid, and reservations released exactly once;
   - auction settlement against a concurrent bid or cancel: a single terminal state and a ledger that stays consistent.
   Each test also runs on the existing MariaDB concurrency job while MariaDB is deployed.
3. Stage B, blocking. When P2 is green, remove `continue-on-error`. The owner adds the job to the branch ruleset's required checks.

**Validation.**

- Stage A job runs and reports.
- Each new concurrency test fails when its `lockForUpdate` is removed locally. Record that negative check in the task evidence; do not commit it.
- Stage B: job green on the P2 head and on `main`.
- Existing jobs are unchanged.

**Rollback.** Stage A has no merge effect, so reverting the job is enough. For stage B, the owner removes the required check and the job is reverted. Nothing is deployed.

### P2 — Code compatibility sweep

**Goal.** Make the application, migrations and images correct on PostgreSQL without changing behaviour on MariaDB.

**Owned paths.** These are the exact files from the P1 stage A failure list and the inventory in §2, within:

- `app/**`
- `database/migrations/**`
- `config/database.php`
- `deploy/synology/docker/platform.Dockerfile`
- `scripts/acceptance/docker/**`
- `.env.example`
- the `tests/**` files that fail
- `phpstan.neon.dist` only if it is affected

A PR in this packet that touches `deploy/synology/compose.yml` or `deploy/synology/scripts/**` is out of scope; that work belongs to P4.

**Rules for migrations.** All 68 migrations are already applied on staging MariaDB. A change to an existing migration must leave the MariaDB result identical, either through a driver branch or by being a no-op on MariaDB. Corrections that only add something, such as a `CHECK` constraint or an expression index, go into new forward migrations that run on both engines. Migrations are not squashed or rebaselined. PostgreSQL replays the same 68 migrations from an empty database (P3).

**Checklist.** Each item is closed with a test or with a recorded reason it does not apply.

1. **Driver branches.** In `NativeAdmissionAttempts`, replace `SET SESSION innodb_lock_wait_timeout` with `SET LOCAL lock_timeout` inside the transaction on `pgsql`, keeping the same bound. Review every `getDriverName()` branch, and the `LOCK TABLES` branch in migration `2026_10_08_090000`; on PostgreSQL use `LOCK TABLE ... IN ACCESS EXCLUSIVE MODE` inside the migration transaction. The `PreproductionNativeRouteStore` driver and host change waits for the ADR 0044 amendment in P4.
2. **Isolation.** InnoDB defaults to REPEATABLE READ and PostgreSQL to READ COMMITTED. List every transaction that reads the same rows more than once without a lock and relies on one snapshot. `NativeAccountCharactersReadModel:56-58` is the known case. Fix each with explicit row locks, or with `REPEATABLE READ` plus a retry on SQLSTATE `40001`. Keep READ COMMITTED as the default.
3. **Error codes.** Map MySQL `1062`, `1205`, `1213` and `1020` to SQLSTATE `23505`, `55P03`, `40P01` and `40001`. Prefer Laravel's `UniqueConstraintViolationException` and its concurrency-error detection over raw vendor codes.
4. **No `UNSIGNED`.** PostgreSQL has no unsigned types. For every value column whose non-negativity is an invariant (balances, amounts, counters, revisions, `chance_numerator`/`chance_denominator`), add `CHECK (col >= 0)` in a forward migration. MariaDB 11.8 enforces `CHECK` too. Plain foreign keys need nothing. Laravel's `unsignedInteger` becomes `integer` on PostgreSQL, whose maximum is 2^31-1; confirm that no column's documented range exceeds it.
5. **Case and accent rules.** `utf8mb4_unicode_ci` makes `=`, `LIKE` and unique indexes case-insensitive and largely accent-insensitive.
   - `identities.email` is canonicalized by `app/Identity/Support/CanonicalEmail.php`. Prove that every write path goes through it, and add a unique index on `lower(email)` as a safety net.
   - For the other unique `slug`/`key`/`code` columns, decide case rules per column.
   - Search `LIKE` sites become `ILIKE` or `LOWER(col) LIKE LOWER(?)`.
   - Search becomes accent-sensitive on PostgreSQL, which matters for Polish content. The owner accepted this and no `unaccent` extension is added (owner answer Q6). Record it in `FEDERATED_SEARCH_ARCHITECTURE.md`.
6. **`enum`.** The 4 `enum` columns become `varchar` with a `CHECK` constraint. Changing the allowed values later means replacing the constraint.
7. **JSON.** The 27 `json` columns become PostgreSQL `json`, which has no equality operator. Remove `=`, `DISTINCT`, `GROUP BY` and unique use on JSON columns. Never use `jsonb` for stored bytes that are signed, hashed or replayed verbatim; they stay `text` (for example `native_game_evidence_observations.response_json`).
8. **Booleans.** `tinyint(1)` becomes `boolean`. Raw `= 1` and `= 0` in the 18 raw-SQL files become bound `true`/`false`.
9. **`insertOrIgnore`.** On PostgreSQL it becomes `ON CONFLICT DO NOTHING`, which ignores only unique conflicts, whereas MySQL `IGNORE` also downgrades other errors. Confirm each caller depends only on uniqueness.
10. **Ordering.** `NULL` sorts first in ascending order on MariaDB and last on PostgreSQL. Add explicit `NULLS FIRST`/`NULLS LAST` where pagination or UI order depends on it.
11. **Identifiers and keys.** Bigint auto-increment surrogates stay as identity columns. ADR 0028 keeps `identities.id` local, with `AccountId` as a UUIDv7 column. `uuid` columns become native `uuid`. ULIDs stay `char(26)`. Code must not assume ids are gap-free or that id order matches insertion order across sessions. Laravel-generated index names longer than 63 bytes are truncated on PostgreSQL; check them for collisions.
12. **Transactional DDL.** On PostgreSQL each migration runs in a transaction. Migrations that combine DDL with long data rewrites or explicit locks must still succeed inside one transaction.
13. **Time.** Session timezone UTC on both engines. Laravel `timestamp` becomes `timestamp(0) without time zone`. Columns where sub-second ordering matters keep explicit precision.
14. **Configuration.**
    - Add a `pgsql_migrations` connection for the migration owner role (P4 supplies the credentials).
    - Make `production:verify-configuration` accept `pgsql`; it keeps refusing `sqlite` (F-13).
    - `.env.example` gets placeholders only.
    - Session, cache and queue stay on `file`/`sync`; no database driver is added.
15. **Images.** Add `libpq` and `pdo_pgsql` to `platform.Dockerfile` and the acceptance image, and `pdo_pgsql` to the `setup-php` extension lists. Keep `pdo_mysql`.
16. **Test names.** Tests named `*MariaDbTest` become engine-parameterized where they test Platform behaviour. Tests of the legacy-engine connection stay MariaDB-only until #1474 retires them.

**Validation.**

- The P1 PostgreSQL job runs the full suite green.
- All existing MariaDB and SQLite jobs stay green.
- `migrate:fresh` followed by `migrate:rollback` succeeds on both engines in CI.
- PHPStan passes.
- P1 stage B is enabled at the end of this packet.

**Rollback.** Revert the PR. Every change is dual-engine, so the deployed MariaDB path is unaffected. Forward `CHECK` migrations have guarded `down()` methods.

### P3 — Clean-slate bootstrap on PostgreSQL

**Goal.** Make a fresh PostgreSQL database plus an empty native-evidence witness directory reach a working staging Platform with one repeatable, scripted procedure. No data is carried over from MariaDB (owner answer Q1).

**Why a clean slate is safe now.** The coupling that would make copying necessary does not exist yet:

- The Game node is not deployed anywhere, so no Game state refers to Platform `AccountId`s, World or Channel ids, OAuth client ids or operation receipts (ADR 0028).
- All staging data is test data. Testers register again.
- The native evidence witness (`OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md`) fails closed when a database binding exists and its directory is empty, or when relational history is below the retained floor. A fresh database together with an empty directory is the supported bootstrap case, at revision 1. The directory and the database are therefore reset together, never one without the other.

**Bootstrap sequence.** Most steps already run in every staging deploy (`deploy/synology/scripts/deploy.sh:765-775`, `lib.sh`) and are idempotent. On an empty database they create the state from scratch:

1. **Witness reset.** With the `platform` service stopped, move the directory named by `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY` (on the `platform_storage` volume) aside to a dated name. It is never edited, and P6 deletes it together with the MariaDB volume. The script records the MariaDB-era `witness-store.id` at the freeze and moves aside only a directory that is empty or carries that id. It refuses when the directory holds anything a PostgreSQL witness wrote, including a store id alone from the P6a provenance-only init (which runs only in P6 step 4, after the last rollback point), or when the target database has any `native_game_evidence_witness_stores`, `native_game_signing_trust_*` or native evidence row. A trust publication attempt can advance the witness even when its database transaction then rolls back. That leaves the missing-profile state: a trust-state floor of 1 with no profile row and no database trace. So the directory check matters as much as the database check. This plan has no automated repair for that state (P6 step 4), and it is never handled by a re-run. Together they make a re-run impossible after the first witnessed write. The script also refuses when the new directory is not empty after the move.
2. **Schema.** `php artisan migrate --force` on the empty database, as the owner role on the `pgsql_migrations` connection (P4). Migrations also seed the built-in roles and permissions; there is no separate seeder.
3. **Passport keys.** `passport:keys` runs only when `storage/oauth-*.key` are missing. Existing keys on `platform_storage` are kept.
4. **Native admission trust.** `game-auth:native-admission:publish-trust` publishes the existing key from the `oteryn-native-admission-keys` volume into the new database, followed by `game-auth:native-admission:self-check`. The key volume is kept. This is the first witnessed write. It runs only when native admission is on, so in P6 it is deferred to P6 step 4, after the last rollback point.
5. **OAuth clients.** `game-auth:oauth-client:ensure` and `game-auth:oauth-client:ensure --rust` register new clients. The new client ids are not secrets. The operator updates every consumer that pins the old ids (`docs/operations/NATIVE_LOGIN_OWNER_STEPS.md`).
6. **World and route.** `game-auth:world:ensure`, then `game-auth:native-topology:issue`, which mints a new WorldId. The operator sets `OTERYN_STAGING_NATIVE_WORLD_ID` to it, and the deploy refuses until it matches (`lib.sh:924-927`). `game-auth:native-route:publish` then re-publishes the route records. Route publication on PostgreSQL waits for the ADR 0044 amendment (P4).
7. **Administration and content.** `admin:bootstrap` creates the first administrator from operator-supplied values. `wiki:launch-content:install` and the existing acceptance seed scripts are optional.
8. **Checks.** `production:verify-configuration`, `game-auth:native-admission:self-check` when native admission is on, and `health-check.sh`. `game-auth:native-evidence:reconcile` is a recovery command, not a routine check.

**Owned paths.**

- `deploy/synology/scripts/bootstrap-postgres.sh` (new). It runs steps 1 to 8 in order against the PostgreSQL engine, refuses on a non-empty target database, and prints the new non-secret ids (OAuth clients, WorldId) for the operator.
- `deploy/synology/tests/**`, `tests/ci/test_synology_*.py`: tests for the refusal paths (non-empty database, witness binding present, non-empty directory after the move)
- `docs/operations/PLATFORM_POSTGRES_BOOTSTRAP.md` (new runbook: the sequence above, the operator variable updates and the expected outputs)

**Optional fallback, not a required step.** If the owner later needs to keep staging data, a table-copy tool can be added as a separate packet: an artisan command that copies MariaDB tables to PostgreSQL in primary-key order with foreign keys enforced, refuses unclassified tables, and verifies row counts, per-table SHA-256 over canonical rows, wallet ledger sums and the evidence `source_revision` maxima. Because Game is not deployed and the data is test data, this plan does not build it. Its design notes stay in this paragraph only.

**Validation.**

- CI: run the bootstrap script against an empty PostgreSQL service container and an empty witness directory, then `health-check.sh`-equivalent checks and the native admission self-check.
- Negative tests: the script refuses on a database with an existing witness binding or trust row, on a witness directory carrying a store id other than the recorded MariaDB-era one, and on a directory a failed trust publication advanced without a database row (the missing-profile state).
- A rerun on a bootstrapped database is refused, not partially applied.

**Rollback.** Revert the PR. The script only acts on the PostgreSQL engine, which is idle until P6. Nothing deployed changes.

### P4 — Synology Compose: PostgreSQL service, TLS and role split

**Goal.** Add a Platform-owned `postgres:17` service beside MariaDB, selectable by one engine switch that stays on MariaDB. Add TLS, a healthcheck, its own volume and separate principals. Move the migration step to the owner role (F-06).

**Owned paths.**

- `deploy/synology/compose.yml`, `deploy/synology/compose.marketplace.yml`
- `deploy/synology/postgres/**` (new: `init/` role and database bootstrap, `postgresql.conf`, `pg_hba.conf`)
- `deploy/synology/tls/**`: a PostgreSQL server certificate issued by the stack CA in `tls-init`
- `deploy/synology/scripts/{deploy,lib,health-check,deploy-impact,recover-schema,prepare-fresh-schema-baseline,production-target-preflight,marketplace-staging,rollback}.sh`
- `deploy/synology/release-contract.env`
- `deploy/synology/README.md`, `deploy/synology/tests/**`, `tests/ci/test_synology_*.py`
- DB-service references only, in `.github/workflows/{deploy-synology-staging,recover-synology-staging-schema,synology-production-target-preflight,synology-diagnostics,synology-container-hygiene,repair-synology-autostart,repair-synology-compose-orphans}.yml`
- `app/GameAuth/Worlds/PreproductionNativeRouteStore.php` and its tests
- an amendment to `docs/architecture/adr/0044-preproduction-store-native-route-publication.md` (owner acceptance required)

**Work.**

1. **Service.**
   - Image `postgres:17.x`, pinned by digest. Volume `postgres_data`.
   - Private network only, with no published host port, so nothing collides with Game's instance on the NAS.
   - Healthcheck: `pg_isready` plus a TLS-only login probe by the runtime role.
   - `restart: always`.
2. **TLS.**
   - `ssl=on`, TLS 1.3 minimum, with the server certificate signed by the stack CA from `tls-init` (owner answer Q4). No operator-supplied certificate.
   - `pg_hba.conf` has only `hostssl ... scram-sha-256` lines plus local socket access for bootstrap; `host` without SSL is rejected.
   - Platform connects with `sslmode=verify-full` and the CA mounted read-only.
3. **Roles.** The init script is idempotent and refuses unsafe names, like the MariaDB one.
   - `oteryn_platform_owner`: `LOGIN`, owns database `oteryn_platform` and schema `public`.
   - `oteryn_platform_app`: `LOGIN`, `CONNECT` and `USAGE` only; DML on all tables and `USAGE`/`SELECT` on sequences through `ALTER DEFAULT PRIVILEGES FOR ROLE oteryn_platform_owner`; no `CREATE`, not an owner.
   - `oteryn_platform_backup`: `LOGIN REPLICATION`, used by P5 for `pg_basebackup` only. A replication role cannot read tables, so it is never used for `pg_dump`.
   - `oteryn_platform_dump`: `LOGIN`, read-only, used for the deploy's pre-migration logical dump. It gets `CONNECT` on `oteryn_platform` only, `USAGE` on schema `public`, `SELECT` on all current tables and sequences, and `SELECT` on future ones through `ALTER DEFAULT PRIVILEGES FOR ROLE oteryn_platform_owner`. It gets no DML, no `CREATE`, no `pg_read_all_data` and no access to other databases.
   - `REVOKE ALL ON DATABASE oteryn_platform FROM PUBLIC`, and `REVOKE CREATE ON SCHEMA public FROM PUBLIC`.
   - The superuser password is used only by the init step and never reaches the `platform` service.
4. **Migration step.** Replace `compose exec -T platform php artisan migrate` (`deploy.sh:765`, `marketplace-staging.sh:196`) with a one-off `compose run --rm` container on the `pgsql_migrations` connection that receives the owner credentials. The runtime `platform` container gets only the app role.
5. **Engine switch.**
   - `PLATFORM_DB_ENGINE=mariadb|postgres` in the release contract, default `mariadb`. Deploy scripts branch on it for readiness, pre-migration dump (`pg_dump -Fc` as `oteryn_platform_dump`, then validated with `pg_restore --list` and, in the P4 validation, a full restore into a scratch database), restore, preflight and health.
   - With `mariadb`, PostgreSQL runs idle and empty except for roles. Only P6 flips the switch.
6. **Volume guard.** The hygiene and orphan-repair workflows must never remove `mariadb_data`, `postgres_data` or the `platform_storage` volume that holds the witness directory. Add a test that proves it. Deleting `mariadb_data` is an explicit P6 operator step, never a side effect of a workflow.
7. **ADR 0044 amendment.** The single allowlisted preproduction store identity becomes driver `pgsql`, host `postgres`, port `5432`, database `oteryn_platform`, taking effect with the engine switch. Until the owner accepts it, `OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED` stays `false` on the PostgreSQL engine.

**Validation.**

- `docker compose config` renders for both engine values.
- The deploy contract tests in `deploy/synology/tests` and `tests/ci` pass.
- Staging deploy with `mariadb` is unchanged and the PostgreSQL service is healthy.
- A non-TLS connection is refused; `verify-full` succeeds.
- The runtime role gets `permission denied` for `CREATE TABLE`, `ALTER TABLE` and `DROP TABLE`, and owns no relation.
- `docker inspect platform` shows no owner or superuser password.
- A migration run as the owner on an empty database matches the P2 schema.
- A real pre-migration dump by `oteryn_platform_dump` succeeds after a migration that adds a table, proving the default privileges, and restores into a scratch database with matching row counts. The dump role is refused on `INSERT`, `UPDATE`, `DELETE` and `CREATE`.

**Rollback.** Revert the PR. The idle PostgreSQL service and its volume can be removed by the operator. MariaDB never changed.

### P5 — Backup and PITR on the NAS

**Goal.** Close F-01 for the target engine on staging: continuous WAL archiving, scheduled base backups and a dated restore drill, all on the NAS (owner answer Q5). Staging targets are provisional: RPO 24 hours and RTO 4 hours. They do not block staging, and the owner sets final values before production (review decision D-2).

**Required before production or real players.** The off-NAS part of F-02 is deferred, not dropped. Before any production use, any real player, wallet or native Bazaar enablement with real value, or any payment activation, a later packet must add an encrypted off-NAS copy of the `pg_basebackup` base backups and the `archive_command` WAL archive, a restore drill from that copy, and owner-set RPO and RTO (ADR 0046 §7). `PRODUCTION_READINESS_CHECKLIST.md` records this as an open blocking row.

**Owned paths.**

- `deploy/synology/postgres/backup/**` (new: the `archive_command` wrapper, the base backup job, the retention job)
- `deploy/synology/compose.yml`: a `postgres-backup` sidecar service
- `deploy/synology/scripts/{backup-postgres,restore-postgres-pitr}.sh` (new)
- `.github/workflows/recover-synology-staging-schema.yml`: extended with a `pitr-drill` mode, because the workflow budget is full
- `docs/operations/INCIDENT_RECOVERY_RUNBOOK.md` (Platform PostgreSQL section)
- `docs/operations/PRODUCTION_READINESS_CHECKLIST.md` (backup rows, including the off-NAS requirement)
- `docs/operations/PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md` (new, dated)

**Work.**

1. **WAL.** `wal_level=replica`, `archive_mode=on`, `archive_timeout=60s`. `archive_command` writes to a spool on a NAS path outside the database volume. Failure is visible: `pg_stat_archiver.failed_count` and spool age feed `health-check.sh`.
2. **Base backups.** A daily `pg_basebackup -X stream` by `oteryn_platform_backup`, written to the spool. Staging retention is 7 daily and 4 weekly backups. Hold integrity checksums for every file. Encryption is optional on the NAS and required for the later off-NAS copy.
3. **Witness and keys.** The `platform_storage` native-evidence witness directory and the external `oteryn-native-admission-keys` volume are backed up on the NAS on their own schedule. The runbook records the restore order: database to its target time, then the witness, which is never lowered, then `game-auth:native-evidence:reconcile` where the contract allows it. When restored history is below the retained floor, evidence reads stay fail-closed: recovery needs a restore point at or above the floor, or a fix-forward under an owner decision. The witness is never lowered, emptied or deleted. The joint Platform and Game restore procedure (F-04) is referenced here, and Game owns its side once the Game node is deployed.
4. **Restore drill.** Restore the newest base backup plus WAL to a named target time into a scratch PostgreSQL container with its own volume on the NAS. Then:
   - check row counts of key tables, the wallet ledger-sum invariant and the migrations head;
   - run `production:verify-configuration`;
   - record the measured restore time and data-loss window against the provisional RPO and RTO as dated evidence;
   - discard the scratch instance.
5. **Tooling.** Plain `pg_basebackup` plus `archive_command` scripts. pgBackRest or WAL-G may replace them before production; that choice does not change the gates.

**Validation.**

- `pg_stat_archiver` shows successful archiving and no failures for 24 hours.
- A forced `pg_switch_wal()` segment appears in the NAS spool.
- One full drill restores to a chosen timestamp, and its evidence is dated.
- The runbook section is reviewed.

**Rollback.** Turn off `archive_mode` and stop the sidecar. Backups are additive and do not affect live data.

### P6 — Staging cutover

**Goal.** Move the Synology staging Platform from MariaDB to a clean-slate PostgreSQL database, with dated evidence. Rollback exists only before the first witnessed write on PostgreSQL, which comes at the start of step 4 with the provenance-only init and the first trust publication attempt (owner answer Q2). After a successful smoke test the cutover is declared done and the MariaDB volume is deleted at once, with no retention window and no final dump (owner answer Q3).

**Owned paths.**

- `deploy/synology/scripts/cutover-postgres.sh` (new; orchestrates the steps below and refuses when preconditions fail)
- `.github/workflows/deploy-synology-staging.yml`: a cutover input
- P6b, the maintenance-gate design note: `docs/operations/PLATFORM_POSTGRES_MAINTENANCE_GATE_DESIGN.md` (new). It is a separate sub-deliverable, reviewed and accepted before P6 starts, and is its only owned path until then.
- `.github/workflows/cloudflare-oteryn-endpoints.yml`, `scripts/operations/cloudflare-oteryn-endpoints.sh`, `tests/operations/cloudflare-oteryn-endpoints/**`: implemented only after P6b is accepted, as the `maintenance-close` and `maintenance-open` modes and their tests, extending the existing workflow because the workflow budget is full
- `docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md`: the two modes
- P6a, the pre-step-4 trust safeguards: a separate sub-deliverable. Its design note is reviewed and accepted on its own before any implementation, and P6a is then merged and deployed before step 4 (a precondition below). P6a covers the provenance-only init and the attempt evidence in *Trust safeguards (P6a)* below. The design note fixes the mechanism, the exact paths and the tests. P6a is expected to touch the native evidence witness under `app/GameAuth/NativeEvidence/`, the native admission console commands and their tests under `tests/Feature/GameAuth/`. It includes no missing-profile repair.
- `docs/operations/PLATFORM_POSTGRES_CUTOVER_EVIDENCE.md` (new, dated)
- the packet's task record

The engine switch value is a repository variable that the operator sets.

**Preconditions, checked by the script.**

- P1 stage B is required and green.
- P2, P3 and P4 are merged and deployed with engine `mariadb`, and the ADR 0044 amendment is accepted.
- P5 archiving is active and one drill has passed.
- The P6a design note is accepted, and P6a is merged and deployed.
- The P6b design note is accepted, and the gate it specifies is merged.
- No tunnel ingress rule other than the two canonical ones targets the loopback origins (see *No other route to the origins* below).
- The owner gave a go decision and a window.
- `MARKETPLACE_ENABLED=false`.

**Trust safeguards (P6a).** The first trust publication attempt advances the trust-state witness before its database write. A failed attempt can therefore leave the missing-profile state: a floor of 1 and no profile row. `reconcile --trust` refuses it (`NativeEvidenceRecoveryReconciler.php:61-71`), and so does a new publication (`NativeSigningTrustRegistry.php:47-50`). Creating a profile for it would erase the history `historicalKeyIdExists()` relies on. A failed publication cannot be reliably told apart from a restore or relational data loss.

- **True before the first publication attempt in step 4:**
  - The PostgreSQL witness-store id and its database binding exist. Today the witness creates them only when a namespace is first entered (`NativeEvidenceHighWaterWitness::reconcileStoreProvenance`), and with native admission off nothing enters one in P3. So P6a provides a provenance-only init that creates and binds the store id and advances no namespace floor. It runs at the start of step 4, after the last rollback point.
  - Durable attempt evidence exists outside the database and outside the witness directory, on storage that no database or Platform volume restore rolls back. It binds a new P6 run id, that store id, the trust scope and the key id being published. The exact mechanism belongs to the P6a design note.
- **The missing-profile state has no automated repair in this plan.** If it occurs, the cutover stops and is not declared done:
  - native admission stays off and the scope stays unavailable;
  - forward reconciliation is used only where it already applies;
  - the owner decides.

  Nothing in P6 depends on a repair. A repair mechanism is a topic for the P6a design note. It must prove that it tells a failed publication apart from a restore, and it needs its own review and the producer contract owner's review before it is built.
- **A restore never synthesizes a profile.** P6a carries one negative test for this.
- **A successor key is new.** Any later successor profile uses a newly generated key. `deploy/synology/scripts/generate-native-admission-key.sh` appends every generated key id to `used.kids`. The successor id must appear there exactly once, as its own generation record, and must not appear in database history (`historicalKeyIdExists()`).
- **Forward-only.** P6a never lowers, moves, edits or replaces the PostgreSQL witness, and nothing in it reopens a rollback or a P3 re-run.

**Edge gate instead of maintenance mode.** The smoke tests must reach a running application: `health-check.sh` and the Gateway's calls to the Platform issuer would only get maintenance responses under `php artisan down`. The host bindings cannot serve as the gate either: `deploy.sh` and `deploy-impact.sh` require `PLATFORM_BIND_ADDRESS` and `GATEWAY_BIND_ADDRESS` to stay loopback-only, and the Cloudflare Tunnel reaches the public hostnames through exactly those loopback origins (`docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md`, fixed endpoint contract). So the bindings stay loopback, the application stays up, and the cutover closes the public edge instead:

- **Close.** A new bounded mode of the existing `Cloudflare Oteryn Endpoints` workflow, `maintenance-close`, changes only the two canonical ingress rules (`oteryn.molehill.cloud` and `gateway.molehill.cloud`) from their fixed loopback origins to the Cloudflare `http_status:503` service. It uses the same safeguards as `apply`: a confirmation phrase, a fresh read, rejection of locally managed tunnels and ambiguous rules, preservation of every unrelated ingress rule and the final catch-all, and a re-read and hash check immediately before `PUT`. DNS records are not touched. The tunnel connector keeps running, because the tunnel may carry unrelated routes.
- **No other route to the origins (P6 precondition).** The script preserves unrelated ingress rules (`cloudflare-oteryn-endpoints.sh:176-184`), and the Platform has no trusted-host allowlist in `bootstrap/app.php`. So any other rule that targets the loopback origins would keep the application public while the canonical rules return 503. This includes a wildcard hostname, the legacy Gateway hostname and the final catch-all. `maintenance-close` therefore fails closed: it refuses, and changes nothing, when any ingress rule other than the two canonical ones has a service on port 8000 or 8080 of the connector host (`127.0.0.1`, `localhost`, `[::1]` or `0.0.0.0`). The cutover script runs the same check as a precondition before the freeze, and the operator removes or repoints any such rule through the normal reviewed path first.
- **Gate state machine (P6b).** The maintenance-gate state machine is owned by a separate design note, P6b, which is accepted before P6 starts. P6 cannot start without it. That covers the marker, the lock, ordering relative to the ingress write, crash recovery and tests per transition. Until then the plan makes no claim about gate semantics beyond two: the gate fails closed, and no mutating entry point may reopen the origins except `maintenance-open`.
- **External check.** Before any smoke test, the cutover script requires two proofs: a fresh API read shows both canonical rules on `http_status:503`, and HTTPS requests from the GitHub runner, outside the NAS, to both public hostnames get the edge's 503. The same read shows that no other rule targets the origins. A request to a random, unused subdomain that reaches the catch-all, and to every other hostname the tunnel serves, gets no application response. The script refuses to continue otherwise, and records both proofs in the evidence.
- **Smoke on loopback.** All smoke tests run on the NAS against the loopback origins `127.0.0.1:8000` and `127.0.0.1:8080`, and inside the Compose network. No public request reaches the application.
- **Open.** `maintenance-open` restores exactly the two fixed loopback origins with the same safeguards. A fresh `audit` and an external request to each hostname must then show the normal contract. Opening happens only in step 7, or at the end of a rollback after `health-check.sh` passes on MariaDB.
- `audit` reports the closed state as bounded drift, not as a failure, so a closed edge stays visible.
- No maintenance bypass secret, cookie or path is used or committed. The gate needs only the existing `production-cloudflare` environment and its token.

**Steps.**

1. **Freeze.** Close the edge gate (`maintenance-close`) and pass the external check, set native admission off (the Gateway refuses protocol version 2 login), stop the scheduler, then stop the `platform` and `mariadb` containers. The `mariadb_data` volume and the MariaDB-era witness directory are not modified from here on.
2. **Bootstrap.** Set `PLATFORM_DB_ENGINE=postgres` and run the P3 bootstrap: MariaDB-era witness directory moved aside, a new empty PostgreSQL witness directory, fresh migrations, keys, OAuth clients, World and route. Native admission is still off, so trust publication waits for step 4. The operator updates `OTERYN_STAGING_NATIVE_WORLD_ID` and the OAuth client ids of consumers, then deploys behind the gate.
3. **Smoke tests without witnessed writes**, on the NAS against the loopback origins: `health-check.sh`, web registration and login, admin login and a wallet read.
   - On failure: **rollback.** Set `PLATFORM_DB_ENGINE=mariadb`. Remove the PostgreSQL witness directory, which is still empty because trust publication has not been attempted, and move the MariaDB-era one back. Deploy behind the gate, run `health-check.sh`, then open the edge gate (`maintenance-open`) and confirm the public hostnames from outside. MariaDB and its witness directory are exactly as they were at the freeze. Keep the PostgreSQL volume for diagnosis.
   - Instead of a rollback, a clean-slate re-run of P3 (emptied database and emptied PostgreSQL witness directory) is also allowed here.
   - **Last rollback point.** Steps 3 and earlier are the only places where rollback or a P3 re-run is allowed.
4. **Native admission re-enable, still behind the gate.** The scheduler stays stopped. `cutover-postgres.sh` first runs the P6a provenance-only init and records the attempt evidence for this run. The operator then turns native admission on through the deploy, which runs `game-auth:native-admission:publish-trust` and then `game-auth:native-admission:self-check`, and refuses on failure. The public Gateway hostname still returns the edge's 503, so only clients on the NAS reach the Gateway through its loopback origin.
   - `NativeSigningTrustRegistry::publishTrustedKey` advances the retained trust-state witness before its database transaction commits. The first publication attempt therefore advances a witness floor whether it succeeds or fails. It follows the provenance-only init, which already writes to the witness. From the start of step 4 on (owner answer Q2):
     - there is no rollback to MariaDB and no clean-slate re-run of P3;
     - the PostgreSQL witness directory is never emptied, deleted, replaced or lowered, and the database is never emptied under it.
   - **Publication or self-check fails.** Turn native admission off again and keep the witness and the database.
     - When the cause is outside trust state, for example configuration or the key file, fix it and retry. A retry on a consistent state is idempotent.
     - When the trust state is ambiguous because the witness is ahead of the database, reads and mutations stay unavailable by contract (`OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md`, trust state). There are two such states:
       - **Profile row present, issuer revision behind the floor.** Recover with `game-auth:native-evidence:reconcile --trust=fresh`. It moves the relational issuer revision forward to the retained floor and leaves that profile version revoked.
       - **Missing profile.** The first publication on the clean slate advanced the witness to 1, then its database transaction did not leave a profile row. `reconcile --trust` refuses this state (`NativeEvidenceRecoveryReconciler.php:61-71`), and so does a new publication (`NativeSigningTrustRegistry.php:47-50`). This plan has no automated repair for it (*Trust safeguards (P6a)*). Native admission stays off with the gate closed, the cutover stops and is not declared done, and the operator reports to the owner, who decides.
     - A profile left revoked by `reconcile --trust=fresh` needs a successor profile under the successor key rule before native admission can work. No console command publishes one today, so this case also stops the cutover for an owner decision.
     - In every case the PostgreSQL witness is never removed, lowered or replaced. The cutover is not declared done until the self-check passes.
5. **Native login smoke.** Run the native login end-to-end smoke and the evidence read path on the NAS against the loopback Gateway origin. The same forward-only rule applies to the evidence it writes.
   - A failure is fixed forward with the witness kept.
   - An account generation left with the witness ahead is recovered with `game-auth:native-evidence:reconcile --account-id=...`, which only moves state forward to the retained floor.
   - If a state cannot be reconciled, native login stays off until a fix lands, and the witness is not touched.
6. **Declare done and retire MariaDB.** Only when step 5 passes. Until then the MariaDB volume stays as it is, but it is no longer a rollback target after step 4 starts. When step 5 passes, record the cutover as done. Then, as an operator action without further approval:
   - delete the `mariadb_data` volume and remove the `mariadb` container;
   - delete the MariaDB-era witness directory moved aside in step 2. It is the high-water witness of the retired MariaDB store and is bound to a database that no longer exists, so retiring it lowers nothing. The live PostgreSQL witness directory is a different directory and is not touched;
   - take no final dump.
7. **Unfreeze.** Open the edge gate (`maintenance-open`), confirm with `audit` and external requests that both public hostnames reach their origins again, start the scheduler and record the evidence: timings, image tags, the gate check, the new non-secret OAuth client ids and WorldId, and the deletion time.

**Validation.** The cutover evidence file is complete. Staging deploy and health stay green on PostgreSQL. A post-cutover P5 drill passes.

**Rollback.** Only as in step 3, before the provenance-only init and the first trust publication attempt, ending with the gate opened on MariaDB. From step 4 on there is no rollback, only forward reconciliation that keeps the PostgreSQL witness. That includes the missing-profile state left by a first publication whose transaction rolled back after the witness advanced. That state has no automated repair in this plan. Native admission stays off, the cutover stops and the owner decides. It is never handled by a rollback or a P3 re-run. After step 6 there is no MariaDB data to return to, which the owner accepted because all data is test data.

### P7 — Remove MariaDB code paths and reconcile docs

**Goal.** Delete the Platform's MariaDB code, configuration and scripts, and bring every live document in line with the new engine. The MariaDB volume is already gone after P6.

**Owned paths.**

- `deploy/synology/compose.yml` and `compose.marketplace.yml`: remove `mariadb`, `mariadb_data` and the engine switch
- `deploy/synology/mariadb/**` (delete)
- `deploy/synology/scripts/**` and `deploy/synology/release-contract.env`: remove the MariaDB branches
- `deploy/synology/tests/**`, `tests/ci/**`
- `.github/workflows/**`: Platform-DB `mariadb` service containers become `postgres`; legacy-engine test services stay until #1474
- `phpunit.xml`, `phpunit.pgsql.xml`: decide the default real-database suite
- `config/database.php`: remove the `mysql` Platform connection; the legacy-engine connections stay until #1474
- `docs/architecture/DATA_OWNERSHIP.md`: "Database credentials" records the delivered owner, runtime, physical backup and logical dump split and the engine
- `docs/architecture/SYSTEM_ARCHITECTURE.md`
- `docs/architecture/adr/README.md`: status notes
- `docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json`: database-engine and backup records (review D-7)
- `deploy/synology/README.md`
- `docs/operations/{INCIDENT_RECOVERY_RUNBOOK,PRODUCTION_READINESS_CHECKLIST,PRODUCTION_TOPOLOGY_EVIDENCE,SYNOLOGY_ROLLBACK_SCHEMA_SAFETY,NATIVE_LOGIN_OWNER_STEPS}.md`
- `docs/agents/{BUILD_TEST_MATRIX,PROJECT_STATE,REPOSITORY_MAP}.md`
- the remaining live documents found by `rg -il mariadb docs`

Files under `docs/agents/tasks/archive/`, `docs/evidence/` and dated reviews are historical and are not rewritten.

**Work.** Confirm that the P6 cutover evidence is complete and a P5 drill on PostgreSQL passed after cutover. The operator removes the retired MariaDB secrets (`PLATFORM_DB_PASSWORD` and the MariaDB root password). `pdo_mysql` is removed from the images only when #1474 has retired the legacy-engine connections.

**Validation.**

- `rg -il mariadb deploy config app .github/workflows` finds only legacy-engine paths owned by #1474.
- The full CI is green.
- A staging deploy is green.
- The ADR registry, documentation IA and legacy-token ratchet checks pass, and the baseline is not raised.

**Rollback.** Revert the PR to restore the MariaDB code path. There is no MariaDB data to return to.

## 5. What the owner or operator supplies

The repository contains none of these. Each is set as a GitHub environment secret or variable for the Synology staging environment, or held on the NAS, by the operator.

| Input | Needed by | Notes |
|---|---|---|
| PostgreSQL bootstrap superuser password | P4 | Init only; never in the `platform` service |
| Owner, runtime, physical backup and logical dump role passwords | P4, P5 | Four distinct secrets; the old `PLATFORM_DB_PASSWORD` is removed in P7 |
| Required-check ruleset edit | P1 stage B | Repository admin |
| ADR 0044 amendment acceptance | P4 | Owner |
| First administrator values for `admin:bootstrap` | P3, P6 | Operator; never committed |
| New `OTERYN_STAGING_NATIVE_WORLD_ID` and the consumers' OAuth client ids after bootstrap | P6 | Non-secret values printed by the bootstrap |
| NAS access: space for the backup spool (about 2x the database size plus WAL), a scratch volume for drills, Container Manager or shell to delete `mariadb_data` | P4 to P6 | No firewall or port change; PostgreSQL publishes no port |
| Go decision and maintenance window | P6 | Owner |
| Runs of the `Cloudflare Oteryn Endpoints` workflow in the `production-cloudflare` environment to close and reopen the edge | P6 | Existing token and environment; no new secret |
| Off-NAS backup target, its credentials and an encryption key pair; production RPO and RTO | Before production or real players | Not needed for staging (ADR 0046 §7) |

## 6. Owner answers (2026-10-12)

| ID | Question | Answer |
|---|---|---|
| Q1 | Data at cutover | **Clean slate.** The Game node is not deployed anywhere and all staging data is test data. P3 re-creates state on a fresh database and resets the witness directory together with it. A table-copy tool stays an optional, documented fallback. |
| Q2 | Rollback after native evidence was written on PostgreSQL | **No rollback after the first evidence write.** The boundary is the first witnessed write, at the start of P6 step 4. That is the provenance-only witness-store init, followed by the first trust publication attempt, which advances the trust-state witness before its database write. Before it, rollback means pointing Compose back at the MariaDB volume and the untouched MariaDB-era witness directory (P6 step 3). After it, keep the PostgreSQL witness and reconcile forward only. |
| Q3 | Retention of the MariaDB volume | **None.** The volume is deleted right after a successful cutover smoke test, and no final dump is taken. |
| Q4 | PostgreSQL TLS certificate source | **Stack CA via `tls-init`.** |
| Q5 | Backups | **On the Synology only for now**, with provisional staging RPO 24 hours and RTO 4 hours. An encrypted off-NAS copy (`pg_basebackup` plus `archive_command`) is REQUIRED before any production or real-player use. |
| Q6 | Accent-insensitive search | **Accent-sensitive search, no `unaccent`.** |

## 7. Review findings covered

| Finding | Packet |
|---|---|
| F-01 no scheduled backup or PITR | P5 |
| F-02 single NAS holds everything | Deferred: the off-NAS encrypted copy is required before production or real players (ADR 0046 §7); host placement stays D-3 |
| F-04 no cross-system restore | P5 (Platform side of the joint runbook); Game's side starts when the Game node is deployed |
| F-05 wallet and bid concurrency proven only on SQLite | P1 |
| F-06 runtime user owns the schema | P4 |
| F-12 destructive `down()` on financial tables | P2 forward migrations keep guarded `down()`; existing ones stay a separate packet |
| F-13 `sqlite` default | P2 (`production:verify-configuration` keeps refusing it) |
