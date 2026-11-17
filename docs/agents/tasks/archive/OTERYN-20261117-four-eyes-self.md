---
task_id: OTERYN-20261117-four-eyes-self
governing_issue: 1928
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-four-eyes-self

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1928
(review 5 findings). Plan §5.4 four eyes, first part: staff never act on
their own account.

1. RBAC: a role manager never grants a role to itself and never widens the
   permissions of a role it holds (`rbac/storage.rs` `change_impl`).
   Narrowing and revoking stay allowed.
2. Commerce: staff never submit a premium, balance or payment command whose
   beneficiary is their own account (reconciliation excepted), and never
   approve a pending operation whose beneficiary they are.

Out of scope (separate PR): a second staff approval for premium grants. It
needs a digest entry in the premium repository (approval has only the reason
hash) and a second staff identity in the E2E setup.

## Acceptance criteria

- [x] Self role grant is refused with no membership; widening a held role is refused; narrowing it succeeds (`db_role_managers_never_grant_or_widen_their_own_roles`)
- [x] Grant and Adjust targeting the actor are denied and audited; the beneficiary cannot approve a correction for itself (`postgres_staff_never_grant_premium_or_balance_to_themselves`)
- [x] E2E `staff-rbac-self` expects the self grant refusal and still proves self revocation
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-four-eyes-self.md
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/rbac/tests.rs
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/e2e/specs/staff-rbac-self.spec.mjs
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
updated_at: 2026-11-17T13:20:00Z
head: 528f4d455a41267300896795d7b39d1fa8a1dd79
branch: claude/happy-davinci-ijz3wy-four-eyes
pr: 1930
status: completed
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-four-eyes-self.md
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/rbac/tests.rs
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/e2e/specs/staff-rbac-self.spec.mjs
proven:
  - The new tests fail on main without the fix and pass with it (PostgreSQL 18)
  - No migration, contract, fixture, flag or workflow change
derived:
  - Only staff-rbac-self granted a role to its own actor among the E2E specs; staff-rbac-roles grants to another account
unknown:
  - Coordinator Jira synchronization
  - E2E staff-rbac-self run against a live portal
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Making premium grants pending in this PR fails at approval, because premium::mutate needs the plaintext reason and approval holds only its hash
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-four-eyes-self.md
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/rbac/tests.rs
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/e2e/specs/staff-rbac-self.spec.mjs
validation:
  - command: cargo test -p oteryn-admin --lib (PostgreSQL 18)
    result: PASS
    evidence: 199 passed, 0 failed
  - command: cargo fmt --all --check; cargo clippy -p oteryn-admin --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
  - command: Playwright staff-rbac-self.spec.mjs
    result: NOT_RUN
    evidence: no live portal in this session; node --check passes
blockers:
  - none
next_action: none; delivered by PR 1930 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1930 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1930 merged as 528f4d455a41267300896795d7b39d1fa8a1dd79
```
