---
task_id: OTERYN-20261117-email-chain
governing_issue: 1928
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-email-chain

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1928
(review 5 findings). While an applied primary email change can still be
reverted, no new primary change is accepted, so a revert always returns the
account to the address from before the change, never to one link of a chain
of changes (`email_change::request`).

## Acceptance criteria

- [x] X to Y applied; a request for Z inside Y's revert window is refused; the revert restores X; after the revert a new change is accepted (`r5_no_chained_primary_change_inside_the_revert_window`)
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-email-chain.md
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
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
updated_at: 2026-11-17T13:00:00Z
head: 149b24b6
branch: claude/happy-davinci-ijz3wy-email-chain
pr: 1929
status: ready
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-email-chain.md
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
proven:
  - The new test is red without the fix (panic at email_change_tests.rs:792) and green with it (PostgreSQL 18)
  - No migration, contract, fixture, flag or workflow change
derived:
  - Backup address changes are unaffected; only primary changes are serialized
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-email-chain.md
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
validation:
  - command: cargo test -p oteryn-identity --lib (PostgreSQL 18)
    result: PASS
    evidence: 166 passed, 0 failed; the new test fails without the fix
  - command: cargo fmt --all --check; cargo clippy -p oteryn-identity --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
blockers:
  - none
next_action: Portal RS CI on the PR head; archive this packet once the PR lands.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Task branch for the chained email change fix; retained until merge
source_branch_evidence: The branch records the published head
```
