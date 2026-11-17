---
task_id: OTERYN-20261117-classb-atlas
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-classb-atlas

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, class B, step 2, atlas. The private marker and map report handlers
opened writer transactions with a plain `pool.begin()` and waited on the account
row lock without the shared three-second mutation budget. Both now arm
`oteryn_db::mutation_budget` before the first statement, as the web session
rotation already does. `oteryn-db` moves from dev-dependencies to dependencies.

## Acceptance criteria

- [x] A held account row lock sheds marker creation and map reports as 503 within the mutation budget (test red without the change, green with it)
- [x] Lock order and refusals unchanged
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-atlas.md
  - portal-rs/crates/atlas/Cargo.toml
  - portal-rs/crates/atlas/src/spawns.rs
  - portal-rs/crates/atlas/src/spawns_tests.rs
modules:
  - atlas
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
updated_at: 2026-11-17T04:15:00Z
head: a5bd9b02a10ecb1b4d346f4d09f42f4281d1943e
branch: claude/happy-davinci-ijz3wy-classb-atlas
pr: 1903
status: completed
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-atlas.md
  - portal-rs/crates/atlas/Cargo.toml
  - portal-rs/crates/atlas/src/spawns.rs
  - portal-rs/crates/atlas/src/spawns_tests.rs
proven:
  - No migration, contract, fixture, flag or wire byte change
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
  - docs/agents/tasks/active/OTERYN-20261117-classb-atlas.md
  - portal-rs/crates/atlas/Cargo.toml
  - portal-rs/crates/atlas/src/spawns.rs
  - portal-rs/crates/atlas/src/spawns_tests.rs
validation:
  - command: cargo test --locked -p oteryn-atlas --lib held_account (PostgreSQL 18), without and with the change
    result: PASS
    evidence: postgres_held_account_lock_sheds_marker_and_report_within_mutation_budget fails at the 10 s wait without the change and passes with it
  - command: cargo test --locked -p oteryn-atlas (PostgreSQL 18)
    result: PASS
    evidence: 34 passed, 0 failed
  - command: cargo fmt --all --check; cargo clippy --locked -p oteryn-atlas --all-targets -- -D warnings
    result: PASS
    evidence: exit 0
blockers:
  - none
next_action: none; delivered by PR 1903 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1903 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1903 merged as a5bd9b02a10ecb1b4d346f4d09f42f4281d1943e
```
