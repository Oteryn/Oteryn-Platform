---
task_id: OTERYN-20261117-classb-characters
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-classb-characters

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, class B, step 2, characters crate. Saving profile preferences took
the account row lock with `SELECT ... FOR UPDATE` on a plain transaction and had
no lock or statement timeout. A held account lock made the save wait without
bound. `save` now arms `oteryn_db::mutation_budget` right after `begin`, so a
held lock sheds as `Unavailable` within the shared three-second budget. The
bounded retry still covers only serialization failures and deadlocks.
Restore notices and command receipts are internal Game ingest with their own
retry contract, so they are triaged separately.

## Acceptance criteria

- [x] A held account row lock sheds a profile preference save as Unavailable within the mutation budget (test red without the change, green with it)
- [x] Lock order (account before feed) and the bounded retry unchanged
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-characters.md
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
modules:
  - characters
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
head: fba3b7aaef6d116c44a9492dbe7c6d7184073926
branch: claude/happy-davinci-ijz3wy-classb-characters
pr: 1906
status: completed
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-characters.md
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
proven:
  - No migration, contract, fixture, flag or wire byte change
  - The shed check extends postgres_profile_takes_account_before_feed_like_termination, which keeps proving account-before-feed order
derived:
  - A per-crate raw-begin guard (coordinator note) is a later step once the remaining characters writers are triaged
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-characters.md
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
validation:
  - command: cargo test --locked -p oteryn-characters --lib account_before_feed (PostgreSQL 18), without and with the change
    result: PASS
    evidence: the profile update fails at the 10 s wait without the change and sheds as Unavailable with it
  - command: cargo test --locked -p oteryn-characters (PostgreSQL 18)
    result: PASS
    evidence: lib 97 passed, 0 failed
  - command: cargo fmt -p oteryn-characters --check; cargo clippy --locked -p oteryn-characters --all-targets -- -D warnings
    result: PASS
    evidence: exit 0
blockers:
  - none
next_action: none; delivered by PR 1906 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1906 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1906 merged as fba3b7aaef6d116c44a9492dbe7c6d7184073926
```
