---
task_id: OTERYN-20261117-refund-reconcile
governing_issue: 1922
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-refund-reconcile

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1922.

Round 4, money path, admin commerce. After `REFUND_MAX_ATTEMPTS` the
`commerce.refund` outbox job is `dead` and its `commerce_refunds` row stays
unsettled (`pending`, or `refunding` under a stale claim).
`commerce::reconcile_dead_refunds` finds unsettled rows
(`provider_reference IS NULL`) whose job is dead and whose claim is not live
under the delivery path's own rule (`REFUND_CLAIM_STALE_AFTER`, unchanged),
locks row and job (`FOR UPDATE OF r,o`), resets that same job to `pending`
with `attempts=0` (same key and payload) and writes one
`commerce.refund.reconciled` audit row per revival, all in one budgeted
transaction. The worker then runs the existing claim, call, finalize path;
the provider idempotency key stays the intent id. The actor must be active
and hold `admin.access` and `payments.reconcile`. A row whose intent the
payment ledger already shows refunded is revived like any unsettled row and
is settled by `claim_refund` from the ledger without a provider call.
Operator command: `oteryn-portal commerce-refunds-reconcile [--dry-run]
--actor <uuid>`. No migration.

## Acceptance criteria

- [x] Dead job over a pending row is revived; delivery calls the provider with the intent id and settles with one dispatch audit
- [x] Dead job over a stale refunding row is revived and settles; a fresh claim is untouched
- [x] A settled row with a dead job is not revived or audited and the provider is not called
- [x] `--dry-run` changes nothing; a non-staff actor is refused
- [x] A second run revives nothing
- [x] The command parses `--dry-run`/`--actor` and refuses a missing `--actor`
- [ ] Portal RS CI green on the PR head (no PR yet)

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-refund-reconcile.md
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/commerce/reconcile.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/refund_reconcile_cli.rs
  - portal-rs/bin/oteryn-portal/src/refund_reconcile_cli_tests.rs
  - portal-rs/README.md
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
updated_at: 2026-11-17T14:00:00Z
head: 20d4f17d
branch: claude/happy-davinci-ijz3wy-refund-reconcile
pr: 1926
status: ready
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-refund-reconcile.md
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/commerce/reconcile.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/refund_reconcile_cli.rs
  - portal-rs/bin/oteryn-portal/src/refund_reconcile_cli_tests.rs
  - portal-rs/README.md
proven:
  - The revived job keeps its id, kind, idempotency key (intent id) and payload; the outbox claims it at attempt 1 and the delivery path settles the row (PostgreSQL 18 test)
  - The staleness rule is the exact negation of claim_refund's claim_live expression with the same REFUND_CLAIM_STALE_AFTER bind
  - No migration, contract, fixture, flag, workflow or payments change
derived:
  - Lock order row then job matches Refund apply (insert refund, then enqueue)
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Calling claim/call/finalize directly from the command would need the provider in the operator process and a second dispatch site
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-refund-reconcile.md
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/commerce/reconcile.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/refund_reconcile_cli.rs
  - portal-rs/bin/oteryn-portal/src/refund_reconcile_cli_tests.rs
  - portal-rs/README.md
validation:
  - command: cargo test -p oteryn-admin --lib refund_reconcile (PostgreSQL 18.7)
    result: PASS
    evidence: 5 passed; red without the function (E0425) and under mutations (no-op reconcile, settled filter, staleness clause, dead filter, dry-run rollback each turn the matching test red)
  - command: cargo test -p oteryn-admin (PostgreSQL 18.7)
    result: PASS
    evidence: 197 passed, 0 failed, no Skipping line
  - command: cargo test -p oteryn-portal --bin oteryn-portal refund_reconcile_cli
    result: PASS
    evidence: 1 passed; red when a missing --actor is accepted
  - command: cargo fmt --all --check; cargo clippy -p oteryn-admin -p oteryn-portal --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
blockers:
  - none
next_action: Portal RS CI on PR 1926
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Task branch for the refund reconcile work; retained until merge
source_branch_evidence: The branch records the published head
```
