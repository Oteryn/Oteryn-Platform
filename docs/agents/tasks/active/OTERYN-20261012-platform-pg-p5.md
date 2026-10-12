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
  - PR #1579 (P4, branch claude/platform-pg-p4-20261012) lands first
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T18:30:00Z
head: UNKNOWN
branch: claude/platform-pg-p5-20261012
pr: none
status: validating
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
  - tests/ci/test_synology_postgres_backup_contract.py
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/operations/**
validation:
  - command: python3 -m pytest tests/ci deploy/synology/tests -q
    result: PASS
    evidence: 421 passed, 91 subtests passed
  - command: shellcheck and bash -n on the new scripts; sh -n tls/init.sh; docker compose config with and without the postgres profile
    result: PASS
    evidence: clean; only SC1091 info for sourcing lib.sh
  - command: local drill harness (docs/operations/PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md)
    result: PASS
    evidence: probe, target-time and immediate drills passed; scratch discarded
blockers:
  - none
next_action: Open the PR to main stacked on #1579, then wait for CI and review.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

- Jira sync: pending (no mapped Story verified in this session).
- Not in this packet's paths, so these are follow-ups:
  - `scripts/ci/classify_synology_builds.py` does not list `backup-postgres.sh` as a deployment runtime input;
  - the build workflow's `bash -n` list omits the new scripts;
  - witness and admission-key backup custody needs an owner decision (see the incident runbook).
