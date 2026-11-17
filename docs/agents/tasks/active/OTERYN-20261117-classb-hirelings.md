---
task_id: OTERYN-20261117-classb-hirelings
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-classb-hirelings

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, class B, step 2, first crate. The hireling command enqueue opened its
writer transaction with a plain `pool.begin()` and waited on the issuer-wide
sequence row lock without the shared three-second mutation budget. It now runs in
`oteryn_db::transaction::run_transaction`. `oteryn_db::outbox::enqueue_on` takes
the connection of a transaction the caller owns, so the outbox row stays in the
budgeted transaction; `enqueue` delegates to it unchanged.

## Acceptance criteria

- [x] A held sequence row lock sheds the enqueue as a storage refusal within the mutation budget (test red without the change, green with it)
- [x] Lock order, refusal and replay behaviour unchanged; refusals still roll back
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-hirelings.md
  - docs/agents/tasks/active/OTERYN-20261117-r4-lock-order-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261117-r4-lock-order-fixes.md
  - portal-rs/crates/db/src/outbox.rs
  - portal-rs/crates/hirelings/src/commands.rs
  - portal-rs/crates/hirelings/tests/commands.rs
modules:
  - hirelings
  - db
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
updated_at: 2026-11-17T04:10:00Z
head: d2453556bdcb8cc3b66d0c22d5c57e5b45adc8b9
branch: claude/happy-davinci-ijz3wy-classb-small
pr: none
status: validating
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-hirelings.md
  - docs/agents/tasks/active/OTERYN-20261117-r4-lock-order-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261117-r4-lock-order-fixes.md
  - portal-rs/crates/db/src/outbox.rs
  - portal-rs/crates/hirelings/src/commands.rs
  - portal-rs/crates/hirelings/tests/commands.rs
proven:
  - No migration, contract, fixture, flag or wire byte change
  - The replay path now commits a transaction that wrote nothing; refusal paths still roll back
derived:
  - The site list for later crates is the class B inventory on Issue 530; sites whose first statement is an rbac lock helper already arm the budget
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-hirelings.md
  - docs/agents/tasks/active/OTERYN-20261117-r4-lock-order-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261117-r4-lock-order-fixes.md
  - portal-rs/crates/db/src/outbox.rs
  - portal-rs/crates/hirelings/src/commands.rs
  - portal-rs/crates/hirelings/tests/commands.rs
validation:
  - command: cargo test --locked -p oteryn-hirelings --test commands (PostgreSQL 18), without and with the change
    result: PASS
    evidence: postgres_held_sequence_lock_sheds_within_mutation_budget fails at the 10 s wait without the change and passes with it; the other 4 cases pass both ways
  - command: cargo test --locked -p oteryn-hirelings -p oteryn-db (PostgreSQL 18)
    result: PASS
    evidence: 9 test binaries ok, 0 failed
  - command: cargo fmt --all --check; cargo clippy --locked -p oteryn-hirelings -p oteryn-db --all-targets -- -D warnings
    result: PASS
    evidence: exit 0
blockers:
  - none
next_action: open the PR and wait for Portal RS CI
```
