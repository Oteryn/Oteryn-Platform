# ADR 0046 — Platform persistence moves to PostgreSQL

## Status

Accepted — 2026-10-12 (owner decision)

- Decision owner: repository owner
- Governing Issue: #1474
- Delivery plan: `docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md` (packets P1 to P7)
- Input: owner decision D-1 in `docs/architecture/reviews/OTERYN_PLATFORM_ARCHITECTURE_AND_DATABASE_REVIEW_2026-10-12.md` (PR #1560)
- Applies to: the Platform's own relational store, logical database `oteryn_platform`, in every environment
- Does not apply to: the Oteryn Game database `oteryn_game`, or the two legacy-engine `mysql` connections in `config/database.php`, which are retired under #1474 and not migrated
- Does not authorize: any single packet, deployment, data copy, volume deletion, secret, credential or production change. Each packet needs its own task, PR and evidence. Production go-live remains gated by ADR 0007.

## Context

The Platform runs MariaDB 11.8 (`deploy/synology/compose.yml`, service `mariadb`). MariaDB was chosen for compatibility with the legacy game engine's shared schema, and #1474 retires that dependency.

Oteryn-Game ADR-0013 §2 makes the Platform's database engine a Platform-owned choice. It allows a move to PostgreSQL through a dedicated Platform decision that weighs material benefit: correctness or concurrency, measurable operational simplification, security or isolation, lower maintenance, retirement of a costly compatibility constraint, or a business or availability need. It adds that "language or database-stack uniformity alone is not sufficient justification". This ADR is that dedicated Platform decision.

The 2026-10-12 architecture and database review assessed those criteria (§4.3) and recommended option D-1(a): stay on MariaDB 11.8 and fix backups first. None of the criteria had been measured as met. The review's cost list for a migration is: 68 migrations and MariaDB-specific code, re-qualifying every real-database concurrency proof, the deploy, backup and rollback scripts and about 20 CI workflows, and a data migration with downtime and rollback on a single NAS.

On 2026-10-12 the owner chose option D-1(b) instead: plan the PostgreSQL migration now.

## Decision

### 1. Engine

PostgreSQL 17 replaces MariaDB 11.8 as the engine of `oteryn_platform`. The move runs through the ordered packets in `PLATFORM_POSTGRES_MIGRATION_PLAN.md`. MariaDB stays the deployed engine until the staging cutover (P6) and is removed in P7.

### 2. Owner rationale

The owner's words: "it has to be done sometime, and right now it is all test data".

- **The move is cheapest now.** Every row in every Platform environment is test data. The largest costs in the review are moving data with downtime, planning rollback, and re-qualifying against stored state. They are at their minimum today. They grow with every real account, wallet entry, Bazaar step and Game receipt that refers to Platform state. The owner treats the move as inevitable, and deferring it only raises its price.
- **A costly compatibility constraint is retired** (ADR-0013 §2). The only reason MariaDB was chosen, compatibility with the legacy engine's schema, is removed by #1474.
- **Expected operational simplification** (ADR-0013 §2). One engine and one backup and point-in-time-recovery toolchain (base backup plus WAL), which DUR-02 §11 already requires for Game. The backup work in review finding F-01 is then built once, on the target engine, instead of once for MariaDB and again after a later migration. P5 measures this benefit.
- **Correctness tools the Platform can use.** Transactional DDL, so a failed migration rolls back in full. `CHECK` constraints for non-negative balances where MariaDB relies on `UNSIGNED`, and partial unique indexes for "one active X" invariants.

This record does not claim more than the review supports. The benefits above are prospective, and the timing argument is the deciding one. The decision does not rest on stack uniformity. If P5 shows no operational simplification, the owner may revisit the decision before P7 removes MariaDB.

### 3. Ownership invariants are unchanged

Oteryn-Game ADR-0004 §2 and ADR-0013 §3 stay in force:

- separate logical databases `oteryn_platform` and `oteryn_game`;
- separate roles and credentials, with no credential of either side configured on the other;
- separate migration histories;
- no shared-table writers, no cross-database foreign keys or queries;
- cross-boundary state changes only through the versioned commands and receipts that already exist (`app/CharacterAuthority/`, `app/Marketplace/Native/`).

Running the same engine as Game gives neither side access to, or co-ownership of, the other's data.

### 4. Placement: separate instances, never one shared cluster

Platform and Game each run their own PostgreSQL instance: separate data directory, WAL stream, base backups, timeline, configuration, superuser and upgrade schedule. The Platform must not create its database inside Game's cluster, and Game must not host Platform data.

Reasoning:

- **PITR is cluster-wide.** A base backup plus WAL replay restores the whole cluster to one point in time. With a shared cluster, restoring one database rolls the other back too, or forces a slower logical restore.
- **DUR-02 needs independent restore.** DUR-02 §11 requires that a Game PITR restore reopens only behind a newer recovery fence. The Platform's native evidence witness (`docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md`) imposes the same rule on Platform restores. Each side must be restorable without moving the other.
- **Shared resources couple both sides.** A shared cluster also shares `max_connections`, memory, vacuum and checkpoint load, major-version upgrades and maintenance windows.

ADR-0004 §2 allows one cluster "for operational simplicity" as long as the option to separate is kept. This ADR declines that option for the Platform. On the single Synology NAS, the Platform instance runs in the Platform Compose project on its private network, with no published host port. Game's instance in Container Manager is not touched.

The owner may revisit placement through a superseding ADR that shows how independent PITR and restore fencing are kept.

### 5. Roles

The Platform instance has distinct principals:

- a **migration owner** role that owns the database and schema and is used only by the deploy's migration step;
- a **runtime** role with data privileges only: no schema ownership, no `CREATE`, no DDL;
- a **backup/replication** role;
- a superuser used only for bootstrap and never present in application configuration.

This closes review finding F-06, where the runtime user is the schema owner with `ALL PRIVILEGES`.

### 6. Transport

Platform connections to its PostgreSQL instance require TLS with server verification, even inside the Compose network. `pg_hba` admits only `hostssl` with SCRAM-SHA-256.

### 7. Backup gate

Until P5 has produced base backup plus WAL archiving to an off-NAS target and a dated, successful restore drill, the PostgreSQL-backed Platform is for testing only. That means no production go-live, no wallet or native Bazaar enablement and no payment activation on it. The owner sets the RPO and RTO (review decision D-2).

### 8. Transition rules

- From P1 on, CI runs the Platform test suite on PostgreSQL, and P1 makes that job blocking once P2 is green. New code and migrations must then work on both engines until P7.
- ADR 0044 allowlists exactly one preproduction store identity: driver `mysql`, host `mariadb`, port `3306`, database `oteryn_platform`. This ADR does not change that allowlist. P4 carries an ADR 0044 amendment that replaces the store identity, keeping one allowlisted store at a time, for owner acceptance. Until it is accepted, route publication stays off on the PostgreSQL store.
- `pdo_mysql` stays in the Platform image while the legacy-engine connections exist (#1474).
- Destructive steps, namely deleting the MariaDB volume, its secrets and its init scripts, happen only in P7, after the retention window set in P6.

## Consequences

Positive:

- one database engine and one PITR toolchain across the ecosystem, with independent instances;
- transactional migrations and declarative constraints for financial and state-machine invariants;
- least-privilege database principals delivered with the migration;
- review finding F-05 (wallet and bid concurrency proven only on SQLite) is closed on the target engine.

Negative and risks:

- Every MariaDB-specific path must be found and changed: lock-wait settings, isolation assumptions (InnoDB REPEATABLE READ versus PostgreSQL READ COMMITTED), vendor error codes, `UNSIGNED`, case-insensitive collation, `insertOrIgnore` semantics, NULL ordering, driver-gated code and the deploy scripts.
- A dual-engine period in CI and code, with its own cost.
- Staging downtime at cutover, and a rollback that must keep the native evidence witness consistent (see P6).
- The new deploy and backup surface must be qualified before production.
- This decision does not fix the other review findings (F-02 single-host placement, F-03 the mTLS deployable, F-07 the web tier, F-08 Redis, F-09 the scheduler). They keep their own packets.

## Alternatives considered

- **Stay on MariaDB 11.8 (review D-1(a)).** Rejected by the owner: it defers a cost the owner considers inevitable into a period with real data.
- **One PostgreSQL cluster shared with Game, with separate databases.** Rejected: PITR and restore fencing are cluster-wide (§4).
- **Defer without a record (D-1(c)).** Rejected: the engine choice affects every packet in the backup and least-privilege work.
- **A managed or off-NAS PostgreSQL service for the Platform.** Not decided here. Production placement remains review decision D-3. The plan is engine-only and runs on the NAS for staging.

## Reversal

The decision is reversible until P7. The MariaDB volume and a final logical dump are retained through the P6 window, and P6 defines the rollback. After P7, reversal would need a new migration.
