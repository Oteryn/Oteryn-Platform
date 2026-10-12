---
task_id: OTERYN-20261012-platform-pg-p3
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
search_first:
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/lib.sh
optional_reads:
  - docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md
---

# OTERYN-20261012 Platform PostgreSQL P3: clean-slate bootstrap

## Goal

Governing GitHub Issue: #1474 (native-only migration) is the canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Implement plan packet P3 (PLATFORM_POSTGRES_MIGRATION_PLAN.md section 4, ADR 0046): one scripted procedure that takes an empty PostgreSQL database and an empty native evidence witness directory to a working Platform, refuses a re-run and every non-clean witness or database state before any change, and has a CI-runnable test. No production action; nothing runs against the Synology stack in this task.

## Acceptance criteria

- [x] `deploy/synology/scripts/bootstrap-postgres.sh` with `record-witness` (freeze record) and `run` (plan steps 1 to 8), Compose mode for Synology and `--local` mode for CI.
- [x] Bootstrap from an empty PostgreSQL database and an empty witness directory passes, with native admission off and on.
- [x] Refused before any change: re-run on a bootstrapped database, existing witness binding row, existing trust row, foreign store id, missing-profile directory, recorded directory advanced or removed after the freeze; stop before the schema when the new directory is not empty after the move.
- [x] Runbook `docs/operations/PLATFORM_POSTGRES_BOOTSTRAP.md`.
- [ ] Exact-head CI passes (pending at push).

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/bootstrap-postgres.sh
  - deploy/synology/tests/**
  - tests/ci/test_synology_*.py
  - docs/operations/PLATFORM_POSTGRES_BOOTSTRAP.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p3.md
modules:
  - synology-deploy
  - database
dependencies:
  - PR 1576 (P2 code compatibility; this branch is based on its head 7f787748)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T23:30:00Z
head: branch head of claude/platform-pg-p3-20261012 (the PR's head)
branch: claude/platform-pg-p3-20261012
pr: 1578
status: waiting
context_routes:
  - database
  - synology-deploy
owned_paths:
  - deploy/synology/scripts/bootstrap-postgres.sh
  - deploy/synology/tests/**
  - tests/ci/test_synology_*.py
  - docs/operations/PLATFORM_POSTGRES_BOOTSTRAP.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p3.md
proven:
  - Local run (PostgreSQL 16.15, PHP 8.5.11) of tests/ci/test_synology_postgres_bootstrap.py with OTERYN_PG_BOOTSTRAP_REQUIRE_LIVE=1, 18 tests pass; without PostgreSQL the 9 live cases skip.
  - With native admission off (APP_ENV production) the run migrates, seeds roles, creates both OAuth clients and the World row, passes production:verify-configuration, and leaves no trust row, no witness binding and an empty 0700 witness directory.
  - With native admission on (APP_ENV preproduction) publish-trust on PostgreSQL creates a new witness-store.id and its binding row; the MariaDB-era directory is moved aside byte-identical.
  - production:verify-configuration requires APP_ENV=production while publish-trust runs only in testing or preproduction, so the script runs the former only in production.
  - admin:bootstrap and wiki:launch-content:install need an MFA-confirmed identity, so they are operator follow-ups and not part of the empty-database run.
  - deploy/synology/tests/test_platform_mtls_contract.py requires every oteryn_native_admission_ingress_stop caller to return the listener behind the trust gate, so the bootstrap requires ingress stopped instead of stopping it, and starts only platform.
derived:
  - A SHA-256 manifest of the MariaDB-era directory at the freeze makes the "advanced after the freeze" state (missing-profile with the recorded store id) detectable, which a store-id comparison alone cannot.
unknown:
  - Behaviour on the Synology host; it needs P4 (postgres service, owner role variables DB_MIGRATIONS_USERNAME and DB_MIGRATIONS_PASSWORD) and runs only in P6.
conflicts:
  - The lead said not to touch deploy/synology, but the packet owns deploy/synology/scripts/bootstrap-postgres.sh and deploy/synology/tests/**; only those owned paths were written (only the script).
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Calling oteryn_native_admission_ingress_stop from the bootstrap; it breaks the mTLS listener contract because the bootstrap has no trust-gated way back.
  - Running health-check.sh inside the bootstrap; it needs the full stack, which the following deploy brings up.
  - Using oteryn_publish_native_route; it compares the issued WorldId with the configured fallback, which a clean slate changes.
changed_paths:
  - deploy/synology/scripts/bootstrap-postgres.sh
  - tests/ci/test_synology_postgres_bootstrap.py
  - docs/operations/PLATFORM_POSTGRES_BOOTSTRAP.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p3.md
validation:
  - command: OTERYN_PG_BOOTSTRAP_REQUIRE_LIVE=1 python3 tests/ci/test_synology_postgres_bootstrap.py (local PostgreSQL 16, PHP 8.5)
    result: PASS
    evidence: 18 tests, 0 skipped
  - command: python3 deploy/synology/tests/*.py and the unittest-style tests/ci/test_synology_*.py
    result: PASS
    evidence: local run; the pytest-style files need pytest, which is not installed locally
  - command: php8.5 vendor/bin/phpstan analyse; php8.5 vendor/bin/pint --test
    result: PASS
    evidence: local run on the unchanged PHP tree (no PHP file changed)
  - command: php8.5 vendor/bin/phpunit --configuration phpunit.pgsql.xml (local PostgreSQL 16, database oteryn_test)
    result: FAIL
    evidence: 969 tests, 2 errors and 2 failures, all in PreproductionStoreRouteCommandsTest (native topology on the preproduction store, the ADR 0044 amendment that P4 owns); no PHP file changed here, so they come from base 7f787748 or the local environment, and CI's platform_postgres job is the authority
  - command: python3 tools/validation/legacy_token_ratchet.py; python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: local run
blockers:
  - none
next_action: none for this task after merge. Hooking the live test into the platform_postgres CI job (ci.yml, not owned here) and wiring P4's variables are follow-ups for the lead.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR
source_branch_evidence: same-repository PR to main from claude/platform-pg-p3-20261012; the source ref is deleted automatically after merge
```

## Notes

No data is copied from MariaDB. The script never drops or empties a database or a witness directory and never edits the MariaDB-era witness.
