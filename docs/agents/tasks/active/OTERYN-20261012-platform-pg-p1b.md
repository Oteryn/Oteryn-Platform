---
task_id: OTERYN-20261012-platform-pg-p1b
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
search_first:
  - .github/workflows/ci.yml
optional_reads: []
---

# OTERYN-20261012 Platform PostgreSQL P1 stage B

## Goal

Governing GitHub Issue: #1474 (native-only migration) is the canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Promote the `platform-postgres` CI job from report to blocking on PostgreSQL 17 and 16, give the acceptance image `pdo_pgsql`, and record the accent-sensitivity decision in the federated search architecture.

## Acceptance criteria

- [x] `platform-postgres` has no `continue-on-error`, runs a 17 and 16 matrix, fails on any failing test, and is a need of `platform-gate`.
- [x] `deploy/ci/playwright-php.Dockerfile` installs and verifies `pdo_pgsql`.
- [x] `FEDERATED_SEARCH_ARCHITECTURE.md` states accent-sensitive PostgreSQL matching with no `unaccent`.
- [ ] Exact-head CI green on both PostgreSQL versions.
- [ ] Owner adds `platform-postgres (17)` and `platform-postgres (16)` to the branch ruleset (admin step, not done here).

## Ownership

```yaml
owned_paths:
  - .github/workflows/ci.yml
  - deploy/ci/playwright-php.Dockerfile
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p1b.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-pg-p1b.md
modules:
  - ci
dependencies:
  - PR 1576 (P1 stage A and P2 sweep)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T22:00:00Z
head: UNKNOWN
branch: claude/platform-pg-p1b-20261012
pr: none
status: validating
context_routes:
  - ci
owned_paths:
  - .github/workflows/ci.yml
  - deploy/ci/playwright-php.Dockerfile
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-p1b.md
proven:
  - the plan makes P1 stage B the removal of continue-on-error plus an owner ruleset edit
  - the workflow already loads pdo_pgsql in the platform-postgres job
derived:
  - other setup-php lists run SQLite or MariaDB suites and need no pdo_pgsql
unknown:
  - whether the suite is green on PostgreSQL 16 on this head
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - .github/workflows/ci.yml
  - deploy/ci/playwright-php.Dockerfile
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
validation:
  - command: yaml parse of ci.yml; tests/ci workflow policy scripts
    result: PASS
    evidence: local run
blockers:
  - none
next_action: wait for exact-head CI, then hand the ruleset step to the owner
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: pending
```

## Notes

BUILD_TEST_MATRIX.md and TEST_STRATEGY.md still describe the job as non-blocking; they are outside this packet and need a follow-up edit.
