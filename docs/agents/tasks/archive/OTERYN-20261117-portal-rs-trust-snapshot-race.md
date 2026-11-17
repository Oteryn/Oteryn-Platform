---
task_id: OTERYN-20261117-portal-rs-trust-snapshot-race
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-portal-rs-trust-snapshot-race

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Portal RS CI test shard 1 fails intermittently in gameauth audit05 tests with
`Unavailable(trust.rs:52, Stale)` (main 0e29210e; PR 1898 head 17269518 in
`wave3_01_recovery_replay_expires_during_resigning`). Five audit05 fixture
sites read `now` before `registry.snapshot()` reads its own clock; when a
second boundary falls between the two, `trusted_key` refuses
`now < observed_at`. Fix: read the snapshot first at those five sites, as
every other activation site and the product path (tickets.rs, recovery.rs:
"trust self-check is refreshed before the issuer clock is read") already do.
No product code or guard change.

## Acceptance criteria

- [x] Red: a temporary 1.1 s sleep between the clock read and the snapshot reproduces the exact CI error in wave3_01
- [x] Green: with the reorder, the same sleep between the snapshot and the clock read passes
- [x] `cargo test --locked -p oteryn-gameauth --lib` 165 passed; fmt and clippy clean
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - portal-rs/crates/gameauth/src/admission/audit05_tests.rs
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-trust-snapshot-race.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-e2e-hosted.md
  - docs/agents/tasks/archive/OTERYN-20261116-portal-rs-e2e-hosted.md
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-league-fence-hold.md
  - docs/agents/tasks/archive/OTERYN-20261117-portal-rs-league-fence-hold.md
modules:
  - gameauth
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
updated_at: 2026-11-17T03:20:00Z
head: d6b16a54cccded8656c09ee343614d84e574d51c
branch: claude/happy-davinci-ijz3wy-trustrace
pr: 1900
status: completed
context_routes: []
owned_paths:
  - portal-rs/crates/gameauth/src/admission/audit05_tests.rs
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-trust-snapshot-race.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-e2e-hosted.md
  - docs/agents/tasks/archive/OTERYN-20261116-portal-rs-e2e-hosted.md
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-league-fence-hold.md
  - docs/agents/tasks/archive/OTERYN-20261117-portal-rs-league-fence-hold.md
proven:
  - All .activate( sites in portal-rs enumerated on origin/main; only the five audit05 sites read the clock before the snapshot
  - Product signing paths refresh trust before reading the clock
derived: []
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: audit05 tests Unavailable trust.rs:52 Stale
  evidence: Portal RS CI test shard 1 on main 0e29210e and PR 1898 head 17269518
rejected_hypotheses:
  - Product trust refresh order (already snapshot before clock)
changed_paths:
  - portal-rs/crates/gameauth/src/admission/audit05_tests.rs
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-trust-snapshot-race.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-e2e-hosted.md
  - docs/agents/tasks/archive/OTERYN-20261116-portal-rs-e2e-hosted.md
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-league-fence-hold.md
  - docs/agents/tasks/archive/OTERYN-20261117-portal-rs-league-fence-hold.md
validation:
  - command: cargo test --locked -p oteryn-gameauth --lib (PostgreSQL 18)
    result: PASS
    evidence: 165 passed
  - command: cargo fmt --all --check; cargo clippy --locked -p oteryn-gameauth --all-targets -- -D warnings
    result: PASS
    evidence: fmt clean; clippy finished with no warnings
blockers:
  - none
next_action: none; delivered by PR 1900 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1900 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1900 merged as d6b16a54cccded8656c09ee343614d84e574d51c
```
