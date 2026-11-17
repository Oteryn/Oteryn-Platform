---
task_id: OTERYN-20261117-premium-4eyes-b
governing_issue: 1928
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-premium-4eyes-b

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1928
(D2 follow-up, owner request 2026-11-17). Two residual risks from D2 (#1933):

1. A replayed grant submit whose target became value-ineligible after the
   grant was recorded returned `Denied` and wrote another `commerce.denied`
   audit row. Submit now applies the eligibility check only to an operation
   that is not yet recorded (its row is locked before the check); a replay
   reports the recorded state.
2. No test covered an audit failure during grant approval. A new test proves
   the entitlement and the completion roll back and the grant stays
   approvable.

## Acceptance criteria

- [x] A replayed grant submit after the target turns ineligible reports the recorded pending state and writes no `commerce.denied` row (`postgres_grant_replay_after_target_turns_ineligible_reports_recorded_state`; red on the D2 tree)
- [x] An audit failure on grant approval rolls back the entitlement and completion, and a later approval succeeds (`postgres_grant_approval_audit_failure_rolls_back_entitlement`)
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-premium-4eyes-b.md
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - docs/agents/tasks/archive/OTERYN-20261117-premium-4eyes.md
  - docs/agents/tasks/active/OTERYN-20261117-premium-4eyes.md
modules:
  - admin
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
updated_at: 2026-11-17T14:37:00Z
head: 5510ab0a840a009eecdab01fd988823a258d1f30
branch: claude/happy-davinci-ijz3wy-premium-4eyes-b
pr: 1934
status: ready
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-premium-4eyes-b.md
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - docs/agents/tasks/archive/OTERYN-20261117-premium-4eyes.md
  - docs/agents/tasks/active/OTERYN-20261117-premium-4eyes.md
proven:
  - The replay test fails on the D2 tree (Err(Denied) at the replay) and passes with the change (PostgreSQL 18)
  - No migration, contract, fixture, flag or workflow change
derived:
  - The operation row is locked FOR UPDATE inside the role-fence resource closure before the eligibility check, so the existence check is stable for the transaction
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Moving the eligibility check after the insert; that would record a pending row for an ineligible target
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-premium-4eyes-b.md
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - docs/agents/tasks/archive/OTERYN-20261117-premium-4eyes.md
validation:
  - command: cargo test -p oteryn-admin --lib commerce:: (PostgreSQL 18)
    result: PASS
    evidence: 35 passed, 0 failed on main 5510ab0a plus this change
  - command: cargo fmt --check -p oteryn-admin; cargo clippy -p oteryn-admin --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
blockers:
  - none
next_action: Portal RS CI on the PR head; archive this packet once the PR lands.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
