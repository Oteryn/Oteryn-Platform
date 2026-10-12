# ADR 0044 — Preproduction-store native route publication

## Status

Accepted — 2026-10-11 (owner decision, option a)

- Decision record: `ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1` amendment
- Decision owner: repository owner
- Governing Issue: #1474
- Applies to: the two existing operator commands `game-auth:native-topology:issue` and `game-auth:native-route:publish`, when they run against the Synology preproduction MariaDB store
- Does not authorize: production or any other deployed environment, any database other than the allowlisted store, more than one native World route record, a change to `DisposableNativeStore` beyond Amendment D1023 below, a change to the gateway or admission contract, or any secret, credential or key handling
- Amended: 2026-10-12 by owner decision D1023 (testing-only loopback PostgreSQL profile of `DisposableNativeStore`, ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 §2 amendment)
- Proposed amendment: P4 — PostgreSQL store identity of the preproduction-store mode (2026-10-12, below; status Proposed until the owner accepts it)

## Context

Native login on the Synology preproduction stack needs one published native route record: the `native_route_*` columns on the `game_channels` row of the configured native World (contract `OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` §17). The operator commands that write that record are fenced by `DisposableNativeStore`, which admits only a retained per-run SQLite file (and, in tests, the in-memory and loopback concurrency stores). The fence therefore refuses the deployed MariaDB store, and the deploy step that publishes the route (default OFF behind `OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED`) cannot succeed.

`ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1` §2 framed three options. On 2026-10-11 the owner chose option a: a strictly fenced, separate preproduction mode inside the existing commands, leaving the disposable-store fence untouched.

## Decision

The two commands gain a **preproduction-store mode** with its own fence, `App\GameAuth\Worlds\PreproductionNativeRouteStore`. `DisposableNativeStore` is not changed and still guards every other path.

### Entry

The mode is entered only by the explicit switch `game-auth.native_route_preproduction_store.enabled` (environment `GAME_AUTH_NATIVE_ROUTE_PREPRODUCTION_STORE_ENABLED`, default off). Unset, `false` or `"false"` keeps the unchanged disposable path. `true` or `"true"` enters the mode. Any other value refuses (`NATIVE_PREPROD_STORE_SWITCH_INVALID`) instead of falling back. There is no command flag, so the deploy invocation does not change. On Synology the switch follows `OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED` (`deploy/synology/compose.yml`), so it is on exactly when route publication is enabled.

### Fence

Once the mode is entered, every condition must hold before any read or write of the store. Each violation fails closed with a named reason and nothing written:

| Condition | Refusal |
| --- | --- |
| `APP_ENV` is exactly `preproduction` (not `testing`, `local`, `staging`, `production` or anything else) | `NATIVE_PREPROD_STORE_ENVIRONMENT_REFUSED` |
| `game-auth.native_admission.unverified_character_world_id` (from `OTERYN_STAGING_NATIVE_WORLD_ID`) is a canonical lowercase UUIDv7 | `NATIVE_PREPROD_STORE_WORLD_UNCONFIGURED` |
| No outer transaction is open on the default connection | `NATIVE_PREPROD_STORE_TRANSACTION_OPEN` |
| The default connection is driver `mysql`, host `mariadb`, port `3306`, database `oteryn_platform`, no unix socket and no read/write split | `NATIVE_PREPROD_STORE_DATABASE_REFUSED` |

The host, port and database allowlist is code, not configuration, so no environment variable can widen it. Widening it needs a new decision.

### Scope: exactly one route record

Inside one transaction, with the World row (and Channel row) locked:

- **Topology issue.** A missing World row refuses (`NATIVE_PREPROD_STORE_WORLD_ROW_MISSING`). Another World row that already holds a WorldId refuses (`NATIVE_PREPROD_STORE_FOREIGN_WORLD`). A row with no WorldId gets one minted, as on the disposable path. The operator compares it with the configured World and copies it into `OTERYN_STAGING_NATIVE_WORLD_ID` when they differ. A retained WorldId other than the configured one refuses (`NATIVE_PREPROD_STORE_WORLD_MISMATCH`) and names the stored WorldId. The Channel is created when absent.
- **Route publish.** It publishes only for the World row whose WorldId equals the configured World (otherwise `NATIVE_PREPROD_STORE_WORLD_MISMATCH`). The Channel must already be issued (`NATIVE_PREPROD_STORE_CHANNEL_NOT_ISSUED`). No other World may hold a WorldId (`NATIVE_PREPROD_STORE_FOREIGN_WORLD`), and no other Channel may hold a route record (`NATIVE_PREPROD_STORE_FOREIGN_ROUTE`). The update must touch exactly one row.

### Idempotence

The route version and revision rules are those of the disposable path: same endpoint and same login flag gives result `unchanged` with no write; an endpoint change advances the version; a login-flag change alone keeps the revision. Re-running issue on a retained World and Channel gives `retained`.

### Audit and output

Every successful run writes one actor-less `admin_audit_events` row in the same transaction:

- `game_auth.native_topology.preproduction_store_issue`, target `native_world`
- `game_auth.native_route.preproduction_store_publish`, target `native_channel`

The metadata holds identifiers, route version and revision, login flag and result only. A structured log line records completion. Command output is one JSON object of identifiers, route fields, `store: "preproduction"` and `result`. Refusals print the reason code and a fixed message. Neither output nor audit carries a credential, connection string or database host.

## Consequences

- With the switch off, both commands behave exactly as before and refuse the deployed store through `DisposableNativeStore`.
- The deploy route-publication step works once `OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED` and `OTERYN_STAGING_NATIVE_WORLD_ID` are set and the image carries this mode.
- A second native World or Channel route on the store needs a new decision. The fence refuses it today.

## Rollback

Set `OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED` to false (or unset it) and redeploy. The mode is then unreachable, and the published route record stays as data. Login can be closed without removing the record by publishing `--login-enabled=false`. A Platform image built before this change does not have the mode, and the deploy step then fails closed through `DisposableNativeStore`.

## Verification

`tests/Feature/GameAuth/NativeTopology/PreproductionStoreRouteCommandsTest.php` covers four things:

- The mode works only when every condition holds.
- Each violated condition refuses with its named reason and nothing written.
- The world and route scope refusals.
- The default path, which stays unchanged with the switch off.

`NativeOperatorCommandsTest` keeps covering the disposable path.

## Amendment D1023 — testing-only loopback PostgreSQL profile (2026-10-12)

### Context

ADR 0046 moves Platform persistence to PostgreSQL, and its P2 packet runs the whole test suite on PostgreSQL 17 (`phpunit.pgsql.xml`). `DisposableNativeStore::connection()` admitted only SQLite and the loopback MariaDB `oteryn_concurrency` database, so every native topology, route-record and issuer test refused on PostgreSQL. ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 §2 and this ADR said the guard may not be relaxed or forked. On 2026-10-12 the owner chose (answer "1a") to amend both narrowly rather than skip those tests or leave them failing.

### Decision

ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 §2 is amended: `DisposableNativeStore::connection()` admits one more profile, and nothing else changes. The profile is fixed in code, and no environment variable can widen it. All of these must hold, or the guard fails closed with a `LogicException` and nothing is read or written:

| Condition | Refused otherwise |
| --- | --- |
| `APP_ENV` is exactly `testing` | `preproduction`, `production`, `staging`, `local` and every other environment |
| Driver `pgsql` on the default connection | (other drivers keep their existing branches) |
| Database name exactly `oteryn_test` | every other name, including `oteryn`, `oteryn_platform`, `oteryn_concurrency` and case or whitespace variants |
| Host exactly `127.0.0.1` or `localhost` | every other host, `::1`, a socket path, an empty or missing host, and a host list |
| No `read`/`write` split on the connection | any split, even to loopback |
| No outer transaction (the existing check) | an open transaction |

- The profile carries no secret. The guard reads no password, and its refusal messages name no host, database user or credential.
- `retainedRun()` is unchanged: the operator commands (`game-auth:native-topology:issue`, `game-auth:native-route:publish`, `game-auth:native-trust:publish-key`) still require the retained per-run SQLite file and refuse PostgreSQL.
- The MariaDB and SQLite branches are unchanged, byte for byte.
- The preproduction-store mode of this ADR is unchanged. Its PostgreSQL store identity remains the P4 amendment of ADR 0046.

### Consequences

- The native topology, route-record and issuer tests run on PostgreSQL in `testing` against the CI service database, as they already do on SQLite.
- No deployed environment can reach the profile: preproduction and production refuse it by environment, and the Synology database names differ.

### Verification

`tests/Feature/GameAuth/NativeTopology/DisposableNativeStorePostgresProfileTest.php` proves the admitted profile and each refusal (host, database, environment, read/write split), that the operator commands still refuse it, and that no refusal message carries a credential or host. It uses a lazy probe connection and so runs on every suite engine.

## Amendment P4 — PostgreSQL store identity (2026-10-12)

### Status

Proposed — 2026-10-12. Packet P4 of `docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md` (ADR 0046 §8). Takes effect when the owner accepts it; until then the deploy keeps it unreachable (below).

### Context

ADR 0046 moves Platform persistence to PostgreSQL, and packet P4 adds the Synology PostgreSQL service beside MariaDB. The fence above admits only driver `mysql`, host `mariadb`, port `3306`, so after the staging cutover (P6) the route-publication step would refuse the deployed store. The four preproduction-store tests (`PreproductionStoreRouteCommandsTest`) also failed on the PostgreSQL test suite (`phpunit.pgsql.xml`), because their store double could only take the MariaDB identity.

### Decision

The database condition of the fence becomes one identity per driver, still fixed in code:

| Driver | Host | Port | TLS | Database |
| --- | --- | --- | --- | --- |
| `mysql` (removed in P7) | `mariadb` | `3306` | unchanged | `oteryn_platform` |
| `pgsql` | `postgres` (the Compose service) | `5432` | `sslmode` exactly `verify-full` | `oteryn_platform` |

- The driver of the default connection selects the identity. A driver crossed with the other identity's host or port refuses, as does every other driver.
- Every other condition is unchanged: `APP_ENV` exactly `preproduction`, a canonical UUIDv7 World, no outer transaction, no unix socket and no read/write split. Each violation refuses with `NATIVE_PREPROD_STORE_DATABASE_REFUSED` (or the existing named reason) and nothing written.
- No environment variable widens the allowlist. A deployment has one default connection, chosen by `PLATFORM_DB_ENGINE` in `deploy/synology/release-contract.env`, so exactly one identity is reachable at a time.
- Scope, idempotence, audit and output are unchanged.
- `DisposableNativeStore` is unchanged by this amendment.

Until this amendment is accepted and P6 lands, `deploy/synology/scripts/deploy.sh` refuses `PLATFORM_DB_ENGINE=postgres`, and refuses it together with `OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED=true` in any case, so no deployed run can enter the PostgreSQL identity.

### Consequences

- The preproduction-store tests run on both suite engines. The store double now takes either identity.
- After the cutover the route-publication step works against the PostgreSQL store only over verified TLS to the Compose service.
- Removing the MariaDB identity is part of P7.

### Rollback

Revert this amendment's code change; the fence then refuses every `pgsql` connection again. Data written by an earlier run stays.

### Verification

`tests/Feature/GameAuth/NativeTopology/PreproductionStoreRouteCommandsTest.php` adds:

- the PostgreSQL identity issuing and publishing exactly one route record over `verify-full`;
- refusals for every other TLS mode or a missing mode, a crossed host or port in either direction, loopback, another port or database, a unix socket and a read replica.

The existing four tests keep their assertions and now pass on SQLite and on PostgreSQL. `tests/ci/test_synology_postgres_contract.py` covers the deploy-side refusal.

## Related records

- ADR 0029 — Platform World/Channel identity and topology
- `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` §17
- Issue #1474
- ADR 0046 — Platform persistence moves to PostgreSQL (Amendment D1023)
- `docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md` — packet P4 (Amendment P4)
