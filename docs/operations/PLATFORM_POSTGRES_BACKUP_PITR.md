# Platform PostgreSQL backups and point-in-time recovery on the NAS

Status: staging mechanism (plan P5, `PLATFORM_POSTGRES_MIGRATION_PLAN.md`; ADR 0046 §7). Provisional staging targets: RPO 24 hours, RTO 4 hours. The owner sets the production values. Backups are taken on the Synology NAS, and `offsite-postgres-backup.sh` copies them, encrypted, to a configured off-NAS destination (*Off-NAS copy*). **A configured off-NAS copy and a passing drill restored from it are required before any production use, real player, wallet or Bazaar value, or payment activation** (`PRODUCTION_READINESS_CHECKLIST.md` §5).

This runbook covers the PostgreSQL Platform database only. It holds no secret, password, dump or backup.

## Components

| Part | Where | What it does |
| --- | --- | --- |
| `archive_command` | `deploy/synology/postgres/backup/archive-wal.sh`, run by `postgres` | Writes each completed WAL segment to the spool as `wal/<segment>.gz` with a `.sha256` file. It refuses a bad name, a missing spool, conflicting content for an existing name, and a new file on a spool below 256 MiB free (`OTERYN_ARCHIVE_MIN_FREE_MB`). A retry of a segment already archived with identical content succeeds whatever the free space, because it writes nothing. On refusal PostgreSQL keeps the segment in `pg_wal` and retries. |
| `restore_command` | `deploy/synology/postgres/backup/restore-wal.sh` | Restores a segment for recovery. An absent file exits 1, the normal end of the archive. A missing archive directory, a missing checksum, a checksum mismatch or a decompression failure exits 199, which aborts recovery, so a damaged archive never ends recovery early and silently. |
| `postgres-backup` sidecar | `compose.yml`, profile `postgres`, same pinned image as `postgres`, private network only, no port | Runs `backup-job.sh service`: a daily base backup and logical dump at `OTERYN_BACKUP_HOUR_UTC` (default 02:00 UTC), retention after each success, and a heartbeat every 60 seconds for its healthcheck. A failed job is retried after one hour at the earliest. |
| `backup-postgres.sh` | `deploy/synology/scripts/` | `reconcile` (every deploy), `check` (every health check), `run <job>` (operator). |
| `restore-postgres-pitr.sh` | `deploy/synology/scripts/` | The restore drill into a scratch instance. With `--offline` it restores without the live server (see *Offline drill*), and with `--offline --spool-volume` from a spool restored off the NAS. |
| `offsite-postgres-backup.sh` | `deploy/synology/scripts/`, config `offsite-backup.env.example` | The encrypted off-NAS copy of the spool, its retention, check and restore drill (see *Off-NAS copy*). |

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
- **Preflight.** On every deploy and in `check`, the spool must have room for the next base backup and dump together: `max(OTERYN_BACKUP_MIN_FREE_MB, 2 x (database MiB + 64) + OTERYN_BACKUP_WAL_RESERVE_MB)` MiB free (defaults 2048 and 1024). A single base backup or dump job reserves only its own backup, `1 x (database MiB + 64)` plus the same WAL reserve and minimum, so the day's dump is not refused for the space its base backup just used. When the room is missing, the job and the deploy reconcile refuse, and `check` reports the backups as degraded.
- **Archive floor.** `archive-wal.sh` refuses below 256 MiB free. WAL then accumulates in `pg_wal` on `postgres_data` until space returns. Watch for this: a full `postgres_data` stops the database.
- **Drill scratch.** The drill needs free Docker storage of at least the restored cluster size plus `OTERYN_DRILL_WAL_MARGIN_MB` (default 1024 MiB). It checks this before it restores.

## Schedule and retention

- **Daily jobs.** One base backup (`pg_basebackup -Ft -z -X stream`, SHA-256 manifest) and one dump (`pg_dump -Fc`). Each runs when the newest one is at least 20 hours old at the configured hour, or at once when none exists or the newest is 25 hours old.
- **Deploy.** The deploy reconcile (`ensure`) also takes one when the newest is 24 hours old or more. It starts the jobs detached in the sidecar and waits for them at most `OTERYN_BACKUP_ENSURE_BUDGET_SECONDS` (default 300), well inside the 30-minute deploy job, because a job can wait up to two hours for the job lock and a backup has no time limit. Within the budget the reconcile ends with the strict check, and a failed job fails the deploy. Past the budget the jobs finish in the background, with their log in `status/ensure.log`. The deploy then goes on with the backups reported degraded (a health check warning) until they finish. A structural failure still fails it.
- **One job at a time.** Every job takes the spool job lock (`status/job.lock`) and decides again under it whether its backup is still needed, so the sidecar loop and a deploy's `ensure` never both take the same day's backup.
- **Kept backups.** For base backups and dumps alike: the newest 7, plus the newest backup of each of the 4 most recent older ISO weeks.
- **WAL cutoff.** WAL files older than the start WAL of the oldest of the newest 7 base backups are removed, which reaches back about seven days. Timeline history files are never removed.
- **Weekly backups.** A weekly base backup restores to its own end only (`--target immediate`), because it carries its WAL in `pg_wal.tar.gz`. Such a drill proves that the chosen backup restores. It does not measure today's RPO: its evidence reads `DATA_LOSS_BOUND_SECONDS=not_applicable`, and the probe drill measures the RPO.

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
bash deploy/synology/scripts/restore-postgres-pitr.sh --offline --target latest              # live server lost
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

It records the restore time against the 4 h RTO, and the data-loss bound against the 24 h RPO, in `<state dir>/pitr-drills/<stamp>/evidence.env` and in the workflow summary, then discards the scratch instance. The data-loss bound is what a restore to now would lose. It is `archive_timeout` with a healthy archiver, and the age of the restored base backup otherwise. A `--target immediate` drill restores one chosen base backup to its own end, so it records the bound as `not_applicable` and is not checked against the RPO. The drill exits non-zero on any failed check. It also fails when recovery stops before the target, for example because WAL is missing, or when promotion does not come within the RTO. The RTO covers the whole restore, from the base backup choice to promotion. A drill that leaves its scratch container or volume behind, or cannot write its `evidence.env` or workflow summary, is a failed drill: its result reads `failed`, with failure `cleanup: …` or `evidence: …`. An earlier failure stays the reported one, and the late one is added as `cleanup_failure` or `evidence_failure`.

The staging drill runs from **Actions → Recover Synology staging schema → mode `pitr-drill`** on `main`, with an optional `pitr_target_time`. It uses no secret: it reads the running containers through Docker. The job times out after 420 minutes. That covers up to 2 hours waiting for the job lock, the 4 h RTO, and an hour for the probe, the checks after promotion and the cleanup. A change to the lock wait or the RTO needs a matching job timeout. Record each dated result in `PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md`. Run a drill after every change to this mechanism and after the P6 cutover.

### Offline drill

The probe and `--target-time` drills need a running live server: they read its system identifier and archiver state and write marker rows. When `postgres_data` is lost or damaged, or PostgreSQL does not start, use `--offline`. It never executes anything in the live server or the backup sidecar and writes no marker:

- The image and the spool volume come from the Docker metadata of the `postgres` container in any state, or of the `postgres-backup` sidecar. When neither container exists, create one without starting it: `docker compose --profile postgres create postgres-backup`.
- The cluster comes from the spool: the one system identifier of its complete base backups. When the spool holds more than one cluster, the drill lists them with their newest base backup and refuses. Name the one to restore with `--system-identifier <id>`.
- `--target latest` (offline only) replays every archived segment, then promotes. `--target-time 'T'` needs a commit after T in the archive. The drill refuses at once when the newest archived segment is older than T. `--target immediate --backup <stamp>` works as above.
- The evidence records `ARCHIVE_END` (the newest archived segment) and `REPLAYED_TO` (the last replayed commit). It records `DATA_LOSS_BOUND_SECONDS=not_applicable`, because no live archiver is read and the time of the loss is unknown. The incident record gives the data loss: from `REPLAYED_TO` to the time of the loss.
- The Platform checks take the image from the `platform` container in any state, so a fenced, stopped Platform still serves. They run their own container against the scratch instance.

## Real point-in-time restore

Use this for corruption or loss of the database. The spool itself is on the same NAS. When the spool or the NAS is lost too, restore the spool from the off-NAS copy first (*Off-NAS copy*, *Disaster restore*), then continue here with `--offline --spool-volume`. Follow `INCIDENT_RECOVERY_RUNBOOK.md` (Platform database) for fencing, escalation and the native evidence order.

1. Fence writes: close the edge gate and stop `platform`, the scheduler and every Platform consumer. Record the last known healthy time T in UTC.
   - When `postgres_data` is lost or damaged, or PostgreSQL does not start, also stop `postgres` and `postgres-backup` at once: `docker compose --profile postgres stop postgres postgres-backup`. Do not deploy and do not start `postgres` until the restore. The image initialises an empty data volume as a new cluster. That cluster archives into the same spool, and the sidecar backs it up there.
2. Run the drill against the target. A passing drill proves that the backup and WAL reach it and gives the restore time.
   - With a running live server: `restore-postgres-pitr.sh --target-time 'T'`.
   - With the live server lost, damaged or not starting: `restore-postgres-pitr.sh --offline --target latest`, or `--offline --target-time 'T'` when the archive holds a commit after T (see *Offline drill*). Add `--system-identifier <id>` when the drill reports more than one cluster in the spool.
   - For a time before the oldest daily base backup, use the nearest weekly backup with `--target immediate --backup <stamp>` (with `--offline` when the live server is lost).
3. Stop `postgres` and `postgres-backup`. Never delete `postgres_data`. Restore into a **new** named volume instead:
   1. Create it, for example `postgres_data_restore_<stamp>`.
   2. As uid 999, check `SHA256SUMS`, unpack `base.tar.gz` of the base backup the drill chose into it and `pg_wal.tar.gz` into its `pg_wal`, then run `pg_verifybackup`.
   3. Write `recovery.signal`.
   4. Set `restore_command = 'bash /bootstrap/postgres-backup/restore-wal.sh "%f" "%p"'` and the target the drill used: `recovery_target_time = 'T+00'` with `recovery_target_action = 'promote'`, or no recovery target for `--target latest` (recovery promotes at the end of the archive).
   5. Mount the spool read-only. The scratch-instance steps in `restore-postgres-pitr.sh` are the reference.
4. Start a one-off server on the new volume, wait for promotion, run the drill checks, then repoint the `postgres` service to the restored volume with an owner-reviewed Compose change. The old volume stays until the owner decides.
5. After promotion the cluster is on a new timeline. Its archive continues in the same spool, and the history file is kept.
6. Restore the native evidence witness only as `INCIDENT_RECOVERY_RUNBOOK.md` directs: never lowered, then `game-auth:native-evidence:reconcile`.
7. If an empty cluster wrote into the spool after the loss, move only its `base/` and `dump/` directories aside before the deploy: `backup.info` names its other `system_identifier`. Move them to a `retired-<stamp>` directory of the spool. Never use *Another cluster* below here. It retires the restored cluster's archive too. Recovery never replays a segment of the empty cluster: PostgreSQL refuses WAL of another database system, which fails the restore instead.
8. Run `deploy.sh` (or the health check), take a fresh base backup with `backup-postgres.sh run base`, and record the incident timeline, RTO and data loss.

For a logical restore of single tables, use the dump: `pg_restore -l` lists it and `pg_restore -t <table>` restores into a scratch database. Never restore over the live database without an owner decision.

## Another cluster

A re-initialised PostgreSQL server (a new `postgres_data`, for example after the P3 clean-slate bootstrap) is a new cluster with a new system identifier. Its WAL segment names collide with the archived ones, so `archive-wal.sh` refuses them, and the spool's base backups cannot restore it. `check` reports degraded and the deploy reconcile refuses.

This procedure is for a deliberate re-initialisation only. After a loss, the restored cluster keeps the spool's identity, and only the empty cluster's backups are moved aside (*Real point-in-time restore*, step 7).

Retire the old content without deleting it:

```bash
docker compose --profile postgres exec -u postgres postgres-backup sh -c '
  r=/var/lib/oteryn-postgres-backup/retired-$(date -u +%Y%m%dT%H%M%SZ) && mkdir "$r" &&
  mv /var/lib/oteryn-postgres-backup/wal /var/lib/oteryn-postgres-backup/base /var/lib/oteryn-postgres-backup/dump "$r"/ &&
  mkdir /var/lib/oteryn-postgres-backup/wal /var/lib/oteryn-postgres-backup/base /var/lib/oteryn-postgres-backup/dump'
```

Then re-run the deploy. Delete a retired directory only on an owner decision, once nothing needs it.

## Off-NAS copy

`deploy/synology/scripts/offsite-postgres-backup.sh` copies the whole spool (`base/`, `wal/`, `dump/`) to a destination outside the NAS with [restic](https://restic.net) 0.18.0, pinned by digest. restic encrypts and authenticates every object on the NAS before it leaves (AES-256 + Poly1305), so the destination stores ciphertext only. This follows the Game database approach (Game ADR 0004, operability review 2026-10-06: RPO 5 min, RTO 4 h). Neither the provider nor any secret is in the repository: the destination is a restic repository URL in an owner-held configuration file.

### Configuration and secret custody

Copy `deploy/synology/offsite-backup.env.example` to a root-only file outside the checkout and outside the spool, for example `/volume1/docker/oteryn-offsite/offsite.env`, and name it with `OTERYN_OFFSITE_CONFIG`. It holds no secret. It names:

| Key | Meaning |
| --- | --- |
| `OTERYN_OFFSITE_REPOSITORY` | The restic repository URL: `s3:`, `b2:`, `azure:`, `gs:`, `sftp:`, `rest:`, `rclone:`, `swift:` or an absolute path. A URL with an embedded password is refused: credentials go in the credentials file. |
| `OTERYN_OFFSITE_PASSWORD_FILE` | The repository password (at least 32 random characters). It encrypts the copy. |
| `OTERYN_OFFSITE_CREDENTIALS_FILE` | Optional `KEY=VALUE` provider credentials passed to restic as environment, scoped to the destination bucket or path only. |
| `OTERYN_OFFSITE_STATE_DIR` | Local state: the last push result, the restic cache and the drill evidence. |
| `OTERYN_OFFSITE_TARGET_RPO_SECONDS` | Off-NAS data-loss target, default 300 (the Game target). The Platform value is owner decision D-2. |
| `OTERYN_OFFSITE_KEEP_WITHIN_HOURLY` / `_DAILY` / `_WEEKLY` | Retention, defaults `48h` / `14d` / `56d`. |
| `OTERYN_OFFSITE_CHECK_SUBSET` | Share of the stored data `check` reads back, default `5%`. |
| `OTERYN_OFFSITE_LOCK_RETRY` | How long a run waits for another run's repository lock, default `5m`. |
| `OTERYN_OFFSITE_SPOOL_VOLUME` | Optional spool volume name, when no `postgres-backup` or `postgres` container exists to find it by Compose labels. |

The script refuses (exit 2) a placeholder or malformed value, and a password or credentials file that is a symlink, not owned by the running user, or readable by group or others (mode 0600 or stricter). Custody:

- The password and the credentials never enter an argument, a container environment variable, a log or the evidence. The password file is bind-mounted read-only into the restic container, and the credentials file is passed with `--env-file`.
- **Keep a second copy of the password off the NAS** (owner password manager or sealed escrow). If the NAS is lost, the copy cannot be read without it. Losing every copy of the password loses the off-NAS backups.
- Use credentials that reach only the destination bucket or path. Where the provider supports it, deny delete to the push credentials and enable object lock or versioning, or use `rest-server --append-only` on a second site (see *Follow-ups*).

### Modes and schedule

Run as root on the NAS (DSM **Control Panel → Task Scheduler → User-defined script**, user `root`) from the deploy directory, with `COMPOSE_PROJECT_NAME` set when the project is not `oteryn-staging`:

```bash
OTERYN_OFFSITE_CONFIG=/volume1/docker/oteryn-offsite/offsite.env bash deploy/synology/scripts/offsite-postgres-backup.sh init     # once
OTERYN_OFFSITE_CONFIG=/volume1/docker/oteryn-offsite/offsite.env bash deploy/synology/scripts/offsite-postgres-backup.sh push     # every minute
OTERYN_OFFSITE_CONFIG=/volume1/docker/oteryn-offsite/offsite.env bash deploy/synology/scripts/offsite-postgres-backup.sh forget   # daily
OTERYN_OFFSITE_CONFIG=/volume1/docker/oteryn-offsite/offsite.env bash deploy/synology/scripts/offsite-postgres-backup.sh check    # weekly
OTERYN_OFFSITE_CONFIG=/volume1/docker/oteryn-offsite/offsite.env bash deploy/synology/scripts/offsite-postgres-backup.sh drill    # monthly
OTERYN_OFFSITE_CONFIG=/volume1/docker/oteryn-offsite/offsite.env bash deploy/synology/scripts/offsite-postgres-backup.sh status   # any time, no network
```

Exit status: 0 ok, 1 failure, 2 usage or configuration, 3 degraded (the newest copy is older than the target RPO).

- **`init`** creates the encrypted repository when none exists. It is idempotent, and refuses on a wrong password or an unreachable destination instead of creating a second repository.
- **`push`** snapshots the spool, mounted read-only, with host `$COMPOSE_PROJECT_NAME` and tag `postgres-spool`. It skips the in-flight `.*.partial` directories and `.*.tmp.*` files. restic uploads only changed data, so a push after a new WAL segment sends about that segment. It takes no spool job lock, so a long base backup never holds the copy back. A snapshot that saw a file disappear (retention ran during it, restic exit 3) is retried once. A host lock skips a push while the previous one still runs. The result is written to `<state dir>/status/push.last`.
- **Off-NAS lag.** With `archive_timeout = 60s` and a push every minute, the off-NAS copy lags the database by about two minutes plus the push time, inside the 5-minute Game target. A daily base backup push takes longer: the next pushes are skipped meanwhile, and `status` reports degraded if it exceeds the target.
- **`forget`** applies retention to this host's snapshots: the newest snapshot always, then every hourly snapshot of the last 48 hours, every daily of 14 days and every weekly of 8 weeks, and prunes the data no kept snapshot references. Each snapshot holds the whole spool, so every kept snapshot also keeps the about 7 days of WAL before it for point-in-time recovery.
- **`check`** reads the newest snapshot time at the destination and exits 3 when it is older than the target RPO, then runs `restic check --read-data-subset` (5% by default), which downloads and authenticates that share of the stored data. Exit 1 means a damaged repository: run `check` with `OTERYN_OFFSITE_CHECK_SUBSET=100%` and treat the copy as lost until it passes.
- **`status`** reads the local `push.last` only. Wire it into monitoring with the backup `check` (follow-up).

### Restore drill from the off-NAS copy

`drill [--snapshot ID] [--skip-platform-checks]` proves that the copy restores, without touching the live spool:

1. Creates a holder container (`--rm`, `--network none`, `--cap-drop ALL`) with an anonymous scratch volume.
2. Restores the newest snapshot (or `--snapshot ID`) into it with `restic restore --verify`, and recreates the empty `status/job.lock` owned by uid 999.
3. Runs `restore-postgres-pitr.sh --offline --spool-volume <scratch volume> --target latest`, which runs every check of the offline drill (checksums, `pg_verifybackup`, promotion, wallet invariant, migrations, dump, Platform checks) on a scratch instance.
4. Stops the holder, which discards the scratch volume, and fails the drill if the volume remains.

Evidence is written to `<state dir>/drills/<stamp>/offsite.env` (`SNAPSHOT`, `SNAPSHOT_TIME`, `DOWNLOAD_SECONDS`, `PITR_EXIT`, `TOTAL_SECONDS` against `RTO_SECONDS=14400`, `RESULT`) and the PITR evidence to `<state dir>/drills/<stamp>/pitr/`. The repository URL is not recorded: only its scheme. The total time, download plus PITR, must stay inside the 4 h RTO. Record each dated result in `PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md`.

### Disaster restore

When the spool or the whole NAS is lost, on the replacement host:

1. Install Docker and the deploy checkout, put back the configuration, the password (from the off-NAS escrow) and the credentials files.
2. Create, without starting, a container whose labels name the project: `docker compose --profile postgres create postgres-backup`. Never start `postgres` before the restore (*Real point-in-time restore*, step 1).
3. Restore the spool into a new named volume: `offsite-postgres-backup.sh restore postgres_backup_restore_<stamp> [--snapshot ID]`. It refuses an existing volume.
4. Drill and restore from it: `restore-postgres-pitr.sh --offline --spool-volume postgres_backup_restore_<stamp> --target latest` (or `--target-time 'T'`), then follow *Real point-in-time restore* from step 3, mounting that volume as the spool. Repoint the `postgres_backup` volume of the Compose project to it with an owner-reviewed change.

### Owner decisions

- **D-2, Platform RPO.** The off-NAS target defaults to 300 seconds, the Game target; the local spool target is still 24 hours. Set `OTERYN_OFFSITE_TARGET_RPO_SECONDS` to the owner's value.
- **D-3, destination.** Any restic repository: AWS S3, an S3-compatible store (Backblaze B2, Wasabi, Hetzner Object Storage, Cloudflare R2), Azure Blob, or a second-site NAS running `rest-server --append-only` or SFTP. Until a destination is configured, there is no off-NAS copy and production remains blocked.

### Follow-ups

- Append-only or object-locked destination, with a separate credential that alone may prune.
- `status` in `health-check.sh` and alerting on exit 1 or 3.
- The native evidence witness and admission-key encrypted copy (separate root-only job, *Before production*).
- A decision on pgBackRest or WAL-G replacing the spool plus restic.

## Rollback of this mechanism

Set `archive_mode = off` in `deploy/synology/postgres/postgresql.conf` (this needs a `postgres` restart) and stop the `postgres-backup` sidecar. Backups are additive: live data is unaffected and the spool stays.

## Before production

- The off-NAS copy configured at an owner-chosen destination (D-3), scheduled, with the password escrowed outside the NAS (*Off-NAS copy*).
- A passing `offsite-postgres-backup.sh drill` recorded in `PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md`.
- Owner-set RPO and RTO (D-2).
- A decision on pgBackRest or WAL-G.
- The native evidence witness and admission-key backup. Decided custody: a follow-up adds a separate root-only job that writes an encrypted copy. It never uses this spool or the `postgres-backup` sidecar, whose roles must not read Platform storage or keys (see `INCIDENT_RECOVERY_RUNBOOK.md`).
