---
task_id: OTERYN-20261117-r4-lock-order-fixes
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-r4-lock-order-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4 review claims that were reproduced as real on PostgreSQL 18, fixed with a
red-then-green test each and integrated into one PR:

- lockemail: email change took the account row before the address claim; the
  address claim now comes first in request, confirmation/revert and apply_due.
- grantroute: protocol 3 grant issuance re-checks before and after signing that no
  other route record of the World took the selected endpoint (Decision M1).
- coinrev: coin purchase create re-checks for a reversal hold under a shared
  wallet row lock.
- lockcmd: character command receipt joins the shared issuance fence before the
  command row.
- profdeadlock: character profile reads and saves lock account rows before the
  feed, the same order as termination finalization.

Claims pdsessions and lockref were checked and are not real; no change.

## Acceptance criteria

- [x] Each fix has a test that fails without it (worker report) and passes with it (rerun here)
- [x] Focused crate tests pass on the integrated tree
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20261116-r4-classb-budget-tx.md
  - portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs
  - portal-rs/crates/characters/src/commands/storage.rs
  - portal-rs/crates/characters/src/commands/tests.rs
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
  - portal-rs/crates/gameauth/src/admission/audit05_tests.rs
  - portal-rs/crates/gameauth/src/admission/grant.rs
  - portal-rs/crates/gameauth/src/admission/tickets.rs
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
  - portal-rs/crates/payments/src/coin_store.rs
  - docs/agents/tasks/active/OTERYN-20261117-r4-lock-order-fixes.md
modules:
  - identity
  - gameauth
  - payments
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
updated_at: 2026-11-17T01:30:00Z
head: 477a744462a6bf41c963ab3cbaa5acd607bffa2e
branch: claude/happy-davinci-ijz3wy-r4-fixes
pr: pending
status: validating
context_routes: []
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20261116-r4-classb-budget-tx.md
  - portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs
  - portal-rs/crates/characters/src/commands/storage.rs
  - portal-rs/crates/characters/src/commands/tests.rs
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
  - portal-rs/crates/gameauth/src/admission/audit05_tests.rs
  - portal-rs/crates/gameauth/src/admission/grant.rs
  - portal-rs/crates/gameauth/src/admission/tickets.rs
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
  - portal-rs/crates/payments/src/coin_store.rs
  - docs/agents/tasks/active/OTERYN-20261117-r4-lock-order-fixes.md
proven:
  - No migration, contract, fixture, flag or wire byte change
  - No Game repository, production service or production data accessed
  - Fix branches lockemail 1608e7a1, grantroute f6ae89a4, coinrev 76ff3f71, lockcmd 83e380ff, profdeadlock 8db4aae3 merged into origin/main 477a7444 without conflicts
derived:
  - grantroute leaves a window between the last check and commit; the portal has no route writer
  - Lock order checked across the workspace: claim_lock callers take the address before any account row; coin create (payer fence, wallet share, quote) is compatible with reversal (order, wallet, hold); command receipts take restore, fence, command like service receipts; feed publishers lock no account after the feed
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - pdsessions (sessions survive a sensitive change) not real
  - lockref (lock order inversion in payments, wallet, store) not real
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20261116-r4-classb-budget-tx.md
  - portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs
  - portal-rs/crates/characters/src/commands/storage.rs
  - portal-rs/crates/characters/src/commands/tests.rs
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
  - portal-rs/crates/gameauth/src/admission/audit05_tests.rs
  - portal-rs/crates/gameauth/src/admission/grant.rs
  - portal-rs/crates/gameauth/src/admission/tickets.rs
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
  - portal-rs/crates/payments/src/coin_store.rs
  - docs/agents/tasks/active/OTERYN-20261117-r4-lock-order-fixes.md
validation:
  - command: red-then-green per fix (r4_address_claim_wait_retains_no_account_row, r4_protocol3_refuses_an_endpoint_shared_after_selection, create_waiting_on_the_payer_claim_sees_a_reversal_hold, db_receipt_publication_waits_for_shared_issuance_fence, postgres_profile_takes_account_before_feed_like_termination)
    result: PASS
    evidence: red side reported by the round-4 worker on each fix branch; green side rerun here on the integrated tree (row below)
  - command: cargo test --locked -p oteryn-identity / -p oteryn-gameauth / -p oteryn-payments / -p oteryn-characters, -p oteryn-portal coin_purchases (PostgreSQL 18)
    result: PASS
    evidence: identity 164+13, gameauth 166, payments 12 (1 ignored, unchanged)+15+18, characters 1+97, portal coin_purchases 25 passed, 0 failed
  - command: cargo fmt --all --check; cargo clippy --locked (five crates) --all-targets -- -D warnings
    result: PASS
    evidence: exit 0
blockers:
  - none
next_action: push, open PR, wait for Portal RS CI
```
