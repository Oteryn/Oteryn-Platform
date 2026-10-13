---
task_id: OTERYN-20261012-platform-pg-p4
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/architecture/adr/0044-preproduction-store-native-route-publication.md
search_first:
  - deploy/synology/compose.yml
  - deploy/synology/scripts/lib.sh
  - app/GameAuth/Worlds/PreproductionNativeRouteStore.php
optional_reads: []
---

# OTERYN-20261012 Platform PostgreSQL P4: Synology service, TLS and role split

## Goal

Governing GitHub Issue: #1474. Packet P4 of `docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md` (ADR 0046, finding F-06): add an idle, digest-pinned PostgreSQL 17 service beside MariaDB on the Synology stack with TLS 1.3 from the stack CA, SCRAM-only `pg_hba`, the owner/app/backup/dump role split, an engine switch that keeps MariaDB, health and volume guards, and the ADR 0044 PostgreSQL store-identity amendment that makes the four preproduction-store tests pass on PostgreSQL. Placeholders only; role secrets are an owner action. No NAS deploy.

## Acceptance criteria

- [x] `postgres` Compose service: digest pin, profile `postgres`, private bridge only, no published or exposed port, `restart: always`, named volume `postgres_data`, health check = `pg_isready` plus a `verify-full` login of the runtime role.
- [x] `tls-init` issues a `postgres` server certificate from a dedicated PostgreSQL CA (key in a tls-init-only volume) into `postgres_tls` and copies only the CA certificate into `postgres_client_ca`; Platform and the scheduler mount only the CA.
- [x] `pg_hba.conf` admits only `hostssl` SCRAM logins of the Platform roles (local peer for `postgres`); `postgresql.conf` enforces TLS 1.3 and SCRAM.
- [x] Role script: owner owns database, schema and objects; app has DML only; dump `SELECT` only; backup `REPLICATION` only; no superuser, createdb, createrole or bypassrls; passwords on stdin, never in argv, env or logs.
- [x] `PLATFORM_DB_ENGINE=mariadb` committed; image contract carries the engine and a mismatch is refused; deploy and the MariaDB-only operator scripts refuse `postgres` in P4.
- [x] Five optional environment secrets rendered all-or-none; deploy reconciles and proves the role contract when set, and the health check then requires a healthy unpublished `postgres`.
- [x] No workflow or deploy script removes a data volume (`tests/ci/test_synology_postgres_contract.py`, run by ci.yml through `test_synology_deploy_release_identity.py`).
- [x] ADR 0044 Amendment P4 (Proposed); `PreproductionStoreRouteCommandsTest` passes on SQLite and PostgreSQL.
- [ ] Owner accepts ADR 0044 Amendment P4 and sets the five `synology-staging` secrets.
- [ ] Exact-head CI passes and the PR merges.

## Ownership

```yaml
owned_paths:
  - deploy/synology/compose.yml
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/postgres/**
  - deploy/synology/tls/**
  - deploy/synology/scripts/{deploy,lib,health-check,deploy-impact,recover-schema,prepare-fresh-schema-baseline,production-target-preflight,marketplace-staging,rollback}.sh
  - deploy/synology/release-contract.env
  - deploy/synology/README.md
  - deploy/synology/tests/**
  - tests/ci/test_synology_*.py
  - .github/workflows/{deploy-synology-staging,recover-synology-staging-schema,synology-production-target-preflight,synology-diagnostics,synology-container-hygiene,repair-synology-autostart,repair-synology-compose-orphans}.yml (database-service references only)
  - app/GameAuth/Worlds/PreproductionNativeRouteStore.php
  - tests/Feature/GameAuth/NativeTopology/PreproductionStoreRouteCommandsTest.php
  - docs/architecture/adr/0044-preproduction-store-native-route-publication.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p4.md
modules:
  - deploy
  - game-auth
dependencies:
  - PR #1576 (P2/P3 base, branch claude/beautiful-franklin-1tp8kj)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T18:00:00Z
head: UNKNOWN
branch: claude/platform-pg-p4-20261012
pr: 1579
status: waiting
context_routes:
  - deploy
  - security
  - game-auth
owned_paths:
  - deploy/synology/**
  - tests/ci/test_synology_*.py
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/repair-synology-autostart.yml
  - .github/workflows/repair-synology-compose-orphans.yml
  - .github/workflows/synology-diagnostics.yml
  - app/GameAuth/Worlds/PreproductionNativeRouteStore.php
  - tests/Feature/GameAuth/NativeTopology/PreproductionStoreRouteCommandsTest.php
  - docs/architecture/adr/0044-preproduction-store-native-route-publication.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p4.md
proven:
  - Against the pinned image locally, TLS 1.2, non-TLS, superuser-over-TCP and other-database logins are refused; backup logs in only for replication.
  - Migrations as the owner create every table; app and dump own no relation; app DDL and TRUNCATE are denied; dump writes are denied; a dump-role pg_dump restores with matching row counts.
  - The role script is idempotent and prints no password; the health check fails on a wrong password.
derived:
  - The superuser password only seeds initdb; pg_hba never admits the superuser over TCP, so rotating that secret later has no effect.
unknown:
  - Whether the NAS Docker version honours the Compose profile and health check exactly as local Compose v5 does (first staging deploy with the secrets set).
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Passing role passwords with docker exec -e: they would show in the container's process environment; stdin is used instead.
changed_paths:
  - deploy/synology/**
  - tests/ci/test_synology_postgres_contract.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tests/ci/test_synology_rollback_contract.py
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/repair-synology-autostart.yml
  - .github/workflows/repair-synology-compose-orphans.yml
  - .github/workflows/synology-diagnostics.yml
  - app/GameAuth/Worlds/PreproductionNativeRouteStore.php
  - tests/Feature/GameAuth/NativeTopology/PreproductionStoreRouteCommandsTest.php
  - docs/architecture/adr/0044-preproduction-store-native-route-publication.md
validation:
  - command: python3 -m pytest tests/ci deploy/synology/tests -q
    result: PASS
    evidence: see PR body for counts
  - command: phpunit PreproductionStoreRouteCommandsTest (phpunit.xml and phpunit.pgsql.xml)
    result: PASS
    evidence: 6 tests each on SQLite and PostgreSQL 17
  - command: phpstan, pint --test, shellcheck, docker compose config (both engines, with and without the postgres profile)
    result: PASS
    evidence: no findings
blockers:
  - none
next_action: none; P4 merged in #1579 and the owner sets the five secrets and accepts ADR 0044 Amendment P4.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: PR #1579 merged from claude/platform-pg-p4-20261012. That ref name was reused for follow-up PR #1585, so the ref still exists as #1585's head and is not absent yet; #1585 uses the same auto_delete_after_merge disposition and its merge deletes the ref (verify with git ls-remote --heads origin claude/platform-pg-p4-20261012 after #1585 merges). This follow-up carries the rollback secret validation and the archive move
```

## Notes

Deferred to P4b/P6 (deploy refuses `PLATFORM_DB_ENGINE=postgres` until then): the owner-role migration step via a one-shot container, and the PostgreSQL branches of schema recovery, the fresh-schema baseline, Marketplace staging control and the production-target preflight.

Unowned follow-ups: `build-synology-staging-images.yml` path filters, `sh -n` list and test list, and `scripts/ci/classify_synology_builds.py` should include `deploy/synology/postgres/`; `deploy/synology/.env.example` should carry the five placeholders; `character-bazaar-staging-control.yml` renders no PostgreSQL secrets, which is safe because the state then reads as absent.
