---
task_id: OTERYN-20261117-classb-identity
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-classb-identity

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, class B, step 2, identity crate. The account termination request and
cancel paths took the account row lock with `SELECT ... FOR UPDATE` on a plain
`pool.begin()` and had no lock or statement timeout. A held account lock made
both wait indefinitely. Each now arms `oteryn_db::mutation_budget` right after
`begin`, so it sheds within the shared three-second budget. The worker-only
`finalize` path is out of scope: it may wait on purpose.
`export.rs` is a read-only REPEATABLE READ transaction that already sets
`transaction_timeout`, so it needs no change.

## Acceptance criteria

- [x] A held account row lock sheds a termination request and a cancel as errors within the mutation budget (test red without the change, green with it), and nothing is written
- [x] Request, cancel, reschedule, finalize and race behaviour unchanged
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-identity.md
  - portal-rs/crates/identity/src/termination.rs
  - portal-rs/crates/identity/src/termination_tests.rs
modules:
  - identity
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
updated_at: 2026-11-17T05:11:18Z
head: c6361bd0a0d891f3d45d6dbe164a0b448362ac50
branch: claude/happy-davinci-ijz3wy-classb-identity
pr: 1905
status: completed
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-identity.md
  - portal-rs/crates/identity/src/termination.rs
  - portal-rs/crates/identity/src/termination_tests.rs
proven:
  - No migration, contract, fixture, flag or wire byte change
  - Refusals roll back; the pending termination keeps cancelled_at NULL and the event count unchanged
derived:
  - A per-crate raw-begin guard (coordinator note) is a later step once the remaining identity writers are triaged
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-identity.md
  - portal-rs/crates/identity/src/termination.rs
  - portal-rs/crates/identity/src/termination_tests.rs
validation:
  - command: cargo test --locked -p oteryn-identity --lib held_account_lock (PostgreSQL 18), without and with the change
    result: PASS
    evidence: postgres_held_account_lock_sheds_request_and_cancel_within_mutation_budget fails at the 10 s wait without the change and passes with it
  - command: cargo test --locked -p oteryn-identity (PostgreSQL 18)
    result: PASS
    evidence: lib 165 passed, integration 13 passed, 0 failed
  - command: cargo fmt -p oteryn-identity --check; cargo clippy --locked -p oteryn-identity --all-targets -- -D warnings
    result: PASS
    evidence: exit 0
blockers:
  - none
next_action: none; delivered by PR 1905 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1905 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1905 merged as c6361bd0a0d891f3d45d6dbe164a0b448362ac50
```
