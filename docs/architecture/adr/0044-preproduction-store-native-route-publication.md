# ADR 0044 — Preproduction-store native route publication

## Status

Accepted — 2026-10-11 (owner decision, option a)

- Decision record: `ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1` amendment
- Decision owner: repository owner
- Governing Issue: #1474
- Applies to: the two existing operator commands `game-auth:native-topology:issue` and `game-auth:native-route:publish`, when they run against the Synology preproduction MariaDB store
- Does not authorize: production or any other deployed environment, any database other than the allowlisted store, more than one native World route record, a change to `DisposableNativeStore`, a change to the gateway or admission contract, or any secret, credential or key handling

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

## Related records

- ADR 0029 — Platform World/Channel identity and topology
- `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` §17
- Issue #1474
