---
task_id: OTERYN-20261117-payments-unavailable
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-payments-unavailable

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, coordinator decision (2026-11-17, before the deliver_refund redesign).
`Payments::eligible` (crates/payments/src/storage.rs) mapped every outcome of
`AccountReader::state` other than `Ok(Active)` to `AccountDenied`, so a timed
out or failed account read (for example under the store read budget from PR
1917) was reported as a permanent refusal. It now maps `Ok(Active)` to admit,
`Ok(other)` to `AccountDenied` and `Err(_)` to a new `Error::Unavailable`
(retryable, 503 at the callers). `create_intent`, `ensure_intent` and
`intent` use `eligible`, so all three change; nothing is written on either
refusal. The webhook and `intent_in` paths are unchanged.

## Acceptance criteria

- [x] A failed account read gives Unavailable, not AccountDenied, for create_intent, ensure_intent and intent (red without the change)
- [x] Ok(Restricted/Unknown/Ambiguous) still gives AccountDenied
- [x] ensure_intent whose re-read after commit fails returns Unavailable; the repeated request returns the same intent and no second row is written
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-payments-unavailable.md
  - portal-rs/crates/payments/src/core.rs
  - portal-rs/crates/payments/src/storage.rs
  - portal-rs/crates/payments/tests/payments.rs
modules:
  - payments
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
updated_at: 2026-11-17T12:40:00Z
head: 4f7f55e72e4a08447261831b37234f351ba2b58c
branch: claude/happy-davinci-ijz3wy-payments-unavailable
pr: 1918
status: completed
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-payments-unavailable.md
  - portal-rs/crates/payments/src/core.rs
  - portal-rs/crates/payments/src/storage.rs
  - portal-rs/crates/payments/tests/payments.rs
proven:
  - No migration, amount, flag, contract, fixture or wire byte change
  - Every caller outside the crate already answers 503 for the new variant; store web_payments and coin_store map any payments error to Unavailable, admin payment_error maps it through its catch-all to Unavailable
  - Both refusals return before the provider call or before any write, so neither creates an intent
derived:
  - Active packets OTERYN-20261115-audit-a2-l6 and OTERYN-20261116-a3-drc list these paths but have no PR (pr none), so no live writer overlaps
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-payments-unavailable.md
  - portal-rs/crates/payments/src/core.rs
  - portal-rs/crates/payments/src/storage.rs
  - portal-rs/crates/payments/tests/payments.rs
validation:
  - command: cargo test -p oteryn-payments --test payments -- account_state_read_failure ensure_intent_reread (PostgreSQL 18), with the variant but without the eligible change
    result: PASS
    evidence: red as intended; both new tests fail
  - command: cargo test -p oteryn-payments (PostgreSQL 18)
    result: PASS
    evidence: lib 12 passed, 1 ignored (pre-existing); payments 17 passed; provider_webhook 18 passed
  - command: cargo test -p oteryn-store -p oteryn-admin payment (PostgreSQL 18)
    result: PASS
    evidence: 2 passed, 0 failed
  - command: cargo fmt -p oteryn-payments -- --check; cargo clippy -p oteryn-payments -p oteryn-admin -p oteryn-store --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0 with no warnings
blockers:
  - none
next_action: none; delivered by PR 1918 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1918 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1918 merged as 4f7f55e72e4a08447261831b37234f351ba2b58c
```
