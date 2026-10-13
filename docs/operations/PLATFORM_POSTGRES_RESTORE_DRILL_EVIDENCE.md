# Platform PostgreSQL restore drill evidence

This file records dated, non-secret results of the PostgreSQL backup and point-in-time restore drill (plan P5, `PLATFORM_POSTGRES_MIGRATION_PLAN.md`; runbook `PLATFORM_POSTGRES_BACKUP_PITR.md`). It records no password, dump, backup content or connection string.

Classification:

- `LOCAL_REHEARSAL`: a disposable Docker host with the exact Compose service definitions and scripts of the commit under review. It proves the mechanism, not the NAS.
- `STAGING_PROVEN`: the `pitr-drill` mode of the *Recover Synology staging schema* workflow on the Synology staging stack, on an exact `main` SHA.
- `PRODUCTION_PROVEN`: not applicable until the encrypted off-NAS copy exists (`PRODUCTION_READINESS_CHECKLIST.md` §5).

Provisional staging targets: RPO 86400 s (24 h), RTO 14400 s (4 h).

## Staging drill

Status: **one passing staging drill recorded** (2026-10-12, below). P5 exit still requires 24 hours of `pg_stat_archiver` without failures; that record is not yet attached.

## 2026-10-12: staging drill (`STAGING_PROVEN`)

Source: *Recover Synology Staging Schema*, mode `pitr-drill`, run `38310896412` (attempt 1, `workflow_dispatch` on `main`), head `ba971b872ba243454b3a045d0b8ccb86bc8e68c7` (#1583). Job `pitr-drill` succeeded on runner `oteryn-synology-platform` (group `platform-runners`); job `recover` was skipped. Run window 2026-10-12 21:28:58Z to 21:31:58Z. Values below are the `evidence.env` keys printed in the job log (`pitr-drills/20261012T212920Z/evidence.env`).

| Key | Value |
| --- | --- |
| `RESULT` | `passed`, `CHECKS_PASSED=5` |
| `POSTGRES_IMAGE` | `postgres:17.11@sha256:2d2b8998…df9e3` |
| `ARCHIVER_HEALTHY` | `true`; forced segment `000000010000000000000006` |
| `BASE_BACKUP` | `20261012T211602Z`; `SHA256SUMS` and `pg_verifybackup` passed |
| `TARGET_TIME` / `REPLAYED_TO` | 2026-10-12 21:29:27 UTC / 21:29:24 UTC |
| `MARKERS` | A present, B absent |
| `TABLES` / `MIGRATIONS_HEAD` / `ROW_COUNTS` | 0 / absent / equal to the reference snapshot (the PostgreSQL database holds no Platform schema yet, engine `mariadb`) |
| `LEDGER_INVARIANT` | `not_applicable` |
| `DUMP_VERIFIED` | `20261012T211624Z` (`SHA256SUMS` and `pg_restore -l`) |
| `PLATFORM_IMAGE` / `PLATFORM_ENGINE` | `ghcr.io/oteryn/oteryn-platform@sha256:15e845d7…266c8f` / `mariadb` |
| `MIGRATE_STATUS` / `VERIFY_CONFIGURATION` | `not_applicable` (engine `mariadb`) / `passed` |
| `RESTORE_SECONDS` / `TOTAL_SECONDS` | 41 / 100 |
| `DATA_LOSS_BOUND_SECONDS` | 60 (basis: `archive_timeout` of a healthy archiver) against `RPO_SECONDS=86400` |
| `RTO_SECONDS` | 14400 (target); measured restore 41 s, total 100 s |

Limits: the drill ran against an idle database with no Platform schema, so it proves the mechanism and the NAS path but not a restore of Platform data. The post-cutover P5 drill (P6 validation) covers that.

## 2026-10-12: local rehearsal (`LOCAL_REHEARSAL`)

Environment:

- Docker Engine 29.8.2 and Compose 5.6.0, Compose project `oteryn-p5test`.
- Services `tls-init`, `postgres` and `postgres-backup` from `deploy/synology/compose.yml` at the P5 branch head, pinned image `postgres:17.11@sha256:2d2b8998…df9e3`.
- The P4 role reconcile, then `backup-postgres.sh reconcile`.
- An `oteryn_platform` schema of four tables, including the wallet tables and a `migrations` table.
- The sandbox disk was small, so the harness lowered the space floors: `OTERYN_BACKUP_MIN_FREE_MB=64`, `OTERYN_BACKUP_WAL_RESERVE_MB=16`, `OTERYN_ARCHIVE_MIN_FREE_MB=32`, `OTERYN_DRILL_WAL_MARGIN_MB=64`.
- No Platform image was available, so every drill ran with `--skip-platform-checks`: `migrate:status` and `production:verify-configuration` were not exercised.

| Check | Result |
| --- | --- |
| Spool preflight with too little space (129 MiB free, 162 MiB required) | Reconcile refused, exit 1, `result=refused` |
| Reconcile: credentials, preflight, forced `pg_switch_wal()` segment in the spool, base backup, dump, strict check | Passed |
| `backup-postgres.sh check` | Exit 0 |
| `check` with `OTERYN_BACKUP_MAX_AGE_SECONDS=1` | Exit 3 (degraded) |
| Sidecar recreated when the staged job script changes | Passed |
| Probe drill: marker A present and B absent, row counts and migrations head equal to the snapshot, ledger invariant, dump verified | `RESULT=passed`, restore 5 s, 5 checks, data-loss bound 60 s (archive_timeout of a healthy archiver), total 10 s |
| `--target-time` 30 s in the past | `RESULT=passed`, restore 5 s |
| `--target immediate --backup <stamp>` (base backup alone) | `RESULT=passed`, restore 4 s, data-loss bound = age of the base backup |
| `--target-time` in the future | Refused: `--target-time must be in the past` |
| A target past the end of the archive (before the past-only rule) | Failed fast, `recovery stopped before the target`, scratch discarded |
| Scratch containers left after all drills | 0 |

Every drill verified the base backup with `SHA256SUMS` and `pg_verifybackup` before replay.

What the rehearsal does not prove:

- NAS space, NAS Docker behaviour, or the `platform-runners` runner.
- The Platform image checks.
- A 24-hour archiver record.
- A restore from an off-NAS copy.
