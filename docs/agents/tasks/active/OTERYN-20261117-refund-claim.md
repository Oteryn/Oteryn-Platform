---
task_id: OTERYN-20261117-refund-claim
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-refund-claim

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, money path, admin commerce. The outbox job `commerce.refund`
(`TestPaymentAdapter::deliver_refund`) held the refund row lock
(`FOR UPDATE OF r`) and an open transaction across the external provider
refund call. It now claims the row in a short budgeted transaction (state
`pending` to `refunding`, fresh UUID v7 `attempt_id`, `claimed_at`), calls
the provider outside any transaction, then finalizes in a short budgeted
transaction conditional on `attempt_id` and `state='refunding'`. A claim
younger than `REFUND_CLAIM_STALE_AFTER` (120 s, four times the worker's 30 s
execution budget) answers unavailable so the outbox retries; an older one is
re-claimed. A provider error or an invalid reference releases the claim back
to `pending` (conditional on the attempt id) so the outbox retry dispatches
again at once. The provider idempotency key stays the intent id
(`PaymentProvider::refund(intent_id, amount)`, `commerce_refunds.intent_id`
UNIQUE); the attempt id never crosses the provider interface. No timeout on
the external call (coordinator decision). The `commerce/effects.rs` entry
leaves the crate begin guard's exempt list.

## Acceptance criteria

- [x] The refund row can be locked with NOWAIT while the provider call is in flight (red without the change)
- [x] A concurrent delivery during a fresh claim answers unavailable; the provider is called once (red without the change)
- [x] A stale claim is re-claimed with the same idempotency key; the stale attempt's late finalize writes nothing
- [x] A provider error or a malformed reference releases the claim; the retry succeeds with one dispatch audit
- [x] A recorded reference or a ledger-refunded intent settles without a provider call
- [x] Crate guard passes with the `commerce/effects.rs` exempt entry removed
- [x] A stale attempt finishing under a live re-claim neither records its reference nor releases the claim (red with the attempt-id fence removed from finalize and release)
- [x] The last failed outbox attempt of an unsettled refund logs at error level with operation, state and attempt id (decision tested on PostgreSQL); the reconciler is a separate task, issue #1922
- [ ] Portal RS CI green on the PR head (no PR yet)

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-refund-claim.md
  - portal-rs/Cargo.lock
  - portal-rs/crates/admin/Cargo.toml
  - portal-rs/crates/admin/src/begin_guard_tests.rs
  - portal-rs/crates/admin/src/commerce/README.md
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/migrations/20261117082011_round4_refund_claim.sql
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
updated_at: 2026-11-17T09:30:00Z
head: 83c0255669c088c639b6e24234fadde5d65da337
branch: claude/happy-davinci-ijz3wy-refund-claim
pr: none
status: ready
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-refund-claim.md
  - portal-rs/Cargo.lock
  - portal-rs/crates/admin/Cargo.toml
  - portal-rs/crates/admin/src/begin_guard_tests.rs
  - portal-rs/crates/admin/src/commerce/README.md
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/migrations/20261117082011_round4_refund_claim.sql
proven:
  - Migration is additive; existing rows with provider_reference read as done and others as pending (scratch schema check on PostgreSQL 18)
  - No CHECK ties state to provider_reference writes of the previous binary; provider_reference IS NOT NULL stays the first settled check
  - Provider idempotency key is the intent id (crates/payments/src/provider.rs refund signature and contract comment); unchanged
  - No contract, fixture, flag or workflow change; worker.rs and crates/payments unchanged
  - Only dependency change is tracing 0.1.44 in crates/admin (already in the workspace lockfile; Cargo.lock gains one line)
  - A crash or deadline drop on the last attempt is marked dead by the outbox claim query without running the handler; only outbox.state and last_error record it (outbox::counts has no exporter); recorded in issue #1922
derived:
  - Outbox retry backoff (2^attempts x 0.5-1 s, ten attempts) leaves several attempts after a crashed claim goes stale at 120 s
unknown:
  - Coordinator Jira synchronization
  - Migration version uniqueness checked only against locally fetched remote refs
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-refund-claim.md
  - portal-rs/Cargo.lock
  - portal-rs/crates/admin/Cargo.toml
  - portal-rs/crates/admin/src/begin_guard_tests.rs
  - portal-rs/crates/admin/src/commerce/README.md
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/migrations/20261117082011_round4_refund_claim.sql
validation:
  - command: cargo test --locked -p oteryn-admin --lib postgres_refund (PostgreSQL 18), on the old transaction shape and after the change
    result: PASS
    evidence: before, lock probe failed 55P03 and concurrent delivery hit the 5 s bound (behavioural red); three more failed only on the missing state column; settled-rows behaviour already held; after, 6 of 6 pass
  - command: begin guard scan of commerce/effects.rs at HEAD and in the tree (Python re-implementation of the guard window, not a cargo run)
    result: PASS
    evidence: HEAD 1 unbudgeted begin, tree 0; cargo guard test passes with the exempt entry removed
  - command: cargo test --locked -p oteryn-admin -- --nocapture (PostgreSQL 18)
    result: PASS
    evidence: 189 passed, 0 failed, no Skipping or SKIPPED line in the captured output
  - command: migration backfill on a scratch schema (t50 table, one row with and one without provider_reference, then the new migration)
    result: PASS
    evidence: with reference reads done, without reads pending
  - command: cargo test --locked -p oteryn-db --test migrations; cargo check --locked -p oteryn-portal; cargo fmt --all -- --check; cargo clippy --locked -p oteryn-admin --all-targets -- -D warnings
    result: PASS
    evidence: migrations 3 passed; worker binary compiles (Send bound on RefundDelivery::run holds); fmt and clippy exit 0
  - command: cargo test --locked -p oteryn-admin --lib postgres_refund (PostgreSQL 18), with the new fence and exhaustion tests
    result: PASS
    evidence: 9 passed; with the attempt-id clauses removed from record_dispatch and release_refund, both stale-attempt tests fail (red) and the stale re-claim test still passes
blockers:
  - none
next_action: push after PR 1921 merges and main is merged in, then open the PR
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Task branch for the refund claim PR; retained until merge
source_branch_evidence: The PR records the published head
```
