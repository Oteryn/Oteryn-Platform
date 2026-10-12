# Platform PostgreSQL migration plan

- Status: plan. It authorizes no packet by itself; each packet below needs its own task record, PR and evidence.
- Decision: ADR 0046 (`docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md`)
- Governing Issue: #1474
- Input: `docs/architecture/reviews/OTERYN_PLATFORM_ARCHITECTURE_AND_DATABASE_REVIEW_2026-10-12.md` (PR #1560), findings F-01, F-02, F-04, F-05, F-06, F-12 and F-13
- Baseline: `main` at `b2e86fae` on 2026-10-12

## 1. Scope and fixed rules

The plan moves the Platform's own database, `oteryn_platform`, from MariaDB 11.8 to PostgreSQL 17. It runs in a Platform-owned instance that is separate from Game's instance (ADR 0046 §4).

These rules hold in every packet:

- Game-ADR-0004 invariants: separate logical databases (`oteryn_platform`, `oteryn_game`), roles, credentials and migration histories, and no shared-table writers. No packet reads or writes the Game database.
- The repository holds no secret, certificate, private key, password, dump or backup. Packets commit templates with placeholders only. Section 5 lists what the owner or operator supplies.
- Packets run in the order P1 to P7. P1 stage A can run alongside P2. Every other packet starts only after its predecessors have merged.
- From P1 stage B until P7, code and migrations must pass on both engines.
- The two legacy-engine `mysql` connections in `config/database.php` are out of scope. They retire under #1474. `pdo_mysql` stays in the images until then.
- MariaDB stays deployed and authoritative until the P6 cutover. Its volume is deleted only in P7, after the retention window.
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
| P3 | Data path and copy tooling | P2 | Dry run on a copy of staging data with zero checksum differences | Read access to a staging dump copy |
| P4 | Synology Compose: PostgreSQL service, TLS and role split (F-06) | P2, P3 | PostgreSQL running idle beside MariaDB; role and TLS checks pass; ADR 0044 amendment accepted | Secrets, TLS choice, ADR 0044 amendment acceptance |
| P5 | Backup and PITR: base backup, WAL archive, off-NAS copy and restore drill (F-01, part of F-02) | P4 | Archiving active, off-NAS copy present, one dated restore drill passed | Off-NAS target, encryption key, RPO/RTO |
| P6 | Staging cutover and rollback window | P1 stage B, P3, P4, P5 | Platform on PostgreSQL; dated cutover evidence; window started | Go decision, maintenance window, retention days |
| P7 | Remove MariaDB and reconcile docs | P6 window ended without rollback | No Platform-DB MariaDB path left; docs and registries updated | Approval to delete the volume and secrets |

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

**Rules for migrations.** All 68 migrations are already applied on staging MariaDB. A change to an existing migration must leave the MariaDB result identical, either through a driver branch or by being a no-op on MariaDB. Corrections that only add something, such as a `CHECK` constraint or an expression index, go into new forward migrations that run on both engines. Migrations are not squashed or rebaselined. PostgreSQL replays the same 68 migrations, so both `migrations` tables match (P3 checks this).

**Checklist.** Each item is closed with a test or with a recorded reason it does not apply.

1. **Driver branches.** In `NativeAdmissionAttempts`, replace `SET SESSION innodb_lock_wait_timeout` with `SET LOCAL lock_timeout` inside the transaction on `pgsql`, keeping the same bound. Review every `getDriverName()` branch, and the `LOCK TABLES` branch in migration `2026_10_08_090000`; on PostgreSQL use `LOCK TABLE ... IN ACCESS EXCLUSIVE MODE` inside the migration transaction. The `PreproductionNativeRouteStore` driver and host change waits for the ADR 0044 amendment in P4.
2. **Isolation.** InnoDB defaults to REPEATABLE READ and PostgreSQL to READ COMMITTED. List every transaction that reads the same rows more than once without a lock and relies on one snapshot. `NativeAccountCharactersReadModel:56-58` is the known case. Fix each with explicit row locks, or with `REPEATABLE READ` plus a retry on SQLSTATE `40001`. Keep READ COMMITTED as the default.
3. **Error codes.** Map MySQL `1062`, `1205`, `1213` and `1020` to SQLSTATE `23505`, `55P03`, `40P01` and `40001`. Prefer Laravel's `UniqueConstraintViolationException` and its concurrency-error detection over raw vendor codes.
4. **No `UNSIGNED`.** PostgreSQL has no unsigned types. For every value column whose non-negativity is an invariant (balances, amounts, counters, revisions, `chance_numerator`/`chance_denominator`), add `CHECK (col >= 0)` in a forward migration. MariaDB 11.8 enforces `CHECK` too. Plain foreign keys need nothing. Laravel's `unsignedInteger` becomes `integer` on PostgreSQL, whose maximum is 2^31-1, so P3 verifies that no value exceeds it.
5. **Case and accent rules.** `utf8mb4_unicode_ci` makes `=`, `LIKE` and unique indexes case-insensitive and largely accent-insensitive.
   - `identities.email` is canonicalized by `app/Identity/Support/CanonicalEmail.php`. Prove that every write path goes through it, and add a unique index on `lower(email)` as a safety net.
   - For the other unique `slug`/`key`/`code` columns, decide case rules per column.
   - Search `LIKE` sites become `ILIKE` or `LOWER(col) LIKE LOWER(?)`.
   - Accent-insensitive matching (relevant to Polish content) is lost unless the `unaccent` extension is used. Default: accept accent-sensitive search and record it in `FEDERATED_SEARCH_ARCHITECTURE.md`.
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

### P3 — Data path and copy tooling

**Goal.** Decide, for every table, whether to copy, rebuild or drop it. Build a repeatable copy-and-verify tool. Prove it on a copy of staging data.

**Decision on data.** All staging data is test data. Even so, copying is the default over re-seeding, because Platform state is referenced from outside the database:

- Game stores `AccountId`s (ADR 0028), World and Channel ids, and receipts keyed by Platform operation ids.
- The native evidence witness (`OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md`) fails closed when relational history is below the retained floor.
- Re-seeded identities, worlds or evidence would therefore orphan Game references and stop native login.

Re-seeding remains an owner option (§6, Q1).

| Class | Tables (by prefix or name) | Action |
|---|---|---|
| Copy, identity and cross-boundary | `identities`, `identity_*`, `admin_*`, `oauth_clients`, `game_worlds`, `game_channels`, `game_world_protocol_candidates`, `native_scope_assignments`, `native_assignment_epoch_locks`, `native_game_evidence_*`, `native_game_signing_trust_*`, `native_device_session_*`, `character_command_*`, `character_bootstrap_intent*`, `character_auction*`, `wallet_*`, `payment_*`, `premium_time_*`, `product_catalog_*`, `admin_audit_events` | Copy exactly, keeping primary keys, then reset sequences |
| Copy, content | `managed_page*`, `news_posts`, `events`, `event_translations`, `wiki_*`, `editorial_*`, `site_announcements`, `homepage_template_settings`, `client_release*`, `client_update_*`, `game_catalog_*`, `support_*`, `player_reports`, `enforcement_records`, `character_profile_preferences`, `player_companion_session_analyses` | Copy (cheaper than re-seeding and keeps staging realistic) |
| Rebuild from producers | `game_public_projection_*`, `native_account_character_*`, `native_runtime_status_reports` | Do not copy; Game producers rebuild them after cutover |
| Drop (short-lived) | `game_login_tickets`, `native_admission_attempts`, `oauth_auth_codes`, `oauth_device_codes`, `oauth_access_tokens`, `oauth_refresh_tokens`, `password_reset_tokens` | Not copied. They expire anyway. Testers sign in again. |
| Not copied | `migrations` | PostgreSQL builds its own by replaying the same migrations. The tool refuses unless both name sets are equal. |
| Retired | the legacy-engine account-binding table and other #1474 compatibility tables still present | Copy while they exist; #1474 removes them |

The tool fails closed when any table in the source schema is not classified in its manifest.

**Owned paths.**

- `app/Console/Commands/Operations/PlatformDatabaseCopy.php` (new). It is an artisan command because it runs inside the Platform image, which has both PDO drivers. It is removed in P7.
- `tests/Feature/Operations/PlatformDatabaseCopyTest.php` (new)
- `database/copy/platform-table-manifest.json` (new; the per-table classification and column type map)
- `docs/operations/PLATFORM_POSTGRES_DATA_MANIFEST.md` (new; the human-readable version of the table above)

**Tool behaviour.**

1. Preconditions: source and target `migrations` name sets are equal; the target is empty except for its schema; the source is quiesced, meaning no non-terminal `character_command_intents` or Bazaar transfer steps; both sessions run in UTC.
2. Copy in primary-key order, in batches, inside a single target transaction per table. Disable nothing: foreign keys stay enforced, and tables load in dependency order.
3. Type mapping. The target schema comes from the Laravel migrations, not from converting MariaDB DDL.

   | MariaDB | PostgreSQL | Check |
   |---|---|---|
   | `bigint unsigned auto_increment` | `bigint` identity | `setval` to `max(id)`; values up to 2^63-1 |
   | `int unsigned` | `integer` | refuse values above 2^31-1 |
   | `tinyint(1)` | `boolean` | only 0 and 1 allowed |
   | `varchar(n)`/`char(n)` utf8mb4 | `varchar(n)`/`char(n)` | refuse NUL bytes; valid UTF-8 |
   | `text`/`mediumtext`/`longtext` | `text` | byte-exact |
   | `json` (MariaDB longtext alias) | `json` | text copied byte-exact |
   | `enum` | `varchar` + `CHECK` | value in the allowed set |
   | `timestamp`/`datetime` | `timestamp(0)` | UTC; refuse zero dates |
   | `decimal(p,s)` | `numeric(p,s)` | exact |
   | `double` | `double precision` | exact text round-trip |
   | `uuid` | `uuid` | lowercase canonical form |

4. Verification: rerun read-only on both sides and refuse on any difference.
   - Per table: row count, plus a SHA-256 over rows in primary-key order, with values serialized canonically (booleans as `t`/`f`, timestamps as ISO-8601 UTC, uuid lowercase).
   - Wallet: for every account, the balance equals the sum of ledger deltas. Holds match open auction reservations.
   - Evidence: the maximum `source_revision` per namespace is equal on both sides, and the `native_game_evidence_witness_stores` binding is unchanged.
   - Sequences: every identity sequence is at or above `max(id)`.
   - Output: one JSON evidence file with no row data.

**Validation.**

- CI test: seed MariaDB with the acceptance fixtures, copy to PostgreSQL, and verify zero differences.
- A negative test proves that an altered row is detected.
- A dry run on a copy of a staging `mariadb-dump`, made by the operator, into a scratch PostgreSQL, with the evidence JSON recorded (row data is never committed).

**Rollback.** The tool only reads the source. Drop and recreate the scratch target. Nothing deployed changes.

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
   - `ssl=on`, TLS 1.3 minimum, with the server certificate signed by the stack CA from `tls-init`. Alternatively an operator-supplied certificate (§6, Q4).
   - `pg_hba.conf` has only `hostssl ... scram-sha-256` lines plus local socket access for bootstrap; `host` without SSL is rejected.
   - Platform connects with `sslmode=verify-full` and the CA mounted read-only.
3. **Roles.** The init script is idempotent and refuses unsafe names, like the MariaDB one.
   - `oteryn_platform_owner`: `LOGIN`, owns database `oteryn_platform` and schema `public`.
   - `oteryn_platform_app`: `LOGIN`, `CONNECT` and `USAGE` only; DML on all tables and `USAGE`/`SELECT` on sequences through `ALTER DEFAULT PRIVILEGES FOR ROLE oteryn_platform_owner`; no `CREATE`, not an owner.
   - `oteryn_platform_backup`: `LOGIN REPLICATION`, used by P5.
   - `REVOKE ALL ON DATABASE oteryn_platform FROM PUBLIC`, and `REVOKE CREATE ON SCHEMA public FROM PUBLIC`.
   - The superuser password is used only by the init step and never reaches the `platform` service.
4. **Migration step.** Replace `compose exec -T platform php artisan migrate` (`deploy.sh:765`, `marketplace-staging.sh:196`) with a one-off `compose run --rm` container on the `pgsql_migrations` connection that receives the owner credentials. The runtime `platform` container gets only the app role.
5. **Engine switch.**
   - `PLATFORM_DB_ENGINE=mariadb|postgres` in the release contract, default `mariadb`. Deploy scripts branch on it for readiness, pre-migration dump (`pg_dump -Fc` as the backup role), restore, preflight and health.
   - With `mariadb`, PostgreSQL runs idle and empty except for roles. Only P6 flips the switch.
6. **Retention guard.** The hygiene and orphan-repair workflows must never remove `mariadb_data` or `postgres_data`. Add a test that proves it.
7. **ADR 0044 amendment.** The single allowlisted preproduction store identity becomes driver `pgsql`, host `postgres`, port `5432`, database `oteryn_platform`, taking effect with the engine switch. Until the owner accepts it, `OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED` stays `false` on the PostgreSQL engine.

**Validation.**

- `docker compose config` renders for both engine values.
- The deploy contract tests in `deploy/synology/tests` and `tests/ci` pass.
- Staging deploy with `mariadb` is unchanged and the PostgreSQL service is healthy.
- A non-TLS connection is refused; `verify-full` succeeds.
- The runtime role gets `permission denied` for `CREATE TABLE`, `ALTER TABLE` and `DROP TABLE`, and owns no relation.
- `docker inspect platform` shows no owner or superuser password.
- A migration run as the owner on an empty database matches the P2 schema.

**Rollback.** Revert the PR. The idle PostgreSQL service and its volume can be removed by the operator. MariaDB never changed.

### P5 — Backup and PITR

**Goal.** Close F-01 for the target engine, and close F-02 for database backups: continuous WAL archiving, scheduled base backups, an encrypted off-NAS copy, and a dated restore drill. This is required before the PostgreSQL-backed Platform is used for anything beyond testing (ADR 0046 §7).

**Owned paths.**

- `deploy/synology/postgres/backup/**` (new: the `archive_command` wrapper, the base backup job, the retention job)
- `deploy/synology/compose.yml`: a `postgres-backup` sidecar service
- `deploy/synology/scripts/{backup-postgres,restore-postgres-pitr}.sh` (new)
- `.github/workflows/recover-synology-staging-schema.yml`: extended with a `pitr-drill` mode, because the workflow budget is full
- `docs/operations/INCIDENT_RECOVERY_RUNBOOK.md` (Platform PostgreSQL section)
- `docs/operations/PRODUCTION_READINESS_CHECKLIST.md` (backup rows)
- `docs/operations/PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md` (new, dated)

**Work.**

1. **WAL.** `wal_level=replica`, `archive_mode=on`, `archive_timeout=60s`. `archive_command` writes to a local spool on a NAS path outside the database volume, and a pusher copies segments off-NAS. Failure is visible: `pg_stat_archiver.failed_count` and spool age feed `health-check.sh`.
2. **Base backups.** A daily `pg_basebackup -X stream` by `oteryn_platform_backup`, written to the spool and pushed off-NAS. Retention for preprod defaults to 7 daily and 4 weekly backups, and the owner sets the final values (D-2).
3. **Encryption.** Backups are encrypted on the NAS with the owner's public key before they leave the host. The private key is kept off the NAS, so a compromised NAS cannot read the backups. Hold integrity checksums for every file.
4. **Witness and keys.** The `platform_storage` native-evidence witness directory and the external `oteryn-native-admission-keys` volume are copied off-NAS on their own schedule. The runbook records the restore order: database to its target time, then the witness, which is never lowered, then `game-auth:native-evidence:reconcile` where the contract allows it. The joint Platform and Game restore procedure (F-04) is referenced here, and Game owns its side.
5. **Restore drill.** Restore the newest base backup plus WAL to a named target time into a scratch PostgreSQL container with its own volume. Then:
   - run the P3 verifier in read-only mode against the expected state, including the ledger-sum check;
   - check the migrations head and `production:verify-configuration`;
   - record the measured restore time and data-loss window as dated evidence;
   - discard the scratch instance.
6. **Alternative tooling.** pgBackRest or WAL-G may replace the plain scripts if the owner prefers. Both give encryption, retention and verification built in. That choice does not change the gates.

**Validation.**

- `pg_stat_archiver` shows successful archiving and no failures for 24 hours.
- A forced `pg_switch_wal()` segment appears off-NAS.
- One full drill restores to a chosen timestamp with zero verifier differences, and its evidence is dated.
- The runbook section is reviewed.

**Rollback.** Turn off `archive_mode` and stop the sidecar. Backups are additive and do not affect live data. Encrypted off-NAS copies follow the owner's retention.

### P6 — Staging cutover and rollback window

**Goal.** Move the Synology staging Platform from MariaDB to PostgreSQL, with dated evidence and a bounded rollback window.

**Owned paths.**

- `deploy/synology/scripts/cutover-postgres.sh` (new; orchestrates the steps below and refuses when preconditions fail)
- `.github/workflows/deploy-synology-staging.yml`: a cutover input
- `docs/operations/PLATFORM_POSTGRES_CUTOVER_EVIDENCE.md` (new, dated)
- the packet's task record

The engine switch value is a repository variable that the operator sets.

**Preconditions, checked by the script.**

- P1 stage B is required and green.
- P2, P3 and P4 are merged and deployed with engine `mariadb`.
- The P3 dry run on a fresh staging copy shows zero differences.
- P5 archiving is active and one drill has passed.
- The owner gave a go decision and a window.
- `MARKETPLACE_ENABLED=false`, and there are no non-terminal command intents or Bazaar transfer steps.
- The Game operator knows that native login is unavailable during the window.

**Steps.**

1. **Freeze.** Run `php artisan down` with a bypass for the operator, set native admission off (Gateway refuses protocol version 2 login), and stop the scheduler.
2. **Snapshot.**
   - A final `mariadb-dump` with its checksum, copied off-NAS.
   - A read-only tar snapshot of the native-evidence witness directory, with `witness-store.id` and the floors recorded.
   - The MariaDB `migrations` name list.
3. **Schema.** Run the migrations on the empty PostgreSQL database as the owner role.
4. **Copy.** Run the P3 tool from MariaDB to PostgreSQL, verify with zero differences, and reset sequences.
5. **Switch.** Set `PLATFORM_DB_ENGINE=postgres` and deploy. Platform starts on PostgreSQL. The witness binding must match the copied `native_game_evidence_witness_stores` row.
6. **Smoke tests.**
   - `health-check.sh`, web login, admin login and a wallet read;
   - the native login end-to-end smoke;
   - the evidence read path;
   - projections rebuilding from Game producers.
7. **Unfreeze**, then record the evidence: timings, the measured RTO, checksums and the image tags.
8. **Retain MariaDB.**
   - Stop the `mariadb` container. Keep `mariadb_data` unmounted and unmodified for N days; the default is 14 and the owner sets the value (Q3).
   - Keep the final dump off-NAS for the same window.
   - The P4 guard keeps hygiene jobs away from both.

**Rollback triggers.** Any of these: a verifier difference, a failed health check or native login smoke, an error rate above the pre-cutover baseline, or an owner request.

**Rollback before step 7 (still frozen).** Set `PLATFORM_DB_ENGINE=mariadb` and deploy. MariaDB has not changed since the freeze.

- If no native evidence was written on PostgreSQL, rollback is exact.
- If the smoke tests advanced the witness, MariaDB history is now below the retained floor and evidence reads fail closed. Restore the step 2 witness snapshot together with MariaDB, as one owner-approved paired restore. The pair is a consistent earlier state, so no anti-rollback check is silently bypassed (Q2).

**Rollback within the window, after unfreeze.** As above, and in addition:

- writes made on PostgreSQL since cutover are discarded (test data, accepted by the owner);
- run the saga reconciliation against Game receipts (F-04); none are expected while the Bazaar is frozen;
- keep the PostgreSQL volume for diagnosis.

Reverse copy from PostgreSQL to MariaDB is not planned.

**After the window.** Rollback needs P5 PITR on PostgreSQL, or the retained dump if the owner decides to restore it.

**Validation.** The cutover evidence file is complete. Staging deploy and health stay green on PostgreSQL for the whole window. A post-cutover P5 drill passes.

### P7 — Remove MariaDB and reconcile docs

**Goal.** Delete the Platform's MariaDB path and bring every live document in line with the new engine.

**Owned paths.**

- `deploy/synology/compose.yml` and `compose.marketplace.yml`: remove `mariadb`, `mariadb_data` and the engine switch
- `deploy/synology/mariadb/**` (delete)
- `deploy/synology/scripts/**` and `deploy/synology/release-contract.env`: remove the MariaDB branches
- `deploy/synology/tests/**`, `tests/ci/**`
- `.github/workflows/**`: Platform-DB `mariadb` service containers become `postgres`; legacy-engine test services stay until #1474
- `phpunit.xml`, `phpunit.pgsql.xml`: decide the default real-database suite
- `config/database.php`: remove the `mysql` Platform connection; the legacy-engine connections stay until #1474
- `app/Console/Commands/Operations/PlatformDatabaseCopy.php`, its test and `database/copy/**` (delete)
- `docs/architecture/DATA_OWNERSHIP.md`: "Database credentials" records the delivered owner, runtime and backup split and the engine
- `docs/architecture/SYSTEM_ARCHITECTURE.md`
- `docs/architecture/adr/README.md`: status notes
- `docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json`: database-engine and backup records (review D-7)
- `deploy/synology/README.md`
- `docs/operations/{INCIDENT_RECOVERY_RUNBOOK,PRODUCTION_READINESS_CHECKLIST,PRODUCTION_TOPOLOGY_EVIDENCE,SYNOLOGY_ROLLBACK_SCHEMA_SAFETY,NATIVE_LOGIN_OWNER_STEPS}.md`
- `docs/agents/{BUILD_TEST_MATRIX,PROJECT_STATE,REPOSITORY_MAP}.md`
- the remaining live documents found by `rg -il mariadb docs`

Files under `docs/agents/tasks/archive/`, `docs/evidence/` and dated reviews are historical and are not rewritten.

**Work.** Before deleting anything:

- confirm the P6 window ended without a rollback and that a P5 drill on PostgreSQL passed after cutover;
- get owner approval;
- delete the `mariadb_data` volume and the retired secrets (operator action);
- the off-NAS final dump is kept or destroyed per the owner's retention.

`pdo_mysql` is removed from the images only when #1474 has retired the legacy-engine connections.

**Validation.**

- `rg -il mariadb deploy config app .github/workflows` finds only legacy-engine paths owned by #1474.
- The full CI is green.
- A staging deploy is green.
- The ADR registry, documentation IA and legacy-token ratchet checks pass, and the baseline is not raised.

**Rollback.** Before the volume is deleted, revert the PR to restore the MariaDB path. After deletion, the only way back is the retained dump, restored under a new owner decision.

## 5. What the owner or operator supplies

The repository contains none of these. Each is set as a GitHub environment secret or variable for the Synology staging environment, or held on the NAS, by the operator.

| Input | Needed by | Notes |
|---|---|---|
| PostgreSQL bootstrap superuser password | P4 | Init only; never in the `platform` service |
| Owner, runtime and backup role passwords | P4, P5 | Three distinct secrets; rotate the old `PLATFORM_DB_PASSWORD` out in P7 |
| TLS decision; certificate and key if not the stack CA | P4 | Default: stack CA via `tls-init` (Q4) |
| Required-check ruleset edit | P1 stage B | Repository admin |
| ADR 0044 amendment acceptance | P4 | Owner |
| Off-NAS backup target: endpoint, bucket or host, credentials | P5 | Encrypted copy only; separate from NAS credentials |
| Backup encryption key pair | P5 | Public key on the NAS, private key kept offline by the owner |
| RPO, RTO and retention | P5 | Review decision D-2 |
| NAS access: space for the spool (about 2x the database size plus WAL), a scratch volume for drills, Container Manager or shell for volume retention and deletion | P4 to P7 | No firewall or port change; PostgreSQL publishes no port |
| Staging dump copy for the P3 dry run | P3 | Never committed |
| Go decision, maintenance window, rollback-window length | P6 | Default 14 days |
| Approval to delete the MariaDB volume and secrets | P7 | After the window |

## 6. Open owner questions

| ID | Question | Options | Recommendation |
|---|---|---|---|
| Q1 | Data at cutover | (a) copy per the P3 manifest; (b) clean slate: re-seed Platform, reset the evidence witness and Game references in a coordinated reset | **(a).** It keeps `AccountId`s, World and Channel ids and evidence floors, and needs no Game reset. |
| Q2 | Rollback after native evidence was written on PostgreSQL | (a) approve a paired restore of the cutover witness snapshot with MariaDB, inside the window only; (b) no rollback after the first evidence write | **(a)**, limited to the staging window. |
| Q3 | Retention window for the MariaDB volume and final dump | 7, 14 or 30 days | **14 days.** |
| Q4 | PostgreSQL TLS certificate source | (a) stack CA via `tls-init`; (b) operator-supplied certificate | **(a).** It is the existing pattern and needs no secret handover. |
| Q5 | Backup tooling | (a) `pg_basebackup` plus `archive_command` scripts; (b) pgBackRest or WAL-G | **(a)** for staging, as decided. Revisit (b) before production. |
| Q6 | Accent-insensitive search | (a) accept accent-sensitive search; (b) add `unaccent` | **(a)** now. Search architecture decides later. |

## 7. Review findings covered

| Finding | Packet |
|---|---|
| F-01 no scheduled backup or PITR | P5 |
| F-02 single NAS holds everything | P5 (off-NAS encrypted copies of the database, witness and keys); host placement stays D-3 |
| F-04 no cross-system restore | P5 (Platform side of the joint runbook) and P6 (saga reconciliation on rollback) |
| F-05 wallet and bid concurrency proven only on SQLite | P1 |
| F-06 runtime user owns the schema | P4 |
| F-12 destructive `down()` on financial tables | P2 forward migrations keep guarded `down()`; existing ones stay a separate packet |
| F-13 `sqlite` default | P2 (`production:verify-configuration` keeps refusing it) |
