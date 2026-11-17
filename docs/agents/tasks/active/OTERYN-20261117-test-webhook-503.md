---
task_id: OTERYN-20261117-test-webhook-503
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-test-webhook-503

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

The testing-only signed payment webhook (`/__test/payments/webhook`,
`crates/store/src/web_payments.rs`, compiled into the portal binary as
`shop_web`) answered 400 for every `Payments::webhook` error. A retryable
storage or account-read failure (`Error::Unavailable`, `Error::Database`) now
answers 503 so the sender redelivers; signature, input and state refusals stay
400 (coordinator decision). The route exists only with the testing gate, the
payments flag and an explicit test key; nothing else changes.

## Acceptance criteria

- [x] A valid signed event that hits a storage failure answers 503 (red on main's handler)
- [x] A forged signature, a missing signature and an unparsable body answer 400
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-test-webhook-503.md
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/store/src/web_payments_tests.rs
modules:
  - store
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
updated_at: 2026-11-17T09:30:00Z
head: eb881d55af6fdc010ec520afd48871f15ac60fab
branch: claude/happy-davinci-ijz3wy-test-webhook-503
pr: none
status: ready
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-test-webhook-503.md
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/store/src/web_payments_tests.rs
proven:
  - Payments::webhook verifies the signature and parses the event before any database access, so refusals never reach storage
  - The production provider webhook route already answers 503 for storage failures (crates/payments/src/provider_webhook.rs Delivery::Unavailable)
derived:
  - none
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-test-webhook-503.md
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/store/src/web_payments_tests.rs
validation:
  - command: cargo test -p oteryn-portal --bin oteryn-portal shop_web::tests::test_webhook (handler from origin/main)
    result: FAIL
    evidence: intended red; panics at the 503 assertion
  - command: cargo test -p oteryn-portal --bin oteryn-portal shop_web:: (PostgreSQL 18)
    result: PASS
    evidence: 10 passed, 1 ignored (pre-existing browser test)
  - command: cargo fmt --check -p oteryn-portal -p oteryn-store; cargo clippy -p oteryn-portal --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
blockers:
  - none
next_action: push and open the PR
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Task branch for the test webhook 503 PR; retained until merge
source_branch_evidence: The PR records the published head
```
