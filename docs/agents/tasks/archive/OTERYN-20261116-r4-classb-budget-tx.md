---
task_id: OTERYN-20261116-r4-classb-budget-tx
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261116-r4-classb-budget-tx

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4 class B, step 1. Writer transactions opened with a plain
`pool.begin()` wait on row and advisory locks without the shared mutation
budget. Step 1 adds one helper, `oteryn_db::transaction::run_transaction`
with an opaque `BudgetTx`, and moves the premium snapshot and staff premium
mutation onto it. Later steps move the remaining writer sites crate by crate
and add a sweep that rejects new raw writer transactions.

## Acceptance criteria

- [x] `run_transaction` starts the mutation budget before the first statement, re-arms `lock_timeout` and `statement_timeout` to the time remaining (never 0) on every connection loan, and sets `transaction_timeout` on PostgreSQL 17+
- [x] `run_transaction` commits on `Ok` and rolls back on `Err`; `BudgetTx` has no public commit or rollback
- [x] A held premium authority lock sheds the snapshot as `Unavailable` within the budget (`snapshot_held_authority_lock_sheds_within_mutation_budget`, red before, green after)
- [x] Premium mutation keeps its lock order: authority row, then request serialization

Closeout of merged PR #1892: its packet `OTERYN-20261116-portal-rs-ci-shards` is archived as completed (same archive as #1893).

## Ownership

```yaml
owned_paths:
  - portal-rs/crates/db/src/lib.rs
  - portal-rs/crates/db/src/transaction.rs
  - portal-rs/crates/db/tests/transaction.rs
  - portal-rs/crates/store/src/premium/repository.rs
  - portal-rs/crates/store/src/premium_tests.rs
  - docs/agents/tasks/active/OTERYN-20261116-r4-classb-budget-tx.md
  - docs/agents/tasks/archive/OTERYN-20261116-portal-rs-ci-shards.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-ci-shards.md
modules:
  - db
  - store
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
updated_at: 2026-11-16T23:40:00Z
head: 477a744462a6bf41c963ab3cbaa5acd607bffa2e
branch: claude/happy-davinci-ijz3wy-r4classB-impl
pr: 1894
status: completed
context_routes: []
owned_paths:
  - portal-rs/crates/db/
  - portal-rs/crates/store/src/premium/repository.rs
  - portal-rs/crates/store/src/premium_tests.rs
  - docs/agents/tasks/active/OTERYN-20261116-r4-classb-budget-tx.md
  - docs/agents/tasks/archive/OTERYN-20261116-portal-rs-ci-shards.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-ci-shards.md
proven:
  - The red test was written on 85336fc6 and the branch is merged with main e9b0bcaaacf61d29593e38454fd77977d86335d2
  - No migration, contract, fixture, flag or wire byte change
  - No Game repository, production service or production data accessed
  - On the merged tree without the fix the snapshot test fails with the snapshot still waiting at 10 s
  - With the fix the snapshot sheds as Unavailable in about 3.0 s
derived:
  - The premium Error maps every sqlx error to Unavailable, so 55P03, 57014 and a transaction_timeout termination all surface as Unavailable
  - With lock_timeout equal to statement_timeout PostgreSQL reports a held lock as 57014; the helper test accepts either code
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - A second deadline beside oteryn.mutation_deadline was rejected; the helper reuses mutation_budget so all writers share one clock
changed_paths:
  - portal-rs/crates/db/src/lib.rs
  - portal-rs/crates/db/src/transaction.rs
  - portal-rs/crates/db/tests/transaction.rs
  - portal-rs/crates/store/src/premium/repository.rs
  - portal-rs/crates/store/src/premium_tests.rs
  - docs/agents/tasks/active/OTERYN-20261116-r4-classb-budget-tx.md
  - docs/agents/tasks/archive/OTERYN-20261116-portal-rs-ci-shards.md
validation:
  - command: cargo test --locked -p oteryn-store --lib snapshot_held_authority (fix absent)
    result: PASS
    evidence: expected red, snapshot still waiting at 10 s on a held authority lock
  - command: cargo test --locked -p oteryn-store --lib snapshot_held_authority, 3 runs
    result: PASS
    evidence: 3 of 3 passed in about 3.04 s
  - command: cargo test --locked -p oteryn-db --test transaction, 3 runs
    result: PASS
    evidence: 4 passed each on PostgreSQL 18, no SKIPPED line
  - command: cargo test --locked -p oteryn-store -p oteryn-db --no-fail-fast
    result: PASS
    evidence: store lib 122 passed 1 ignored browser test unchanged, db 3 plus 2 plus 4 passed, 0 failed
  - command: cargo clippy --locked -p oteryn-db -p oteryn-store --all-targets -- -D warnings
    result: PASS
    evidence: exit 0 (db and store); cargo clippy --locked --all-targets -D warnings on the whole workspace and cargo fmt --all --check also exit 0
blockers:
  - none
next_action: none; delivered by PR 1894 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1894 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1894 merged as 477a744462a6bf41c963ab3cbaa5acd607bffa2e
```
