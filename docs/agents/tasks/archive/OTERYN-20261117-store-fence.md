---
task_id: OTERYN-20261117-store-fence
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-store-fence

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, class B, store crate. A crate guard requires every production
`begin()` in `crates/store/src` to bound its lock waits within four lines or
be listed with its reason. Its first run listed 17 unbounded sites. Ten now arm
a budget right after `begin`: `oteryn_db::mutation_budget` for the profile B
authority, the loyalty snapshot, the loyalty stream, staff code issuance, both
code redemptions and catalog import and activation; the read budget
(`lock_timeout` 1 s, `statement_timeout` 2.5 s) for the two value-eligibility
reads (shop backend and the payments account adapter compiled into the binary
as `shop_web`). Seven are listed: two plain reads with no row or advisory lock,
`Codes::generate` (no production caller), and four that begin through the
crate's `Delivery::begin` and `Purchases::begin` wrappers, which already set
`lock_timeout` 3 s and `statement_timeout` 5 s.

## Acceptance criteria

- [x] Crate guard: every begin() is budgeted or listed with a reason (red without the change)
- [x] Each newly budgeted site has a PostgreSQL test where a held lock makes the call fail unavailable within 4 s (red without the change, except the two redemptions whose budget `redeem_in` already armed)
- [x] Portal RS CI green on the PR head

## Follow-up (not this task)

The owning code is `crates/admin/src/commerce/effects.rs`, which open PR 1913 touches.
deliver_refund (outbox job commerce.refund) must not hold FOR UPDATE across provider.refund():
(1) a short transaction claims the row (pending→refunding plus an attempt id) and commits;
(2) provider.refund() runs outside any transaction, with idempotency key = refund id;
(3) a short finalize transaction moves refunding→done/failed, conditional on the attempt id;
(4) recovery re-claims rows stuck in 'refunding' older than N, which is safe because of the idempotency key.
No timeout on the external call until then.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-store-fence.md
  - portal-rs/crates/store/src/begin_guard_tests.rs
  - portal-rs/crates/store/src/lib.rs
  - portal-rs/crates/store/src/catalog.rs
  - portal-rs/crates/store/src/catalog_tests.rs
  - portal-rs/crates/store/src/codes.rs
  - portal-rs/crates/store/src/codes_tests.rs
  - portal-rs/crates/store/src/loyalty.rs
  - portal-rs/crates/store/src/loyalty_tests.rs
  - portal-rs/crates/store/src/loyalty_stream.rs
  - portal-rs/crates/store/src/loyalty_stream_tests.rs
  - portal-rs/crates/store/src/premium_profile_b.rs
  - portal-rs/crates/store/src/premium_profile_b_tests.rs
  - portal-rs/crates/store/src/web_backend.rs
  - portal-rs/crates/store/src/web_tests.rs
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
updated_at: 2026-11-17T12:00:00Z
head: eb881d55af6fdc010ec520afd48871f15ac60fab
branch: claude/happy-davinci-ijz3wy-store-fence
pr: 1917
status: completed
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-store-fence.md
  - portal-rs/crates/store/src/begin_guard_tests.rs
  - portal-rs/crates/store/src/lib.rs
  - portal-rs/crates/store/src/catalog.rs
  - portal-rs/crates/store/src/catalog_tests.rs
  - portal-rs/crates/store/src/codes.rs
  - portal-rs/crates/store/src/codes_tests.rs
  - portal-rs/crates/store/src/loyalty.rs
  - portal-rs/crates/store/src/loyalty_tests.rs
  - portal-rs/crates/store/src/loyalty_stream.rs
  - portal-rs/crates/store/src/loyalty_stream_tests.rs
  - portal-rs/crates/store/src/premium_profile_b.rs
  - portal-rs/crates/store/src/premium_profile_b_tests.rs
  - portal-rs/crates/store/src/web_backend.rs
  - portal-rs/crates/store/src/web_tests.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/store/src/web_payments_tests.rs
proven:
  - No migration, contract, fixture, flag or wire byte change; no lock order changed
  - Every new budget error (55P03, 57014) maps to the module's existing Unavailable, which the routes answer 503; the catalog CLI exits 2; every error drops the transaction, so nothing commits a partial effect
  - No store transaction spans a provider or other external call
  - Codes::generate has no production caller (bin/oteryn-portal/src/code_routes.rs issues through generate_staff)
  - Staff code issuance and catalog import/activation were already budgeted in production by their binary adapters (rbac authorize_mutation, rbac lock_accounts_and_mutation); the explicit arm makes the store transaction independent of the adapter
derived:
  - The loyalty snapshot's first wait is in referral_campaign_prepare, which rewrites 55P03 into 40001 and so into a retry; with the mutation budget statement_timeout (57014, not retried) fires first, observed under 4 s
  - statement_timeout now caps every statement in the ten budgeted transactions, a new unavailable path independent of locks
  - mutation_budget armed once at begin sets lock_timeout and statement_timeout to about 3 s for each later statement; only a helper that re-arms it shrinks them to the time left, so a single arm bounds each wait, not the whole transaction
  - The payments adapter maps an Err from Accounts::state to AccountDenied (crates/payments/src/storage.rs eligible), a fail-closed refusal rather than 503; payments is outside this task
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-store-fence.md
  - portal-rs/crates/store/src/begin_guard_tests.rs
  - portal-rs/crates/store/src/lib.rs
  - portal-rs/crates/store/src/catalog.rs
  - portal-rs/crates/store/src/catalog_tests.rs
  - portal-rs/crates/store/src/codes.rs
  - portal-rs/crates/store/src/codes_tests.rs
  - portal-rs/crates/store/src/loyalty.rs
  - portal-rs/crates/store/src/loyalty_tests.rs
  - portal-rs/crates/store/src/loyalty_stream.rs
  - portal-rs/crates/store/src/loyalty_stream_tests.rs
  - portal-rs/crates/store/src/premium_profile_b.rs
  - portal-rs/crates/store/src/premium_profile_b_tests.rs
  - portal-rs/crates/store/src/web_backend.rs
  - portal-rs/crates/store/src/web_tests.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/store/src/web_payments_tests.rs
validation:
  - command: cargo test --locked -p oteryn-store --lib -- sheds_within begin_guard (PostgreSQL 18), without the source changes
    result: PASS
    evidence: red as intended; the guard lists 17 sites in 10 files and seven new store tests time out at 10 s; the redemption test (redeem_in armed the budget on main) passes with codes.rs at origin/main
  - command: cargo test --locked -p oteryn-portal --bin oteryn-portal -- postgres_payment_account_state (PostgreSQL 18), without and with the web_payments change
    result: PASS
    evidence: without it the test times out at 10 s; with it shop_web 9 passed, 1 ignored (pre-existing browser test)
  - command: cargo test --locked -p oteryn-store (PostgreSQL 18)
    result: PASS
    evidence: lib 131 passed, 0 failed, 1 ignored (pre-existing postgres_browser_at_360px); no SKIPPED line
  - command: cargo test --locked -p oteryn-community --lib -- referrals:: (PostgreSQL 18)
    result: PASS
    evidence: 57 passed; these drive the loyalty snapshot and stream under held locks
  - command: cargo fmt --all -- --check; cargo clippy --locked -p oteryn-store --all-targets -- -D warnings; cargo clippy --locked -p oteryn-portal --all-targets -- -D warnings
    result: PASS
    evidence: all exit 0
blockers:
  - none
next_action: none; delivered by PR 1917 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1917 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1917 merged as eb881d55af6fdc010ec520afd48871f15ac60fab
```
