---
task_id: OTERYN-20261012-platform-pg-p4b
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
search_first:
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/postgres/init/10-platform-roles.sh
optional_reads: []
---

# OTERYN-20261012 Platform PostgreSQL P4b: postgres-engine deploy, recovery, baseline, Marketplace and preflight

## Goal

Governing GitHub Issue: #1474. Follow-up to packet P4 (PR #1579) of `docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md`: remove deploy's refusal of `PLATFORM_DB_ENGINE=postgres` and add the PostgreSQL branches P4 deferred, so a deploy on that engine migrates as `oteryn_platform_owner` in a one-off container, the runtime connects as `oteryn_platform_app`, and schema recovery, the fresh-schema baseline, Marketplace staging control, the health check and the production-target preflight handle PostgreSQL. The committed engine stays `mariadb`, whose behaviour is unchanged. No NAS deploy and no engine flip (P6).

## Acceptance criteria

- [x] `deploy.sh` accepts `postgres` only with all five secrets and `PLATFORM_DB_NAME=oteryn_platform`; route publication stays refused on `postgres` pending the ADR 0044 amendment.
- [x] Migrations on `postgres` run as the owner in `compose run --rm --no-deps` with FastCGI off and `--database=pgsql_migrations`; the owner password passes by name only and never reaches long-running services; the deploy wrapper brackets it with the pre-migration dump and schema-state hooks.
- [x] Pre-migration backup and fresh baseline: `pg_dump -Fc` as the dump role over `verify-full`, password on stdin, mode 600, kept only when `pg_restore --list` reads it; evidence records `BACKUP_DB_ENGINE=postgres`.
- [x] Recovery restores only into the engine that wrote the backup; on `postgres` it drops the database `WITH (FORCE)`, recreates it with the role script's new `--database-only` mode and restores as the owner in one transaction.
- [x] Marketplace staging control, health check and production-target preflight have PostgreSQL branches (TLS 1.3, SCRAM, `pg_hba`, role split, no non-TLS TCP client, PDO probe as app, scratch-database restore drill).
- [x] Contract tests in `tests/ci/test_synology_postgres_contract.py`.
- [x] Exact-head CI passes and the PR merges (after #1579): PR #1581 final head 8da9016 had every hosted check success or conditionally skipped (see validation), and the PR merged as squash commit 9d2c4cf.
- [ ] Owner handoff, outside this packet's completion (tracked for P6): set the five `synology-staging` PostgreSQL secrets; accept ADR 0044 Amendment P4 before route publication on `postgres`.

## Ownership

```yaml
owned_paths:
  - deploy/synology/postgres/init/10-platform-roles.sh
  - deploy/synology/scripts/{deploy,lib,rollback,health-check,recover-schema,prepare-fresh-schema-baseline,production-target-preflight,marketplace-staging}.sh
  - deploy/synology/README.md
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - tests/ci/test_synology_postgres_contract.py
  - docs/agents/tasks/archive/OTERYN-20261012-platform-pg-p4b.md
modules:
  - deploy
dependencies:
  - PR #1579 (P4, branch claude/platform-pg-p4-20261012)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T20:30:00Z
head: 8da9016120b1f726ed50a469eaa7c5550ccf7ad0
branch: claude/platform-pg-p4b-20261012
pr: 1581
status: completed
context_routes:
  - deploy
  - security
owned_paths:
  - deploy/synology/postgres/init/10-platform-roles.sh
  - deploy/synology/scripts/**
  - deploy/synology/README.md
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - tests/ci/test_synology_postgres_contract.py
  - docs/agents/tasks/archive/OTERYN-20261012-platform-pg-p4b.md
proven:
  - Against PostgreSQL 17.11 locally, the dump helper writes a mode-600 archive that pg_restore lists; the restore recreates the database owned by the owner with app DML, dump SELECT only, no app CREATE on public and no backup CONNECT, and the app can write afterwards.
  - The role script's --database-only mode refuses when a Platform role is missing and creates nothing; an unknown argument exits 2.
  - The preflight restore drill passes and drops its scratch database; the PDO probe returns 0 as app over TLS 1.3, 3 on a wrong password and 5 when the app may CREATE.
derived:
  - The first deploy after the P6 engine flip finds a MariaDB-era state directory, so it takes a pg_dump of the PostgreSQL database and migrates normally; moving the data is the P6 cutover's job.
unknown:
  - Whether the preflight's PostgreSQL checks and the one-off migration container behave on the NAS exactly as locally (first staging run on the postgres engine, P6).
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Running migrations with docker compose exec in the runtime platform container would put the owner password into a long-running container; a one-off run container is used instead.
  - Cross-engine recovery (MariaDB backup into PostgreSQL): refused; the P6 cutover owns the data move.
changed_paths:
  - deploy/synology/postgres/init/10-platform-roles.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/scripts/recover-schema.sh
  - deploy/synology/scripts/prepare-fresh-schema-baseline.sh
  - deploy/synology/scripts/marketplace-staging.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - deploy/synology/README.md
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - tests/ci/test_synology_postgres_contract.py
validation:
  - command: python3 -m pytest tests/ci deploy/synology/tests -q
    result: PASS
    evidence: 411 passed, 91 subtests passed
  - command: shellcheck -x on the changed scripts, bash -n and sh -n
    result: PASS
    evidence: no new findings against the base head; role script clean
  - command: docker compose config, engines mariadb and postgres, with and without the postgres profile
    result: PASS
    evidence: postgres engine renders DB_CONNECTION=pgsql, DB_USERNAME=oteryn_platform_app, DB_SSLMODE=verify-full
  - command: hosted checks on PR #1581 head 8da9016120b1f726ed50a469eaa7c5550ccf7ad0 (exact candidate)
    result: PASS
    evidence: platform-gate success (run 38301476668, https://github.com/Oteryn/Oteryn-Platform/actions/runs/38301476668/job/114987502132); platform-postgres (16) and (17), runtime-tests and test success in the same run; checkpoint-validation success (run 38301476621); Validate Synology deployment package success (run 38301476592); Validate Character Bazaar staging package success (run 38301476718); static-validation success (run 38301476576); concurrency-proof and contract success; CodeQL and its four analyze jobs success; every other check on the head was conditionally skipped (live-preflight, live-dry-run, apply-reviewed-manifest, image build and dispatch), none failed. Review: the last submitted Codex review was on e1ecc6e (one P1, fixed in 8da9016 and its thread resolved); no Codex review was submitted on 8da9016 itself, and 0 review threads were unresolved at merge.
blockers:
  - none
next_action: none; P4b merged in #1581 (final PR head 8da9016, squash merge commit 9d2c4cf). P6 owns the engine flip and the data move.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: PR #1581 (final head 8da9016) merged as squash commit 9d2c4cf from claude/platform-pg-p4b-20261012; git ls-remote --heads origin claude/platform-pg-p4b-20261012 returns nothing after the merge (verified)
```

## Notes

The committed engine stays `mariadb`; the flip and the data move are packet P6. Route publication on `postgres` stays refused until the owner accepts the ADR 0044 PostgreSQL amendment.
