---
task_id: OTERYN-20261012-platform-pg-ci-probe
governing_issue: 1474
required_reads: []
search_first:
  - .github/workflows/ci.yml
  - app/Wallet/WalletMutator.php
  - app/Marketplace/Actions/PlaceAuctionBid.php
optional_reads: []
---

# OTERYN-20261012 Platform PostgreSQL CI probe (P1)

## Goal

Governing GitHub Issue: #1474 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Packet P1 of the Platform PostgreSQL migration plan (PR #1561). Stage A: add a non-blocking `platform-postgres` job to the existing `ci.yml` that runs the whole Platform suite on a `postgres:17` service and uploads the failing-test list as the work queue for P2. Add a `pgsql` connection and `phpunit.pgsql.xml`. Add real eight-process wallet-debit and auction-bid concurrency tests that run only on PostgreSQL, to close review finding F-05. No behaviour change for the default sqlite and mysql connections. Stage B (blocking) waits for P2.

## Acceptance criteria

- [x] `platform-postgres` job exists in `ci.yml`, is `continue-on-error`, is not in the needs of `test` or `platform-gate`, and no check name changes; workflow count stays 53.
- [x] `config/database.php` has a `pgsql` connection read from the environment; defaults unchanged.
- [x] Wallet and bid concurrency tests pass on PostgreSQL and fail when `lockForUpdate` is removed locally (negative check; not committed).
- [x] The new tests skip on sqlite and the existing MariaDB job.
- [ ] Exact-head CI passes and the probe output is recorded in the PR body.

## Ownership

```yaml
owned_paths:
  - .github/workflows/ci.yml
  - phpunit.pgsql.xml
  - config/database.php
  - tests/Feature/Wallet/Concurrency/**
  - tests/Feature/Marketplace/Concurrency/**
  - tests/Support/Concurrency/**
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-ci-probe.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-pg-ci-probe.md
modules:
  - ci
  - wallet
  - marketplace
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T10:45:00Z
head: UNKNOWN
branch: claude/platform-pg-ci-probe-20261012
pr: 1563
status: validating
context_routes:
  - ci-workflows
  - database
owned_paths:
  - .github/workflows/ci.yml
  - phpunit.pgsql.xml
  - config/database.php
  - tests/Feature/Wallet/Concurrency/**
  - tests/Feature/Marketplace/Concurrency/**
  - tests/Support/Concurrency/**
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-pg-ci-probe.md
proven:
  - concurrency tests pass on a local PostgreSQL 16 and fail when lockForUpdate is removed from WalletMutator::lock and PlaceAuctionBid
  - legacy token ratchet unchanged (fixtures resolve legacy-prefixed column names from the schema)
derived:
  - PostgreSQL 17 behaviour matches 16 for row locking
  - on head 18f93b34 the PostgreSQL 17 probe reported 960 tests, 29 skipped, 113 failing (the P2 queue) and all 5 concurrency tests passed
unknown: []
conflicts: []
first_failure:
  marker: platform-postgres red on 18f93b34 (phpunit exit 2)
  evidence: job-level continue-on-error keeps the workflow green but the check run still reports failure for the known P2 suite failures
rejected_hypotheses:
  - a new regression on PostgreSQL 17 (the failing list matches the local PostgreSQL 16 probe and the concurrency tests pass)
changed_paths:
  - .github/workflows/ci.yml
  - phpunit.pgsql.xml
  - config/database.php
  - tests/Support/Concurrency
  - tests/Feature/Wallet/Concurrency
  - tests/Feature/Marketplace/Concurrency
validation:
  - command: phpunit concurrency tests on PostgreSQL 16; workflow inventory, pinning, ci contract tests, legacy token ratchet, checkpoint
    result: PASS
    evidence: local run
  - command: summary step replayed on the 18f93b34 probe JUnit (pass) and with a failed, skipped or missing concurrency test or no report (fails)
    result: PASS
    evidence: local run against the ci-pgsql-probe artifact
blockers:
  - none
next_action: Read the platform-postgres artifact on the exact head, hand the failing list to P2, then archive this record when stage B lands.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR
source_branch_evidence: pending
```

## Notes

Negative check, not committed: removing `lockForUpdate` from `WalletMutator::lock` fails all three wallet tests; removing it from `PlaceAuctionBid` fails both bid tests. `unsignedBigInteger` columns are not non-negative on PostgreSQL either, so the application check in `WalletMutator::applyLocked` is the only guard; the tests prove it holds under races.
