---
task_id: OTERYN-20261015-platform-backup-offnas
governing_issue: 1626
required_reads:
  - docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md
search_first:
  - deploy/synology/scripts/restore-postgres-pitr.sh
  - deploy/synology/postgres/backup/backup-job.sh
optional_reads: []
---

# OTERYN-20261015 Encrypted off-NAS copy of the Platform PostgreSQL backups (PLATFORM-BACKUP-OFFNAS-1)

## Goal

Governing GitHub Issue: #1626 — canonical lifecycle authority for this task.

Close the P5 gap "backups stay on the NAS" (ADR 0046 §7, review 2026-10-12 F-02): an encrypted off-NAS copy of the backup spool with a configurable destination, a retention policy, a scripted restore drill from that copy with verification, and the runbook. It follows the Game database approach (Game ADR 0004, operability review 2026-10-06: RPO 5 min, RTO 4 h). The destination (D-3) and the Platform RPO (D-2) are owner decisions, implemented as configuration.

## Acceptance criteria

- [x] `offsite-postgres-backup.sh` copies the spool with restic (pinned by digest, client-side AES-256 + Poly1305) to any restic repository URL from an owner-held configuration file; no provider, URL or secret in the repository.
- [x] Secret files are refused unless owner-only (0600 or stricter, not a symlink); the password never enters an argument, environment variable, log or evidence; a URL with an embedded password is refused.
- [x] Retention (`forget`): newest snapshot, hourly 48h, daily 14d, weekly 56d, then prune; configurable.
- [x] `check` and `status` exit 3 when the newest copy is older than the configured target RPO (default 300 s); `check` reads back a random data subset.
- [x] `drill` restores a snapshot into a discarded scratch volume and runs `restore-postgres-pitr.sh --offline --spool-volume … --target latest`, with evidence against the 4 h RTO; nothing removes a Docker volume.
- [x] `restore VOLUME` and `restore-postgres-pitr.sh --spool-volume` support a disaster restore on a new host.
- [x] Runbook *Off-NAS copy* section; contract tests wired into ci.yml through the release identity hook.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/offsite-postgres-backup.sh
  - deploy/synology/offsite-backup.env.example
  - deploy/synology/scripts/restore-postgres-pitr.sh
  - scripts/ci/classify_synology_builds.py
  - .github/workflows/build-synology-staging-images.yml
  - tests/ci/test_synology_postgres_offsite_backup_contract.py
  - tests/ci/test_synology_postgres_backup_contract.py
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md
  - docs/operations/PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/agents/tasks/active/OTERYN-20261015-platform-backup-offnas.md
  - docs/agents/tasks/archive/OTERYN-20261015-platform-backup-offnas.md
modules:
  - deploy
dependencies:
  - OTERYN-20261012-platform-pg-p5
blockers:
  - none
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-15T12:00:00Z
head: 12f9050f935fcc944df20b900c57980c12ebd15a
branch: feat/platform-backup-offnas
pr: 1628
status: validating
context_routes:
  - deploy
owned_paths:
  - deploy/synology/scripts/offsite-postgres-backup.sh
  - deploy/synology/offsite-backup.env.example
  - deploy/synology/scripts/restore-postgres-pitr.sh
  - scripts/ci/classify_synology_builds.py
  - .github/workflows/build-synology-staging-images.yml
  - tests/ci/test_synology_postgres_offsite_backup_contract.py
  - tests/ci/test_synology_postgres_backup_contract.py
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md
  - docs/operations/PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/agents/tasks/active/OTERYN-20261015-platform-backup-offnas.md
  - docs/agents/tasks/archive/OTERYN-20261015-platform-backup-offnas.md
proven:
  - An end-to-end run with real Docker, postgres 17 archiving into a spool volume and a local restic repository passed init, push, status, check, forget, restore and drill (offline PITR passed, scratch volume discarded).
  - Stale copy exits 3; unsafe secret modes, URL passwords and malformed values exit 2; a wrong password or corrupted repository exits 1; a failing drill still discards its holder and volume.
derived:
  - Pushing without the spool job lock is safe because the spool only gains files by atomic rename and only retention removes them; restic exit 3 (file vanished) is retried once.
  - Each snapshot holds the whole spool, so every kept snapshot keeps about 7 days of point-in-time recovery.
unknown:
  - Owner choice of destination (D-3) and Platform RPO (D-2); asked through CP.
  - Push duration against a real cloud destination for the daily base backup.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Removing the drill scratch volume with docker volume rm; the repository volume guard forbids it, so the scratch volume is the anonymous volume of a --rm holder container.
  - Passing the restic password by environment variable; it would show in docker inspect, so it is a read-only bind-mounted file.
changed_paths:
  - deploy/synology/scripts/offsite-postgres-backup.sh
  - deploy/synology/offsite-backup.env.example
  - deploy/synology/scripts/restore-postgres-pitr.sh
  - scripts/ci/classify_synology_builds.py
  - .github/workflows/build-synology-staging-images.yml
  - tests/ci/test_synology_postgres_offsite_backup_contract.py
  - tests/ci/test_synology_postgres_backup_contract.py
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md
  - docs/operations/PLATFORM_POSTGRES_RESTORE_DRILL_EVIDENCE.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/agents/tasks/active/OTERYN-20261015-platform-backup-offnas.md
validation:
  - command: python tests/ci/test_synology_deploy_release_identity.py (runs the postgres, backup and off-NAS contract modules)
    result: PASS
    evidence: all hooks OK, volume guard included
  - command: shellcheck deploy/synology/scripts/offsite-postgres-backup.sh deploy/synology/scripts/restore-postgres-pitr.sh
    result: PASS
    evidence: no findings
  - command: python tests/ci/test_workflow_trigger_economy.py; test_synology_deployment_package_routing.py; test_classify_changes.py
    result: PASS
    evidence: routing and workflow contracts unchanged
blockers:
  - none
next_action: Drive the PR to green CI and a clean Codex review, then report to CP with the D-2/D-3 decision question.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
