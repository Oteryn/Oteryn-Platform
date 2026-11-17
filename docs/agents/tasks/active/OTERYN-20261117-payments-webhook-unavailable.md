---
task_id: OTERYN-20261117-payments-webhook-unavailable
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-payments-webhook-unavailable

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, coordinator decision (2026-11-17, after PR 1918). Payment webhooks
mapped every failed account or wallet read to `AccountDenied`, a definite
refusal. `Payments::webhook` (crates/payments/src/storage.rs) now answers
`Unavailable` when `state_in`/`refund_state_in` fails; the coin purchase
webhook (crates/payments/src/coin_store.rs: restriction read in `settle`,
`credit_in`, `reverse_in`) maps a host `CoinError::Unavailable` to
`Unavailable` and keeps every other host refusal as `AccountDenied`.
`HostAccounts` propagates a host storage failure as `Unavailable` instead of
reading it as `AccountState::Unknown`, so PR 1918's split also holds on the
coin path. Every refusal rolls the event back; a redelivery after recovery
applies it once.

## Acceptance criteria

- [x] Payments::webhook: failed account read on a settlement and on a refund gives Unavailable, writes no event, invoice or outbox row; redelivery applies once (red without the change)
- [x] Coin webhook: failed restriction read, credit and reversal debit give Unavailable, change no order or balance; redelivery applies once (red without the change)
- [x] Ok(Restricted/Unknown/Ambiguous) still gives AccountDenied
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-payments-webhook-unavailable.md
  - docs/agents/tasks/archive/OTERYN-20261117-payments-unavailable.md
  - docs/agents/tasks/archive/OTERYN-20261117-store-fence.md
  - portal-rs/crates/payments/src/coin_store.rs
  - portal-rs/crates/payments/src/storage.rs
  - portal-rs/crates/payments/tests/payments.rs
  - portal-rs/crates/payments/tests/provider_webhook.rs
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
updated_at: 2026-11-17T09:10:00Z
head: eb881d55af6fdc010ec520afd48871f15ac60fab
branch: claude/happy-davinci-ijz3wy-payments-webhook-unavailable
pr: none
status: ready
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-payments-webhook-unavailable.md
  - docs/agents/tasks/archive/OTERYN-20261117-payments-unavailable.md
  - docs/agents/tasks/archive/OTERYN-20261117-store-fence.md
  - portal-rs/crates/payments/src/coin_store.rs
  - portal-rs/crates/payments/src/storage.rs
  - portal-rs/crates/payments/tests/payments.rs
  - portal-rs/crates/payments/tests/provider_webhook.rs
proven:
  - No migration, amount, flag, contract, fixture or wire byte change
  - The provider webhook route already answers 503 for any coin webhook error; the in-process callers (store sandbox, coin sandbox) only test is_err, so no caller changes
  - Production CoinHost (bin/oteryn-portal/src/coin_purchases.rs) returns Unavailable for storage failures and Conflict for the balance ceiling, which keeps its hold path
derived:
  - The HostAccounts change is not observable through a public API (every caller maps the error to Unavailable already); it aligns the reader with the split
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-payments-webhook-unavailable.md
  - docs/agents/tasks/archive/OTERYN-20261117-payments-unavailable.md
  - docs/agents/tasks/archive/OTERYN-20261117-store-fence.md
  - portal-rs/crates/payments/src/coin_store.rs
  - portal-rs/crates/payments/src/storage.rs
  - portal-rs/crates/payments/tests/payments.rs
  - portal-rs/crates/payments/tests/provider_webhook.rs
validation:
  - command: new tests with the tests changed but the production files from main (PostgreSQL 18)
    result: PASS
    evidence: red as intended; disabled_and_ineligible_accounts_deny, webhook_account_read_failure_is_unavailable_and_redelivery_applies_once and host_storage_failure_in_a_coin_event_is_unavailable_and_a_retry_applies_once fail
  - command: cargo test -p oteryn-payments (PostgreSQL 18)
    result: PASS
    evidence: lib 12 passed, 1 ignored (pre-existing); payments 18 passed; provider_webhook 19 passed
  - command: cargo fmt --check -p oteryn-payments; cargo clippy -p oteryn-payments -p oteryn-store -p oteryn-admin --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
blockers:
  - none
next_action: open the PR and record CI on its head
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: not yet merged
source_branch_evidence: none
```
