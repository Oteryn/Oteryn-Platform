---
task_id: OTERYN-20261012-platform-pg-p5
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
search_first:
  - deploy/synology/compose.yml
  - deploy/synology/postgres/postgresql.conf
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
optional_reads: []
---

# OTERYN-20261012 Platform PostgreSQL P5: backups and PITR on the NAS

## Goal

Governing GitHub Issue: #1474. Packet P5 of `docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md` (ADR 0046 §7, finding F-01) covers continuous WAL archiving, a daily base backup by the REPLICATION-only backup role, a daily logical dump by the read-only dump role, 7 daily and 4 weekly retention, a NAS spool space preflight that refuses, and a restore drill into a scratch instance that verifies row counts and migrations. It also adds a runbook. The packet is stacked on P4 (PR #1579). It adds no new secret and runs no NAS deploy.

## Acceptance criteria

- [x] `archive_mode=on`, `archive_timeout=60s`. `archive_command` writes gzip plus SHA-256 into the `postgres_backup` spool outside `postgres_data`, refuses conflicting content and low space, and success requires the archived file.
- [x] `postgres-backup` sidecar: same pinned image and profile, private network only, no port, no secret in its environment, heartbeat healthcheck. Passwords reach a 0600 libpq file over stdin.
- [x] Daily `pg_basebackup` (SHA-256 manifest, `SHA256SUMS`, cluster system identifier) and `pg_dump -Fc`. Retention is 7 daily and 4 weekly, and WAL older than the oldest daily backup is pruned.
- [x] Space preflight `max(2048, 2 x (db + 64) + 1024)` MiB. Jobs and the deploy reconcile refuse below it.
- [x] Deploy reconcile proves a forced `pg_switch_wal()` segment reaches the spool, brings backups to at most 24 h old and checks strictly. The health check fails on structural faults and warns on degraded backups (exit 3).
- [x] `restore-postgres-pitr.sh` covers three modes: a probe drill, a past `--target-time`, and `--target immediate --backup`. It uses a scratch container with no network, an anonymous volume and `--rm`, and checks `SHA256SUMS`, `pg_verifybackup`, markers, row counts, the migrations head, the ledger invariant, the dump, and the Platform `migrate:status` and `production:verify-configuration`. It records RTO and RPO evidence.
- [x] `pitr-drill` mode in the recover workflow, with no secret.
- [x] Runbook, incident runbook section with the witness restore order, readiness rows including the off-NAS blocker, and dated local rehearsal evidence.
- [ ] Staging drill passes after merge, and the archiver runs 24 h without failures (owner and operator).
- [ ] Exact-head CI passes and the PR merges.

## Ownership

```yaml
owned_paths:
  - deploy/synology/postgres/backup/**
  - deploy/synology/scripts/backup-postgres.sh
  - deploy/synology/scripts/restore-postgres-pitr.sh
  - deploy/synology/compose.yml (postgres spool mount, postgres-backup service, postgres_backup volume)
  - deploy/synology/postgres/postgresql.conf (archive settings)
  - deploy/synology/scripts/deploy.sh (stage backup scripts, backup reconcile line)
  - deploy/synology/scripts/health-check.sh (backup check block)
  - deploy/synology/tls/init.sh (postgres-backup bootstrap permissions)
  - .github/workflows/recover-synology-staging-schema.yml (pitr-drill mode)
  - .github/workflows/build-synology-staging-images.yml (path filters and bash -n list for the new scripts)
  - .github/workflows/repair-synology-autostart.yml (postgres-backup in the restart repair)
  - scripts/ci/classify_synology_builds.py (route the new scripts)
  - tests/ci/test_synology_postgres_backup_contract.py
  - tests/ci/test_synology_deploy_release_identity.py (hook class only)
  - docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md
  - docs/operations/PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md
  - docs/operations/INCIDENT_RECOVERY_RUNBOOK.md (Platform PostgreSQL section)
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md (§5 rows)
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p5.md
modules:
  - deploy
dependencies:
  - PR #1579 (P4) merged 2026-10-12 and was merged into this branch from origin/main
  - PR #1581 (P4b) merged 2026-10-12 and was merged into this branch from origin/main; the drill job stays secret-free beside the schema-recovery job's PostgreSQL secrets
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-13T04:30:00Z
head: a17978379e032eeeb19472c9955df94b7790a95f
branch: claude/platform-pg-p5-20261012
pr: 1583
status: validating
terminal_pr_policy: archive_pending
context_routes:
  - deploy
owned_paths:
  - deploy/synology/postgres/backup/**
  - deploy/synology/scripts/backup-postgres.sh
  - deploy/synology/scripts/restore-postgres-pitr.sh
  - deploy/synology/compose.yml
  - deploy/synology/postgres/postgresql.conf
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/tls/init.sh
  - .github/workflows/recover-synology-staging-schema.yml
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/repair-synology-autostart.yml
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_postgres_backup_contract.py
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md
  - docs/operations/PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md
  - docs/operations/INCIDENT_RECOVERY_RUNBOOK.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p5.md
proven:
  - Local rehearsal on the pinned image (2026-10-12) passed the reconcile, check, and the probe, target-time and immediate drills. A low-space preflight refused, and a degraded check exited 3.
  - Retention, WAL pruning, archive refusal paths and restore exit codes (1 for absent, 199 for damage) are covered by tests/ci/test_synology_postgres_backup_contract.py.
  - Backup jobs decide again under the spool job lock, the drill's scratch instance holds a shared job lock from base backup choice to its end, and an identical archive retry syncs before success (JobLockTest, DrillLockTest, ArchiveWalTest).
  - A job's preflight reserves only its own backup, while the deploy and check keep the base-plus-dump reserve; a drill that leaves its scratch instance behind records result=failed (JobPreflightTest, DrillEvidenceTest).
  - A drill whose evidence.env or job summary cannot be written fails with result=failed; an intact archive retry succeeds below the free-space floor; the drill job timeout covers the lock wait, the RTO and cleanup, and pg_ctl waits only for the RTO left (DrillEvidenceTest, ArchiveWalTest, DeployWiringTest).
  - The deploy waits for its base backup and dump at most a 300 s budget inside the 30-minute deploy job, then continues with degraded backups while they finish in the sidecar; a --target immediate drill is not held to today's RPO (EnsureBudgetTest, DrillRpoTest).
  - A lost, damaged or non-starting live server is restored with --offline: the image and spool come from Docker metadata, the cluster from the spool (or --system-identifier), nothing runs in the live server, --target latest replays the whole archive, a fenced Platform still names its image, and the runbook covers stopping an auto-initialised empty cluster (DrillOfflineTest).
derived:
  - Recovery to a time stops only at a commit past the target, so the time-target drill writes and archives a marker commit first.
unknown:
  - NAS free space for the spool and the staging drill result (first deploy with the P4 secrets set).
  - Platform image checks in the drill (no Platform image locally).
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Archiving inside postgres_data, which a database volume loss would take with it.
  - Removing the drill's volume by name; the scratch container uses --rm with an anonymous volume instead (VolumeGuard).
changed_paths:
  - deploy/synology/**
  - .github/workflows/recover-synology-staging-schema.yml
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/repair-synology-autostart.yml
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_postgres_backup_contract.py
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/operations/**
validation:
  - command: python3 -m pytest tests/ci deploy/synology/tests -q
    result: PASS
    evidence: 516 passed, 19 skipped, 117 subtests passed (after the Codex fix for the offline restore of a lost live server)
  - command: shellcheck and bash -n on the new scripts; sh -n tls/init.sh; docker compose config with and without the postgres profile
    result: PASS
    evidence: clean; only SC1091 info for sourcing lib.sh
  - command: local drill harness (docs/operations/PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md)
    result: PASS
    evidence: probe, target-time and immediate drills passed; scratch discarded; probe drill re-run passed after the self-review hardening
blockers:
  - none
next_action: archive this record when the P6 drill and cutover packet records the staging pitr-drill pass and the 24 h archiver run without failures; those two criteria stay open here and move to that packet
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: PR #1583 merged as ba971b872ba243454b3a045d0b8ccb86bc8e68c7 from claude/platform-pg-p5-20261012; git ls-remote --heads origin claude/platform-pg-p5-20261012 returns nothing (verified)
```

## Notes

- Jira sync: pending (no mapped Story verified in this session).
- PR #1583, stacked on #1579.
- Build routing: `backup-postgres.sh` is a deployment runtime input (deploy.sh runs it) and `restore-postgres-pitr.sh` an operator input. Both are in the build workflow's path filters, and the new scripts are in its `bash -n` list.
- Owner decisions (2026-10-12):
  - degraded backups stay a health-check warning (the deploy reconcile is strict);
  - witness and admission-key backup stays out of this packet.
- Follow-up: back up the native evidence witness and the admission key volume with a separate root-only job writing an encrypted copy, never through the PostgreSQL spool or sidecar. This is recorded in the backup runbook, the incident runbook and readiness §5.
