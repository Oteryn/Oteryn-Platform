# Platform PostgreSQL cutover evidence

This file records the dated, non-secret results of the staging MariaDB to PostgreSQL cutover (plan P6, `PLATFORM_POSTGRES_MIGRATION_PLAN.md`). The cutover is driven one step per dispatch by the `cutover` action of the *Deploy Synology Staging* workflow, which runs `deploy/synology/scripts/cutover-postgres.sh`. It records no password, dump, token, cookie or connection string.

Status: **pending – not executed**. Every field below is a placeholder until the owner-approved cutover window runs. Dry-run dispatches (`cutover_dry_run=true`, the default) are not evidence and are not recorded here.

## Run header

| Field | Value |
| --- | --- |
| Task id | `OTERYN-YYYYMMDD-<slug>` (pending) |
| Window (UTC) | pending |
| Repository head (`main`) | pending, 40-hex SHA |
| Platform image | pending, `ghcr.io/oteryn/oteryn-platform@sha256:…` |
| Gateway image | pending, `ghcr.io/oteryn/oteryn-gateway@sha256:…` |
| Runner | pending |
| `MARKETPLACE_ENABLED` | `false` (checked by preflight) |

## Gate checks

| Check | Run id | Result |
| --- | --- | --- |
| Open-gate audit before preflight (`oteryn-edge-audit-result`, `gate_state=open`) | pending | pending |
| Maintenance-close run (`closure_run_id`) | pending | pending |
| Closed-gate audit (`closure_match=yes`) before freeze | pending | pending |
| External check: both canonical hosts return 503, a random label gets no 2xx/3xx | n/a | pending |
| Closed-gate audit before native re-enable / declare-done / retire | pending | pending |
| Maintenance-open run | pending | pending |
| Open-gate audit before unfreeze; both hosts reach their origins | pending | pending |
| `platform-cutover-gate-evidence` artifact (`cutover_complete` or `rollback_verified`) | pending | pending |

## Steps

| Step | Subcommand | Started (UTC) | Finished (UTC) | Duration (s) | Result |
| --- | --- | --- | --- | --- | --- |
| 0. Preflight | `preflight` | pending | pending | pending | pending |
| 1. Freeze | `freeze` | pending | pending | pending | pending |
| 2. Bootstrap (engine flipped to `postgres` by reviewed PR) | `bootstrap` | pending | pending | pending | pending |
| 3. Smoke | `smoke` | pending | pending | pending | pending |
| 4. Native re-enable (last rollback point passed when it starts) | `native-reenable` | pending | pending | pending | pending |
| 5. Native smoke | `native-smoke` | pending | pending | pending | pending |
| 6. Declare done | `declare-done` | pending | pending | pending | pending |
| 6. Retire MariaDB (owner-gated, destructive) | `retire-mariadb` | pending | pending | pending | pending |
| 7. Unfreeze | `unfreeze` | pending | pending | pending | pending |
| Rollback (only before step 4) | `rollback` | not used | not used | not used | not used |

Total frozen window (freeze to maintenance-open): pending. Rollback outcome, if any: pending.

## Bootstrap results

| Field | Value |
| --- | --- |
| `WorldId` (native world) | pending |
| OAuth client id (web) | pending, UUID |
| OAuth client id (Rust) | pending, UUID |
| Witness store id | pending, 64-hex |
| MariaDB-era witness moved aside to | pending, `<dir>.mariadb-era-<ts>` |
| Provenance init (P6a hook) | pending |
| First trust publication | pending |

## MariaDB retirement

| Field | Value |
| --- | --- |
| Owner delete confirmation supplied (phrase value is not recorded) | pending |
| `owner_delete_ack` names the task id | pending |
| Prerequisites: native smoke and declare-done recorded | pending |
| Volume deleted | pending, `<project>_mariadb_data` |
| Container removed | pending |
| MariaDB-era witness directory deleted | pending |
| MariaDB deleted at (UTC) | pending |

## Notes and deferrals

- Rollback is forward-only once step 4 starts; the recorded marker, not operator memory, enforces it.
- Until P7 removes `mariadb` from the compose and deploy path, an ordinary staging deploy would recreate the retired service and an empty volume; run none between retirement and P7.
- `health-check.sh` requires the MariaDB service, so health checks cannot run after retirement.
- Pending: record who approved the window and where the owner decision is archived (no personal data).
