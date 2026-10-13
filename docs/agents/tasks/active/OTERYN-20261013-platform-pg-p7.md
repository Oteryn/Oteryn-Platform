---
task_id: OTERYN-20261013-platform-pg-p7
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
search_first:
  - deploy/synology/scripts/cutover-postgres.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/compose.yml
optional_reads:
  - deploy/synology/README.md
---

# OTERYN-20261013-platform-pg-p7

## Goal

Governing GitHub Issue: #1474 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Plan P7 (ADR 0046): remove the MariaDB Platform paths from the Synology staging stack once the P6 cutover has retired MariaDB. Every MariaDB path in these files is still used by the cutover rollback (`cutover-postgres.sh rollback`, `health-check.sh`, `rollback.sh`) until `retire-mariadb`, so nothing here is safe earlier: this is the P7b unit, and it **must not merge before `cutover-postgres.sh retire-mariadb` records `STEP_MARIADB_RETIRED` on staging** (and after PR #1598, the engine flip, which it is stacked on).

## Acceptance criteria

- [x] `compose.yml` has no `mariadb` service, `depends_on: mariadb` or `mariadb_data` volume; the Platform connection defaults are the PostgreSQL app role over verify-full TLS; the scheduler healthcheck is PostgreSQL-only.
- [x] `lib.sh` resolves only `PLATFORM_DB_ENGINE=postgres`; the PostgreSQL secrets are always required; pre-migration backups are always `pg_dump` with `BACKUP_DB_ENGINE=postgres`; the image engine check still refuses MariaDB-era images (the contract key stays).
- [x] `deploy.sh`, `deploy-impact.sh`, `health-check.sh`, `recover-schema.sh`, `prepare-fresh-schema-baseline.sh`, `marketplace-staging.sh` and `production-target-preflight.sh` no longer require `MARIADB_ROOT_PASSWORD`/`PLATFORM_DB_PASSWORD`/`PLATFORM_DB_USER`, start, check or dump MariaDB; the fast and reconcile paths preserve the postgres container; recovery refuses a MariaDB-era backup before any change.
- [x] README describes the PostgreSQL-only stack and the secret retirement order.
- [x] Synology contract tests follow.
- [ ] Merged only after `retire-mariadb` is recorded on staging (owner sequencing).

Kept on purpose (outside this unit or still needed): `cutover-postgres.sh`, `bootstrap-postgres.sh` (the deploy workflow's cutover action); `restore-postgres-pitr.sh`; `pdo_mysql` in the image; `deploy/synology/mariadb/init` and `BUILD_ROUTING.md` (deleting it needs `build-synology-staging-images.yml` and its classifier, a CP-owned workflow change).

Follow-ups for CP/owner (P7c, not in this PR): `deploy-synology-staging.yml` stops rendering `MARIADB_ROOT_PASSWORD`/`PLATFORM_DB_PASSWORD`; `repair-synology-autostart.yml` drops `mariadb` from its loop (it fails closed once retire-mariadb removes the container); CI/acceptance MariaDB service containers; `config/database.php` mysql connection and phpunit; the ADR 0044 mysql row (with Amendment P4); live operations docs, `synology-diagnostics.yml`, the recovery workflow; then the operator removes the two retired MariaDB secrets.

## Ownership

```yaml
owned_paths:
  - deploy/synology/compose.yml
  - deploy/synology/release-contract.env
  - deploy/synology/README.md
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/scripts/recover-schema.sh
  - deploy/synology/scripts/prepare-fresh-schema-baseline.sh
  - deploy/synology/scripts/marketplace-staging.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - deploy/synology/tests/test_platform_mtls_contract.py
  - tests/ci/test_synology_autostart_contract.py
  - tests/ci/test_synology_compose_runtime_contract.py
  - tests/ci/test_synology_marketplace_legacy_state.py
  - tests/ci/test_synology_native_admission_gate.py
  - tests/ci/test_synology_native_admission_trust_publication.py
  - tests/ci/test_synology_postgres_contract.py
  - tests/ci/test_synology_postgres_backup_contract.py
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - docs/agents/tasks/active/OTERYN-20261013-platform-pg-p7.md
  - docs/agents/tasks/archive/OTERYN-20261013-platform-pg-p7.md
modules:
  - deploy/synology
dependencies:
  - PR #1598 (engine flip, cutover step 2) merged
  - cutover-postgres.sh retire-mariadb recorded on staging (STEP_MARIADB_RETIRED)
blockers:
  - retire-mariadb not yet recorded on staging (merge only; review can proceed)
cross_repository_tasks:
  - none
```

## Context checkpoint

`checkpoint_version` remains structural version 1. Validate with `python tools/agents/checkpoint.py <task-path> --require-checkpoint`.

```yaml
checkpoint_version: 1
updated_at: 2026-10-13T16:00:00Z
head: 82e15f4a1650610ce0a1e86c0574bf5dc70a8d0b
branch: claude/platform-p7-20261013
pr: pending
status: validating
context_routes:
  - deploy
owned_paths:
  - deploy/synology/compose.yml
  - deploy/synology/release-contract.env
  - deploy/synology/README.md
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/scripts/recover-schema.sh
  - deploy/synology/scripts/prepare-fresh-schema-baseline.sh
  - deploy/synology/scripts/marketplace-staging.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - deploy/synology/tests/test_platform_mtls_contract.py
  - tests/ci/test_synology_autostart_contract.py
  - tests/ci/test_synology_compose_runtime_contract.py
  - tests/ci/test_synology_marketplace_legacy_state.py
  - tests/ci/test_synology_native_admission_gate.py
  - tests/ci/test_synology_native_admission_trust_publication.py
  - tests/ci/test_synology_postgres_contract.py
  - tests/ci/test_synology_postgres_backup_contract.py
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - docs/agents/tasks/active/OTERYN-20261013-platform-pg-p7.md
proven:
  - cutover-postgres.sh rollback needs the mariadb service, its secrets and the mariadb engine until retire-mariadb; health-check.sh checks mariadb on the mariadb engine; so no MariaDB path is removable before retire-mariadb.
  - The engine-flip gate (PR 1598) refuses a deploy after retire-mariadb while compose.yml still defines mariadb; this PR is the compose change that clears it.
  - retire-mariadb (step 6b) removes the mariadb container and the mariadb_data volume and warns that an ordinary deploy must wait for P7, which would otherwise recreate the retired service.
derived:
  - Merged before retire-mariadb, the next deploy would stop managing the running mariadb container and drop the cutover rollback path.
unknown:
  - none
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Splitting a P7a that lands before retire-mariadb; rejected because every MariaDB path in these files is on the cutover rollback path.
changed_paths:
  - deploy/synology/compose.yml
  - deploy/synology/release-contract.env
  - deploy/synology/README.md
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/scripts/recover-schema.sh
  - deploy/synology/scripts/prepare-fresh-schema-baseline.sh
  - deploy/synology/scripts/marketplace-staging.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - deploy/synology/tests/test_platform_mtls_contract.py
  - tests/ci/test_synology_autostart_contract.py
  - tests/ci/test_synology_compose_runtime_contract.py
  - tests/ci/test_synology_marketplace_legacy_state.py
  - tests/ci/test_synology_native_admission_gate.py
  - tests/ci/test_synology_native_admission_trust_publication.py
  - tests/ci/test_synology_postgres_contract.py
  - tests/ci/test_synology_postgres_backup_contract.py
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - docs/agents/tasks/active/OTERYN-20261013-platform-pg-p7.md
validation:
  - command: python3 tests/ci/test_synology_postgres_contract.py
    result: PASS
    evidence: 67 tests OK
  - command: pytest tests/ci/test_synology_*.py
    result: PASS
    evidence: see PR body for counts
  - command: pytest deploy/synology/tests
    result: PASS
    evidence: 37 passed
  - command: bash -n and shellcheck -x on the changed scripts
    result: PASS
    evidence: no new shellcheck findings against the base
blockers:
  - retire-mariadb not yet recorded on staging (merge only)
next_action: CP posts the Codex review; one fix commit; merge only after PR 1598 and retire-mariadb.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository squash merge of the P7 PR
source_branch_evidence: pending
```

## Notes

- Stacked on PR #1598; the PR base is main, so its diff includes #1598 until that merges.
- `retire-mariadb` removes the mariadb container and `mariadb_data`; until this PR merges, the engine-flip gate refuses every ordinary deploy (compose still defines mariadb).
