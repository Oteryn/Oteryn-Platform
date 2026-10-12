# Oteryn Platform architecture and database review — 2026-10-12

## Verdict

**KEEP MARIADB 11.8; FIX OPERATIONS BEFORE CHANGING ENGINES.**

The Platform's module split, its ownership boundary with Oteryn Game and its concurrency design are sound. The material risks are operational: the repository contains no evidence of scheduled, off-host or point-in-time-recovery backups for Platform data, and no dated restore. NAS and runtime state were not inspected, so whether such backups exist is UNKNOWN; an environment check is required before concluding they are absent. The only scheduler container runs only when the Marketplace is enabled. The Game-to-Platform mTLS boundary has no deployable Platform-side terminator. Most of the default test suite runs on SQLite, which does not reproduce row locks. None of the ADR-0013 §2 criteria for a PostgreSQL migration is met today. A migration would add cost and risk, and it would not fix the backup gaps that matter most now.

This is an independent, documentation-only review. It changes no code, configuration, schema, deployment or secret, and it authorizes nothing. Owner decisions are listed in §7.

## 1. Baseline and method

- Platform: `Oteryn/Oteryn-Platform` `main` at `b2e86fae` (PRs #1556 and #1557 merged) on 2026-10-12. Laravel `^13.8`, PHP `^8.5` (`composer.json`).
- Game: `Oteryn/Oteryn-Game` `main` at `08e13f7de`. Read-only inspection of Oteryn-Game was performed under the owner's Platform-Game integration mandate, tracked in https://github.com/Oteryn/Oteryn-Game/issues/1622; no Game file was changed. It was read only for the accepted ADR-0004, ADR-0013, DUR-02 §11, the Synology Game deploy README and the 2026-10-06 preproduction topology review.
- Governing issue: #1474 (native-only migration). Its programme direction is that Platform owns Identity, web, CMS, admin and commerce. Game owns Character, gameplay and content authority.
- Method: read the architecture authorities (`SYSTEM_ARCHITECTURE.md`, `MODULE_CATALOG.md`, `DATA_OWNERSHIP.md`, ADR 0001 and ADR 0016), the deployment sources under `deploy/synology/`, `config/`, `phpunit.xml` and the CI workflows. Computed the cross-module `use App\<Module>\…` graph over `app/`. Located the specific code paths cited below. No runtime, NAS or production state was inspected, so every operational statement is about what the repository proves.

Severity scale: **P0** stops current work or causes live loss now. **P1** blocks go-live or wallet/Bazaar enablement, or is a security or authority gap that needs an owner decision soon. **P2** is material debt or a defence-in-depth gap. **P3** is hygiene or documentation drift.

## 2. Current topology (as the repository proves it)

```text
Synology NAS (single host, /volume1)
├── Docker Compose project oteryn-staging (APP_ENV=preproduction)   deploy/synology/compose.yml
│   ├── mariadb:11.8       volume mariadb_data        Platform DB, app user = schema owner
│   ├── redis:7.4 (AOF)    volume redis_data          provisioned; no Platform consumer (F-08)
│   ├── platform           php artisan serve :8000    file sessions, file cache, sync queue
│   │                       volume platform_storage    sessions, cache, native-evidence witness
│   ├── internal-proxy     nginx TLS :8443            Gateway -> Platform (server TLS only)
│   ├── gateway            :8080                      service token to Platform
│   └── marketplace-scheduler (overlay, only when MARKETPLACE_ENABLED=true)
├── Container Manager: PostgreSQL 17 (TLS)  BASE/db    Game DB (oteryn_game), owned by Game
└── oteryn-game-server (supervisor.sh, runner uid 1001)  -> Platform internal mTLS :8543 (expected)
Cloudflare Tunnel -> 127.0.0.1:8000 (public website and every /internal/* route on one listener)
```

Evidence: `deploy/synology/compose.yml:19-179`, `deploy/synology/compose.marketplace.yml`, `deploy/synology/PUBLIC_ENDPOINTS.md:5-19`, `deploy/synology/docker/platform.Dockerfile:84-87`. Game side: the Game repository's `deploy/synology-game/README.md` (step 1, "NAS values") and `deploy/synology-game/preprod-topology.toml` (`[platform] internal_mtls_port = 8543`).

The topology already meets the core of ADR-0004 §2 and ADR-0013 §3. There are separate logical databases, separate engines and clusters, separate credentials and migration histories, and no cross-database foreign keys. Cross-boundary mutation goes through versioned commands and receipts (`app/CharacterAuthority/`, `app/Marketplace/Native/`). All of it shares one physical host, one disk pool and one operator.

## 3. Module boundaries and layering

### 3.1 What is sound

- **Modular monolith (ADR 0001) holds.** 32 top-level modules under `app/`. Controllers are concentrated in `app/Http` and depend on module services. Most domain modules depend only on `Identity` and `Audit`.
- **Wallet/Marketplace/Payments layering follows `SYSTEM_ARCHITECTURE.md` "Dependency rules".** `Payments` depends only on `Identity` and `Audit`. `Wallet` does not depend on `Payments`. `Marketplace` reaches Game through `CharacterAuthority` (9 imports) and not through Game storage.
- **Native Bazaar step ledger** (`app/Marketplace/Native/BazaarOwnershipTransfers.php`, `database/migrations/2026_10_11_180000_create_character_auction_transfers.php`). Each saga step attempt is bound to one deterministic operation id (`bazaar:{auction}:{purpose}:{attempt}`). The result comes only from Game's stored receipt, and rollback is refused while rows exist. This is the right shape for cross-database settlement.
- **Pessimistic locking is used consistently** on currency, ticket, admission, topology and support mutations (`lockForUpdate`/`sharedLock` in 70+ files). Lock waits are bounded for the Gateway-held admission call (`app/GameAuth/NativeLogin/NativeAdmissionAttempts.php:115-145`).
- **The native evidence producer has an anti-rollback witness** kept outside the relational restore unit (`docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md:35-43`). A restored database below the witness fails closed. This is the strongest restore-aware design in the repository.

### 3.2 Boundary findings

- **Identity depends on commerce.** `app/Identity/Termination/IdentityTerminationGuard.php:7-8` imports `App\Marketplace\Models\CharacterAuction` and `App\Wallet\Models\WalletAccount` and queries auction states directly. `SYSTEM_ARCHITECTURE.md` says "`Identity` must not depend on payment settlement, product fulfilment or commerce delivery". The guard's intent is correct (block termination while value is in flight), but the dependency points the wrong way. (F-10)
- **Wallet depends on Marketplace.** `app/Wallet/WalletMutator.php:5,29,59` and `app/Wallet/Actions/AdjustWalletBalance.php:7` throw `App\Marketplace\Exceptions\MarketplaceException`. Wallet is the lower layer (Marketplace "uses Wallet services") and should own its own exception vocabulary. (F-10)
- **Identity and GameAuth import each other.** `app/Identity/Actions/RevokeIdentityGameAuthorizations.php:6-9` imports four GameAuth classes, and GameAuth imports Identity 11 times. The revocation coupling is intentional (one locked mutation advances the native security generation), but it is undeclared. (F-11)
- **GameAuth is the largest security module and is not catalogued.** 85 files in ten sub-areas: tickets, native login, admission, evidence, OAuth, device sessions, worlds/topology, runtime status, account characters and bootstrap intent. `MODULE_CATALOG.md` has no `GameAuth`, `CharacterAuthority`, `GameProjections` or `Localization` section. It lists `Payments` and `Products and Entitlements` as "planned", although `app/Payments` (24 files) and `app/ProductsEntitlements` (16 files) are implemented. The catalogue is the boundary authority ADR 0001 relies on, so drift here weakens every later boundary review. (F-11)

### 3.3 Security and authority boundaries to the Game node and the Gateway

- **Gateway → Platform.** A static service token, compared as SHA-256 with a previous-token rotation slot (`compose.yml:88-89`), sent over the internal nginx TLS proxy with a stack-generated CA (`deploy/synology/nginx/internal.conf`). This is adequate for one host. The token is the only factor.
- **Game node → Platform (mTLS).** Seven middlewares trust `SSL_CLIENT_VERIFY`, `SSL_PROTOCOL=TLSv1.3` and an exact `SSL_CLIENT_S_DN` from the server environment. Examples are `app/CharacterAuthority/Http/GuardCharacterCommandPeer.php` and `app/Http/Middleware/GameAuth/RequireNativeEvidenceMtlsPeer.php`. The producer contract requires these values to come from a "separately qualified TLS terminator/FastCGI boundary" (`OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md:65`). The Synology stack has no such terminator. It runs the PHP built-in server and its internal proxy neither verifies client certificates nor passes `SSL_CLIENT_*`. Nothing in the Platform repository listens on the `:8543` port that Game's topology expects. The Game 2026-10-06 preproduction topology review specifies the missing pieces: an `internal-mtls` nginx on `:8543`, PHP-FPM, and an `edge-https` listener that refuses `/internal/`. The guards fail closed, so this is an availability and ownership gap, not a bypass. (F-03)
- **Internal routes share the public listener.** `routes/internal.php` is mounted on the same application (`bootstrap/app.php:33`), and the Cloudflare Tunnel forwards to that listener (`PUBLIC_ENDPOINTS.md:10`). The Gateway admission endpoint is therefore reachable from the internet, protected only by its token. The repository shows no edge or listener rule that keeps `/internal/*` off the public origin. (F-07)

## 4. Database assessment

### 4.1 Platform workloads and what they need from the engine

| Workload | Data shape | Engine requirements | MariaDB 11.8 fit |
|---|---|---|---|
| Accounts / Identity / MFA / sessions inventory | narrow rows, unique email, generation counters | unique constraints, row locks, case rules | Good. Collation `utf8mb4_unicode_ci` gives case-insensitive uniqueness by default. |
| Wallet (Oteryn Coins) | balance projection + append ledger with unique idempotency keys | atomic row locks, non-negative balances, durable commit | Good. `UNSIGNED` columns enforce non-negative balances under strict mode. The SQLite test path does not enforce them (F-05). |
| Payments (deferred, ADR 0003/0021) | provider events, idempotent effects, reconciliation | unique keys, locks, auditable history | Good. Real-MariaDB concurrency tests exist (`tests/Feature/Payments/*MariaDbTest.php`). |
| Character Bazaar auctions / bids | state machine, current-leading-bid reservation, saga ledger | row locks, uniqueness per step, idempotency | Good. MariaDB has no partial unique indexes, so "one active X" invariants rely on locks or generated columns. |
| Support tickets / moderation | CRUD, retention | none special | Good. |
| CMS / Wiki / Events / Downloads | content, search by `LOWER(...) LIKE` | moderate text search | Adequate. Full-text search is a later concern either way (`FEDERATED_SEARCH_ARCHITECTURE.md`). |
| GameAuth tickets, admission, topology, projections | short transactions under Gateway timeouts, epoch fencing | bounded lock waits, consistent locked reads | Good. The code already accounts for InnoDB REPEATABLE READ and `innodb_snapshot_isolation` (`NativeAccountCharactersReadModel.php:56-58`). |

Platform is a control plane with modest volume. Its correctness comes from row locks, unique idempotency keys and short transactions, and InnoDB provides all three. The workloads need no PostgreSQL-only feature such as serializable snapshot isolation, exclusion constraints, deferrable constraints, row-level security or partial indexes.

### 4.2 MariaDB versus PostgreSQL for Platform

**PostgreSQL advantages that would apply:**

- Transactional DDL. A failed Laravel migration rolls back completely. On MariaDB, each DDL statement commits implicitly, so a multi-statement migration that fails part-way leaves partial schema. `deploy/synology/scripts/lib.sh:661-736` mitigates this with a pre-migration dump and `SCHEMA_STATE=unknown`, but cannot remove it.
- Partial unique indexes, CHECK and exclusion constraints, and deferrable FKs express state-machine invariants declaratively.
- One backup and PITR toolchain (base backup + WAL) shared with Game. DUR-02 §11 already requires this toolchain for the Game database.
- Matches the Game team's operational knowledge; one engine to patch, monitor and tune on the NAS.

**PostgreSQL costs and risks:**

- 68 migrations (`database/migrations/`) and MariaDB-specific code (`innodb_lock_wait_timeout` session handling, the `PreproductionNativeRouteStore::ALLOWED_HOSTS = ['mariadb']` allowlist, `utf8mb4_unicode_ci` case-insensitive uniqueness that PostgreSQL would need `citext` or expression indexes to keep).
- Every concurrency proof that now runs on real MariaDB would need to be re-qualified, along with the deploy, backup and rollback scripts and the CI service containers in about 20 workflows.
- A data migration with downtime and a rollback plan on a single NAS.
- No user-visible gain, and no fix for the backup, scheduler or mTLS gaps.

**MariaDB costs that remain if Platform stays:**

- Two engines and two backup toolchains on one host (ADR-0013 "Costs and risks").
- Non-transactional DDL.
- Fewer declarative constraint options.

All three are bounded and already mitigated.

### 4.3 ADR-0013 §2 criteria for a Platform PostgreSQL migration

ADR-0013 §2 lists six kinds of material benefit and adds that "language or database-stack uniformity alone is not sufficient justification".

| ADR-0013 §2 criterion | Status | Evidence |
|---|---|---|
| Stronger correctness or concurrency behaviour needed by Platform workloads | **Not met** | No Platform invariant is shown to need a PostgreSQL-only mechanism. Real-MariaDB concurrency tests pass for payments, game tickets, topology and transfers. The gap that does exist (F-05) is about test coverage, not the engine. |
| Measurable operational simplification | **Not met (unmeasured)** | One PITR toolchain for both databases is plausible, but no Platform or Game backup toolchain exists yet to compare (F-01), so nothing has been measured. |
| Security / isolation improvement | **Not met** | The real isolation gap is that the runtime user is the schema owner with `ALL PRIVILEGES` (F-06). MariaDB can fix that with a split migration principal. |
| Maintenance / support reduction | **Not met** | MariaDB 11.8 is a long-term-support series and Laravel supports both engines as first-class. |
| Retirement of a costly compatibility constraint | **Partially, and pointing the other way** | MariaDB was originally chosen for compatibility with the legacy engine's shared schema. #1474 is retiring that constraint. That removes a reason to stay on MariaDB, but does not by itself create a benefit from moving. |
| Business / availability requirement that justifies the risk | **Not met** | No Platform RPO or RTO has been set (`PRODUCTION_READINESS_CHECKLIST.md:106-109`, `PRODUCTION_TOPOLOGY_EVIDENCE.md:236-241`). |

**Recommendation: stay on MariaDB 11.8.** Do not open a PostgreSQL migration now. Record the decision in a Platform ADR (Owner decision D-1). Reopen it only on a measured trigger:

1. the Platform RPO/RTO cannot be met with MariaDB binlog PITR on the chosen backup target;
2. a Platform invariant needs a PostgreSQL-only mechanism and a defect or design proves it;
3. one engine measurably reduces operator load after both backup toolchains exist and have been exercised; or
4. after the #1474 legacy retirement, a new consumer requires a shared database cluster with Game.

### 4.4 One engine or two; one cluster or separate

- **Two engines are acceptable** (ADR-0013 §3: "heterogeneous database technologies without changing semantic ownership"). The cost is two backup and monitoring toolchains, and F-01 has to be paid for each.
- **Keep separate instances regardless of engine.** DUR-02 §11 requires that a Game PITR restore can reopen only behind a newer recovery fence. Platform's native evidence witness requires the same of Platform restores. Independent instances let either side be restored without rolling back the other. Even if Platform later moved to PostgreSQL, it should not share Game's cluster on the single NAS: a shared cluster couples base-backup and WAL timelines, upgrades, `max_connections` and maintenance windows. ADR-0004 §2 allows one cluster only "for operational simplicity" and requires the option to separate.
- **Same host is the residual risk.** Both databases, the witness volume and every pre-migration dump sit on one NAS volume (F-02).

### 4.5 Cross-database settlement saga (Bazaar)

The saga design is engine-independent and correct in principle. Platform owns the commercial state and the wallet ledger in one Platform transaction. Game owns the ownership change. Steps join through deterministic operation ids and Game-stored receipts (ADR 0016 §7, `app/Marketplace/Native/`). There is no distributed transaction, and nothing is needed from the engine.

The open risks are elsewhere:

- **Restore divergence.** If Platform is restored to an earlier point, Game may hold COMMITTED transfer receipts for steps whose Platform ledger rows no longer exist, and the reverse also happens. Deterministic operation ids let a re-issued step find the same receipt only if the attempt number is the same. The ledger row that records it may be lost. No Platform procedure reconciles auctions, wallet holds and transfer steps against Game receipts after either side is restored (F-04).
- **Reconciliation needs a scheduler** (`marketplace:reconcile` every minute, `routes/console.php:162-164`). That is acceptable for the Bazaar itself, but the same container also carries Identity work (F-09).
- **Wallet concurrency has no real-database proof** (F-05) before the native Bazaar is enabled.

### 4.6 Migrations risk

- The deployment never rolls migrations back automatically. It takes a verified `mariadb-dump` and records evidence before each schema change (`deploy/synology/scripts/lib.sh:707-736`). Image rollback refuses schema reversal (`deploy/synology/scripts/rollback.sh:31`), and restore is evidence-gated (`deploy/synology/scripts/recover-schema.sh`). This is good practice.
- 56 of 68 migrations have a `down()` that drops tables or columns. That includes `2026_07_28_070000_create_wallet_tables.php:33-37`, which drops the wallet ledger without the "refuse while rows exist" guard that the newer Bazaar transfer migration uses. A manual `migrate:rollback` would destroy financial history (F-12).
- MariaDB DDL is non-transactional (see §4.2). The pre-migration dump is the only recovery for a half-applied migration, and that dump is on the same host (F-02).
- `config/database.php:4` defaults to `sqlite` when `DB_CONNECTION` is unset. `ProductionConfigurationVerifier::inspect()` never reads `database.default`, so nothing currently refuses that state; a verifier check must be added and tested (F-13).

### 4.7 Backup and restore gaps

- `docs/operations/INCIDENT_RECOVERY_RUNBOOK.md:184` says: "Current repository state does not prove that an operational backup/restore mechanism exists. Until a dated restore test succeeds, production readiness remains blocked." The brief cited line 188; the sentence is at 184 on the reviewed `main`.
- What exists:
  - a pre-migration logical dump per release, written to `/var/lib/oteryn-staging-state/backups/` on the NAS (`lib.sh:707-729`);
  - a CI-runner dump/restore drill on a near-empty schema (`.github/workflows/phase7-production-like-validation.yml:294-340`; `PRODUCTION_READINESS_CHECKLIST.md:42` records 105 ms and 13 tables), which proves the mechanism but not the operations.
- What is missing from the repository (NAS and runtime state were not inspected, so presence in the environment is UNKNOWN; require an environment check before concluding any of these is absent):
  - scheduled backups;
  - binary-log retention for PITR;
  - an off-host or off-NAS copy;
  - encryption and an access policy for dumps (dumps are `chmod 600` on the same host);
  - a retention policy;
  - a restore drill on representative data with ledger and integrity checks (for example, the wallet balance equals the sum of ledger deltas);
  - backup of the `platform_storage` witness volume and the external `native_admission_keys` volume, with a documented restore order;
  - an RPO/RTO.
- Game has the same gap on the same host. DUR-02 §11 requires "PostgreSQL backup + WAL/PITR capability, automated restore verification". The Game README documents a projection epoch fence for restores but no backup job.

### 4.8 Redis role

After PR #1556, no Platform code or config reads Redis, and #1557 removed the Redis service from most acceptance workflows. `config/database.php` has no `redis` connection, and `CACHE_STORE=file`, `SESSION_DRIVER=file` and `QUEUE_CONNECTION=sync` are set (`compose.yml:73-78`). Yet the stack still provisions `redis:7.4` with AOF and a password (`compose.yml:37-54`). Platform waits for it to be healthy (`compose.yml:113-114`), and `deploy.sh` requires `REDIS_PASSWORD` and starts and preserves it (`deploy/synology/scripts/deploy.sh:21,35,311,425,727`). The `deploy/synology/README.md:20` diagram still shows "Platform <-> Redis". Redis is now a dead dependency: it can block Platform startup, it carries a secret and it holds data nobody owns (F-08).

Redis remains a reasonable *future* choice for cache, sessions, rate limiting and queues once Platform runs more than one web process. It should return only through a decision that names its role and its durability class (cache-only, so no backup is needed).

### 4.9 SQLite in tests

`phpunit.xml:26-27` runs the default suite on in-memory SQLite. In SQLite, `lockForUpdate` and `sharedLock` compile to nothing, `UNSIGNED` is not enforced, and the default `BINARY` collation is case-sensitive, while MariaDB uses `utf8mb4_unicode_ci`. The MariaDB-only branches (`NativeAdmissionAttempts.php:125`) are skipped. The repository does run selected suites on real MariaDB 11.8: payments, game tickets, topology, bootstrap intent, legacy transfer and acceptance workflows. Marketplace and Wallet concurrency tests are not among them; `MarketplaceAuctionTerminalRecoveryConcurrencyTest` runs in-process on SQLite. SQLite stays valuable for fast feedback. It is not evidence for locking, constraint or collation behaviour (F-05).

### 4.10 Single-node NAS risks

- **Hardware and volume.** One host and one storage volume hold Platform MariaDB, the Game PostgreSQL, Redis, the witness volume, the issuer signing keys and every backup. Losing that volume loses everything, including the means of recovery (F-02).
- **Web tier.** `php artisan serve` without `PHP_CLI_SERVER_WORKERS` runs a single PHP worker (`platform.Dockerfile:87`). Public website traffic, the Gateway's admission calls with their 5 s timeout and internal routes all wait in that one queue (F-07). File sessions and file cache tie Platform to one container.
- **Operations.** NAS updates, Container Manager restarts and reboots take everything down together. The Game node restarts from a DSM boot task, and Platform uses `restart: always`; no ordering or health dependency between them is declared.

## 5. Findings

| ID | Sev | Finding | Evidence | Recommended action |
|---|---|---|---|---|
| F-01 | P1 | The repository contains no evidence of a scheduled, retained or PITR-capable backup for the Platform DB, nor of a dated restore on representative data; NAS/runtime state was not inspected, so presence is UNKNOWN. The CI drill proves the mechanism only. | `INCIDENT_RECOVERY_RUNBOOK.md:184`; `PRODUCTION_READINESS_CHECKLIST.md:106-109`; `lib.sh:707-729`; `phase7-production-like-validation.yml:294-340` | First run an environment check on the NAS (existing jobs, binlog retention, off-host copies) before concluding they are absent. Owner sets the RPO/RTO. Where the check finds nothing, add scheduled `mariadb-backup` or logical dumps plus binlog retention, a restore drill with ledger and integrity checks, and dated evidence. This gates go-live and wallet/Bazaar enablement. |
| F-02 | P1 | Single NAS holds both databases, the witness volume, the signing keys and all backups. The repository shows no off-host copy; whether one exists is UNKNOWN until the environment check. | `compose.yml:28,104,166-175`; `lib.sh:678,710`; Game `deploy/synology-game/README.md` step 1 and "NAS values" (`/volume1/oteryn/game-preprod`, `BASE/db`) | Encrypted off-NAS copy (cloud or second device) of DB backups, the witness volume and key escrow, on a separate schedule from DB backups. Restore order documented. |
| F-03 | P1 | Game→Platform mTLS boundary has no Platform-side deployable. The stack runs the PHP built-in server (no FastCGI, no `SSL_CLIENT_*`), nothing listens on `:8543`, and the internal proxy does not verify client certificates. All seven peer-guarded route groups fail closed on this stack. | `platform.Dockerfile:87`; `nginx/internal.conf`; `OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md:65`; Game `preprod-topology.toml` `[platform]`; Game review `OTERYN_GAME_ARCH_PLATFORM_PREPROD_TOPOLOGY_2026-10-06.md` §§5, 10 | One Platform-owned packet that implements the agreed preprod listener set: PHP-FPM, an `internal-mtls` nginx with TLS 1.3 and `ssl_verify_client on` that overwrites `SSL_CLIENT_*` as FastCGI params, and an `edge-https` listener that refuses `/internal/`. Qualify it with the interop checklist. |
| F-04 | P1 | No cross-system restore procedure. Platform restore steps cover only Platform-internal state. Bazaar steps and wallet holds against Game receipts, native evidence `reconcile` commands and the Game projection epoch fence are not tied together. | `INCIDENT_RECOVERY_RUNBOOK.md:172-182`; `OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md:41-43`; Game DUR-02 §11 steps 1-7; Game README step 6a | Joint Platform/Game restore runbook: fence admission, restore, run `game-auth:native-evidence:reconcile`, run the saga reconciliation against Game receipts, then reopen. Exercise it once on preprod. |
| F-05 | P1 | Wallet and bid concurrency has no real-MariaDB proof. The default suite on SQLite drops row locks and does not enforce `UNSIGNED`. | `phpunit.xml:26-27`; `tests/Feature/Marketplace/MarketplaceAuctionTerminalRecoveryConcurrencyTest.php`; the MariaDB-backed set is limited to payments, game auth, topology and legacy transfer tests | Add multi-process MariaDB tests for `PlaceAuctionBid`, `WalletMutator` and settlement before native Bazaar enablement. Also add a ledger-sum-equals-balance integrity check. |
| F-06 | P2 | The Platform runtime user is the schema owner (`GRANT ALL PRIVILEGES`) and the deploy dumps as `root`. This contradicts the migration-owner vs runtime-credential split in `DATA_OWNERSHIP.md` ("Database credentials"). | `mariadb/init/10-platform-database.sh:27`; `compose.yml:71`; `lib.sh:715-716` | Separate migration principal (DDL) and runtime principal (DML on Platform schema), plus a backup principal with `SELECT, LOCK TABLES, SHOW VIEW, EVENT, TRIGGER`. |
| F-07 | P2 | One single-worker built-in PHP server serves the public site, the Gateway admission API and internal routes behind the Cloudflare Tunnel. Public load can starve logins, and `/internal/*` is reachable from the internet behind one token. | `platform.Dockerfile:87`; `PUBLIC_ENDPOINTS.md:10`; `bootstrap/app.php:33`; `compose.yml:88-89` | PHP-FPM with a worker pool (it is also needed for F-03). Refuse `/internal/` on the public listener or at the edge. |
| F-08 | P2 | Redis is provisioned, password-managed and a hard start dependency, but has no consumer. | `compose.yml:37-54,113-114`; `deploy/synology/scripts/deploy.sh:21,35,311,425,727`; `deploy/synology/README.md:20`; `config/cache.php:6`, `config/session.php:6`, `config/queue.php:4` | Remove it from Compose and the deploy contract, or record a decision that gives it a named role (cache, session or queue; not durable). |
| F-09 | P2 | The only scheduler container runs only when `MARKETPLACE_ENABLED=true`. With the Bazaar frozen, `identity:finalize-terminations` (hourly) never runs, so account-termination finalization depends on a commerce flag. | `app/Providers/AccountSecurityServiceProvider.php:12-14`; `compose.marketplace.yml`; `lib.sh:568-586` | Always-on `scheduler` service in the base Compose file. Marketplace reconciliation is gated inside the command, not by the container's existence. |
| F-10 | P2 | Dependency inversions: Identity reads Marketplace and Wallet models, and Wallet throws Marketplace exceptions. | `IdentityTerminationGuard.php:7-8`; `WalletMutator.php:5`; `AdjustWalletBalance.php:7`; `SYSTEM_ARCHITECTURE.md` "Dependency rules" | A `TerminationBlocker` contract in Identity that Marketplace and Wallet implement (tagged container binding). A `WalletException` owned by Wallet. |
| F-11 | P3 | `MODULE_CATALOG.md` omits GameAuth, CharacterAuthority, GameProjections and Localization, and lists the implemented Payments and ProductsEntitlements as "planned". GameAuth (85 files, ten sub-areas) has no declared internal boundary, and the Identity↔GameAuth cycle is undeclared. | `MODULE_CATALOG.md` section list; `app/GameAuth/*` | Catalogue update. Consider splitting `Worlds` (registry and topology) and `NativeRuntimeStatus`/`NativeAccountCharacters` (projection ingestion) out of GameAuth. |
| F-12 | P3 | Destructive `down()` on financial and audit tables, without row guards. | `2026_07_28_070000_create_wallet_tables.php:33-37`; 56/68 migrations drop in `down()` | Refuse `down()` when ledger, audit or saga rows exist, following the `create_character_auction_transfers` pattern. |
| F-13 | P3 | `config/database.php` falls back to `sqlite` when `DB_CONNECTION` is unset. It still defines the two legacy-engine MySQL connections, which are retained under #1474 until retirement. | `config/database.php:4,33-66` | Add and test a `production:verify-configuration` check that refuses the SQLite default when `DB_CONNECTION` is omitted (`ProductionConfigurationVerifier::inspect()` does not read `database.default`). Remove the legacy connections in the #1474 retirement slice. |

**There is no P0.** Nothing in the repository shows live data loss or an exploitable bypass today. The environment is preproduction, payments are deferred, the Bazaar is frozen and the mTLS guards fail closed. F-01, F-02 and F-05 become release-blocking before production or wallet/Bazaar enablement.

## 6. Recommended sequence

1. **Backups first (F-01, F-02, F-04).** Run an environment check of the NAS for existing backup jobs, binlog retention and off-host copies; treat absence as unproven until then. Set the RPO/RTO. Schedule MariaDB backups with binlog PITR, and add the matching Game PostgreSQL job (DUR-02). Copy them off the NAS, including the witness volume and key escrow. Write a joint restore runbook and record one dated drill on each side.
2. **Web tier and the mTLS boundary (F-03, F-07).** One Platform deploy packet to PHP-FPM with `internal-mtls`/`edge-https` nginx, as the Game topology review specifies.
3. **Always-on scheduler, least-privilege DB principals, Redis removal (F-09, F-06, F-08).**
4. **Real-MariaDB wallet and bid concurrency tests and a ledger integrity check (F-05)** before any native Bazaar enablement packet.
5. **Boundary hygiene (F-10, F-11, F-12, F-13).**
6. **DB engine.** Record "stay on MariaDB" (D-1) and the reopen triggers in §4.3. Revisit after step 1 has produced measured backup and restore data for both engines.

## 7. Owner decisions

| ID | Decision | Options | Recommendation |
|---|---|---|---|
| D-1 | Platform database engine | (a) stay on MariaDB 11.8 and record it in a Platform ADR with reopen triggers; (b) plan a PostgreSQL migration now; (c) defer without a record | **(a).** No ADR-0013 §2 criterion is met (§4.3). |
| D-2 | Platform RPO/RTO and backup target | e.g. RPO 15 min / RTO 4 h with off-NAS encrypted storage; or a lower target for preprod only | Set preprod and production targets separately. Preprod needs at least daily backups plus binlog and an off-NAS copy. |
| D-3 | Same-host or separate-host placement for production | (a) keep both databases on one NAS; (b) a second host or managed database for Platform; (c) a NAS-to-NAS replica | Before go-live, at least (c) or off-host PITR. A single NAS is acceptable for preprod only. |
| D-4 | Redis role | (a) remove now; (b) keep as non-durable cache/session/rate-limit store with a named owner | **(a)**. Reintroduce Redis with a decision when Platform scales beyond one web process. |
| D-5 | Ownership of the Platform-side preprod listener set (FPM, `internal-mtls`, `edge-https`) | Platform deploy packet under #1474 or #1419 | A Platform packet, implementing the Game review's §10 layout verbatim. |
| D-6 | Scheduler placement | always-on base service, or keep it in the Marketplace overlay | Always-on base service. |
| D-7 | Whether to add database engine, backup and restore records to `ARCHITECTURE_DECISION_BACKLOG.json` | add records, or track in issues only | Add records: the backlog currently has no database or backup entry. |

## 8. Limits of this review

- It reads repository state only. It does not see NAS configuration, live volumes, Cloudflare rules or any untracked mTLS terminator on the host. If one exists, F-03 is then a source-control and qualification gap rather than a missing component.
- It does not measure performance. Engine statements are qualitative.
- Game-side statements are limited to the Game documents cited, read at the Game SHA above.
