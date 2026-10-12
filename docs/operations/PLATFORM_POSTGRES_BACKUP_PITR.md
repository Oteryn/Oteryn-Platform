# Platform PostgreSQL backups and point-in-time recovery on the NAS

Status: staging mechanism (plan P5, `PLATFORM_POSTGRES_MIGRATION_PLAN.md`; ADR 0046 §7). Provisional staging targets: RPO 24 hours, RTO 4 hours. The owner sets the production values. Backups stay on the Synology NAS. **An encrypted off-NAS copy and a drill restored from it are required before any production use, real player, wallet or Bazaar value, or payment activation** (`PRODUCTION_READINESS_CHECKLIST.md` §5).

This runbook covers the PostgreSQL Platform database only. It holds no secret, password, dump or backup.

## Components

| Part | Where | What it does |
| --- | --- | --- |
| `archive_command` | `deploy/synology/postgres/backup/archive-wal.sh`, run by `postgres` | Writes each completed WAL segment to the spool as `wal/<segment>.gz` with a `.sha256` file. It refuses a bad name, a missing spool, conflicting content for an existing name, and a spool below 256 MiB free (`OTERYN_ARCHIVE_MIN_FREE_MB`). On refusal PostgreSQL keeps the segment in `pg_wal` and retries. |
| `restore_command` | `deploy/synology/postgres/backup/restore-wal.sh` | Restores a segment for recovery. An absent file exits 1, the normal end of the archive. A missing archive directory, a missing checksum, a checksum mismatch or a decompression failure exits 199, which aborts recovery, so a damaged archive never ends recovery early and silently. |
| `postgres-backup` sidecar | `compose.yml`, profile `postgres`, same pinned image as `postgres`, private network only, no port | Runs `backup-job.sh service`: a daily base backup and logical dump at `OTERYN_BACKUP_HOUR_UTC` (default 02:00 UTC), retention after each success, and a heartbeat every 60 seconds for its healthcheck. A failed job is retried after one hour at the earliest. |
| `backup-postgres.sh` | `deploy/synology/scripts/` | `reconcile` (every deploy), `check` (every health check), `run <job>` (operator). |
| `restore-postgres-pitr.sh` | `deploy/synology/scripts/` | The restore drill into a scratch instance. |

Roles (plan P4):

- The base backup uses `oteryn_platform_backup` (`LOGIN REPLICATION`, no table access).
- The dump uses `oteryn_platform_dump` (read-only: `SELECT` and default privileges).

Both connect over `sslmode=verify-full` with the P4 client CA. The deploy passes their passwords (`PLATFORM_POSTGRES_BACKUP_PASSWORD`, `PLATFORM_POSTGRES_DUMP_PASSWORD`) on standard input into a 0600 password file inside the sidecar's own filesystem. They never appear in the spool, an argument, the container environment or a log.

## Spool layout

The spool is the named volume `postgres_backup`, mounted at `/var/lib/oteryn-postgres-backup` in `postgres` and in the sidecar. It is never inside `postgres_data`. Directories are 0700, owned by `postgres` (uid 999).

```text
wal/<segment>.gz, wal/<segment>.gz.sha256      continuous WAL archive
base/<stamp>/  base.tar.gz pg_wal.tar.gz backup_manifest SHA256SUMS backup.info
dump/<stamp>/  oteryn_platform.dump oteryn_platform.toc SHA256SUMS dump.info
status/        heartbeat, base.last, dump.last, prune.last, job.lock
```

- `<stamp>` is `YYYYMMDDTHHMMSSZ` (UTC).
- A backup is complete only once its `backup.info` or `dump.info` exists. Jobs write into `.<stamp>.partial` and rename it at the end, and retention removes leftover partial directories.
- `backup_manifest` carries a SHA-256 checksum for every file of the cluster.
- `backup.info` records the cluster's `system_identifier`, the start WAL file, the end time and the sizes.

## Space

The owner provides the space. These numbers are a starting point; check them with `backup-postgres.sh run status` after a week.

- **Steady state.** About 11 x (compressed base backup + compressed dump) for 7 daily and 4 weekly, plus 7 days of compressed WAL. With `archive_timeout = 60s`, a nearly idle server still produces up to one 16 MiB segment a minute. Those compress to well under 1 MiB each, so allow about 1 GiB a day for an idle staging server, and more under load.
- **Preflight.** Before every base backup or dump, and on every deploy, the spool must have `max(OTERYN_BACKUP_MIN_FREE_MB, 2 x (database MiB + 64) + OTERYN_BACKUP_WAL_RESERVE_MB)` MiB free (defaults 2048 and 1024). When it does not, the job and the deploy reconcile refuse, and `check` reports the backups as degraded.
- **Archive floor.** `archive-wal.sh` refuses below 256 MiB free. WAL then accumulates in `pg_wal` on `postgres_data` until space returns. Watch for this: a full `postgres_data` stops the database.
- **Drill scratch.** The drill needs free Docker storage of at least the restored cluster size plus `OTERYN_DRILL_WAL_MARGIN_MB` (default 1024 MiB). It checks this before it restores.

## Schedule and retention

- **Daily jobs.** One base backup (`pg_basebackup -Ft -z -X stream`, SHA-256 manifest) and one dump (`pg_dump -Fc`). Each runs when the newest one is at least 20 hours old at the configured hour, or at once when none exists or the newest is 25 hours old.
- **Deploy.** The deploy reconcile (`ensure`) also takes one when the newest is 24 hours old or more.
- **One job at a time.** Every job takes the spool job lock (`status/job.lock`) and decides again under it whether its backup is still needed, so the sidecar loop and a deploy's `ensure` never both take the same day's backup.
- **Kept backups.** For base backups and dumps alike: the newest 7, plus the newest backup of each of the 4 most recent older ISO weeks.
- **WAL cutoff.** WAL files older than the start WAL of the oldest of the newest 7 base backups are removed, which reaches back about seven days. Timeline history files are never removed.
- **Weekly backups.** A weekly base backup restores to its own end only (`--target immediate`), because it carries its WAL in `pg_wal.tar.gz`.

## Deploy and health semantics

`deploy.sh` stages the scripts into the bootstrap volume and runs `backup-postgres.sh reconcile` right after the PostgreSQL role reconcile. The reconcile does the following:

1. Starts the sidecar. It recreates the sidecar when the staged job script differs from the copy the running loop uses.
2. Provisions the credentials.
3. Runs the space preflight. It refuses on insufficient space, an unreachable database or missing credentials.
4. Refuses when the spool's newest base backup belongs to another cluster. See *Another cluster* below.
5. Forces a WAL switch and waits for that segment in the spool.
6. Brings the base backup and dump to at most 24 hours old.
7. Runs `check` strictly.

Any failure fails the deploy.

The Platform fast path skips this reconcile. It is chosen only when `backup-postgres.sh check` currently passes strictly. A stopped sidecar, an earlier reconcile that failed, or a degraded backup sends the deploy down the full path, which runs the reconcile.

`health-check.sh` runs `backup-postgres.sh check`:

- **Exit 1, structural.** The sidecar is missing, stopped, unhealthy or publishes a port; `archive_mode` is off; or status cannot be read. The health check fails.
- **Exit 3, degraded.** The latest archive attempt failed; a `.ready` segment is older than 600 seconds; the base backup or dump is missing or older than 26 hours; the newest base backup is of another cluster; or the spool preflight is not `ok`. The health check prints a warning and passes, so a stale backup never blocks a rollback or candidate resolution. The next deploy reconcile repairs it or refuses.

Without the PostgreSQL secrets (before P4 is configured) every mode is a no-op.

Operator commands, run on the NAS from the deploy directory with the deploy `.env`:

```bash
OTERYN_ENV_FILE=/path/to/.env bash deploy/synology/scripts/backup-postgres.sh check
OTERYN_ENV_FILE=/path/to/.env bash deploy/synology/scripts/backup-postgres.sh run status
OTERYN_ENV_FILE=/path/to/.env bash deploy/synology/scripts/backup-postgres.sh run base   # or dump, prune, preflight
```

After a manual sidecar recreate (`docker compose up --force-recreate postgres-backup`) the password file is gone. Jobs then record `credentials_missing` until the next deploy reconcile, or until the operator re-runs reconcile.

## Restore drill

```bash
COMPOSE_PROJECT_NAME=oteryn-staging OTERYN_STATE_DIR=<runner state dir> \
  bash deploy/synology/scripts/restore-postgres-pitr.sh                        # probe drill (default)
bash deploy/synology/scripts/restore-postgres-pitr.sh --target-time '2026-10-12 08:00:00'   # UTC
bash deploy/synology/scripts/restore-postgres-pitr.sh --target immediate --backup 20261005T020000Z
```

The probe drill does the following:

1. Writes marker A into table `oteryn_pitr_drill` of the `postgres` maintenance database. This is the only write to the live server, and it never touches `oteryn_platform`.
2. Records target time T and a row-count snapshot of every `oteryn_platform` table.
3. Writes marker B, forces the WAL segment into the spool, and restores the newest base backup taken before T, from the live cluster, to T.

A `--target-time` drill takes a time in the past. Recovery stops only at a commit after the target, so the drill first writes marker C into the same maintenance table and archives its segment.

The restore runs in a scratch container from the same image. The container has no network (`--network none`), reads the spool read-only, uses its own anonymous data volume and runs with `--rm`. It holds a shared lock on the spool job lock for its whole life, so retention cannot remove the chosen base backup, its WAL or the checked dump during the drill. The drill first waits for a running backup job (at most `OTERYN_DRILL_LOCK_WAIT_SECONDS`, default 7200). A backup job, retention run or deploy `ensure` started during the drill waits for it, and fails after two hours, to be retried by the next loop or deploy. The drill then checks the following:

- the backup's `SHA256SUMS` and `pg_verifybackup`;
- promotion at the target;
- marker A present and B absent;
- row counts equal to the snapshot;
- the wallet ledger invariant, when the wallet tables exist;
- the migrations head;
- the dump's checksums and table of contents;
- with the running Platform image: `migrate:status` with nothing pending, and `production:verify-configuration`.

It records the restore time against the 4 h RTO and the data-loss bound against the 24 h RPO in `<state dir>/pitr-drills/<stamp>/evidence.env` and in the workflow summary, then discards the scratch instance. The drill exits non-zero on any failed check. It also fails when recovery stops before the target, for example because WAL is missing, or when promotion does not come within the RTO.

The staging drill runs from **Actions → Recover Synology staging schema → mode `pitr-drill`** on `main`, with an optional `pitr_target_time`. It uses no secret: it reads the running containers through Docker. Record each dated result in `PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md`. Run a drill after every change to this mechanism and after the P6 cutover.

## Real point-in-time restore

Use this for corruption or loss. Follow `INCIDENT_RECOVERY_RUNBOOK.md` (Platform database) for fencing, escalation and the native evidence order.

1. Fence writes: close the edge gate and stop `platform`, the scheduler and every Platform consumer. Record the last known healthy time T in UTC.
2. Run the drill against T: `restore-postgres-pitr.sh --target-time 'T'`. A passing drill proves that the backup and WAL reach T and gives the restore time. For a time before the oldest daily base backup, use the nearest weekly backup with `--target immediate`.
3. Stop `postgres` and `postgres-backup`. Never delete `postgres_data`. Restore into a **new** named volume instead:
   1. Create it, for example `postgres_data_restore_<stamp>`.
   2. As uid 999, check `SHA256SUMS`, unpack `base.tar.gz` into it and `pg_wal.tar.gz` into its `pg_wal`, then run `pg_verifybackup`.
   3. Write `recovery.signal`.
   4. Set `restore_command = 'bash /bootstrap/postgres-backup/restore-wal.sh "%f" "%p"'`, `recovery_target_time = 'T+00'` and `recovery_target_action = 'promote'`.
   5. Mount the spool read-only. The scratch-instance steps in `restore-postgres-pitr.sh` are the reference.
4. Start a one-off server on the new volume, wait for promotion, run the drill checks, then repoint the `postgres` service to the restored volume with an owner-reviewed Compose change. The old volume stays until the owner decides.
5. After promotion the cluster is on a new timeline. Its archive continues in the same spool, and the history file is kept.
6. Restore the native evidence witness only as `INCIDENT_RECOVERY_RUNBOOK.md` directs: never lowered, then `game-auth:native-evidence:reconcile`.
7. Run `deploy.sh` (or the health check), take a fresh base backup with `backup-postgres.sh run base`, and record the incident timeline, RTO and data loss.

For a logical restore of single tables, use the dump: `pg_restore -l` lists it and `pg_restore -t <table>` restores into a scratch database. Never restore over the live database without an owner decision.

## Another cluster

A re-initialised PostgreSQL server (a new `postgres_data`, for example after the P3 clean-slate bootstrap) is a new cluster with a new system identifier. Its WAL segment names collide with the archived ones, so `archive-wal.sh` refuses them, and the spool's base backups cannot restore it. `check` reports degraded and the deploy reconcile refuses.

Retire the old content without deleting it:

```bash
docker compose --profile postgres exec -u postgres postgres-backup sh -c '
  r=/var/lib/oteryn-postgres-backup/retired-$(date -u +%Y%m%dT%H%M%SZ) && mkdir "$r" &&
  mv /var/lib/oteryn-postgres-backup/wal /var/lib/oteryn-postgres-backup/base /var/lib/oteryn-postgres-backup/dump "$r"/ &&
  mkdir /var/lib/oteryn-postgres-backup/wal /var/lib/oteryn-postgres-backup/base /var/lib/oteryn-postgres-backup/dump'
```

Then re-run the deploy. Delete a retired directory only on an owner decision, once nothing needs it.

## Rollback of this mechanism

Set `archive_mode = off` in `deploy/synology/postgres/postgresql.conf` (this needs a `postgres` restart) and stop the `postgres-backup` sidecar. Backups are additive: live data is unaffected and the spool stays.

## Before production

- An encrypted off-NAS copy of `base/` and `wal/`, with a key held outside the NAS.
- A drill restored from that copy.
- Owner-set RPO and RTO.
- A decision on pgBackRest or WAL-G.
- The native evidence witness and admission-key backup. Decided custody: a follow-up adds a separate root-only job that writes an encrypted copy. It never uses this spool or the `postgres-backup` sidecar, whose roles must not read Platform storage or keys (see `INCIDENT_RECOVERY_RUNBOOK.md`).
