---
task_id: OTERYN-20261115-audit-a2-l6
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit wave 2 L6 repairs

Governing workstream: https://github.com/Oteryn/Oteryn-Platform/issues/536,
verified open 2026-11-15. The owner's task specifies L6-01 through L6-05,
base c6fdc5438a4afa91f32d547b7cfc61f22bf21139 and branch
claude/happy-davinci-ijz3wy-fix-a2-l6. Single writer, no subagents, no PR or merge.
Earlier Bazaar audit ownership is incorporated in this integration base.

All five findings were confirmed and fixed. New value uses a shared SQL
eligibility predicate; existing reservation returns and browser refunds retain
separate identity rules. Delivered browser refunds are refused until reversal
handling exists. Minigame purchases use distinct wallet kinds and Store OTHER.
Only local PostgreSQL and deterministic test providers were used. Existing
feature flags, contract bytes and workflows are unchanged.

Source branch disposition: retain for coordinator review of the requested fix branch.
Jira synchronization remains with the coordinator.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T18:28:49Z
head: c6fdc5438a4afa91f32d547b7cfc61f22bf21139
branch: claude/happy-davinci-ijz3wy-fix-a2-l6
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/bin/oteryn-portal/
  - portal-rs/crates/wallet/
  - portal-rs/crates/store/
  - portal-rs/crates/minigames/
  - portal-rs/crates/community/
  - portal-rs/crates/identity/
  - portal-rs/crates/characters/
  - portal-rs/crates/bazaar/
  - portal-rs/crates/payments/
  - portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql
  - portal-rs/Cargo.lock
  - docs/agents/tasks/active/OTERYN-20261115-audit-a2-l6.md
proven:
  - All five findings confirmed against the requested integration base and fixed
  - Nine PostgreSQL regressions fail against the original implementations
  - Final crate suites total 934 passed, zero failed and three pre-existing ignored browser checks
  - All audit PostgreSQL regressions ran with durations above 0.05 seconds
  - Rust 1.97 formatting and Clippy with warnings denied pass for all touched crates and the database crate
  - Existing tools were reused without installation
  - Repository identity is Oteryn/Oteryn-Platform and governing Issue 536 remains open
  - Existing feature flags, contract bytes and workflows are unchanged
derived: []
unknown:
  - Coordinator review and Jira synchronization
conflicts: []
first_failure:
  marker: expected_before_fix_regressions
  evidence: Nine expected baseline failures; fixture-only failures and local PostgreSQL lock pressure resolved before final qualification
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261115-audit-a2-l6.md
  - portal-rs/Cargo.lock
  - portal-rs/bin/oteryn-portal/src/coin_purchases.rs
  - portal-rs/bin/oteryn-portal/src/wallet_topup.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/coin_ports.rs
  - portal-rs/crates/characters/Cargo.toml
  - portal-rs/crates/characters/src/services/mod.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/minigames/src/collection_pvp/mod.rs
  - portal-rs/crates/minigames/src/collection_pvp/tests.rs
  - portal-rs/crates/minigames/src/pass/mod.rs
  - portal-rs/crates/payments/README.md
  - portal-rs/crates/payments/src/core.rs
  - portal-rs/crates/payments/src/storage.rs
  - portal-rs/crates/payments/tests/payments.rs
  - portal-rs/crates/store/Cargo.toml
  - portal-rs/crates/store/src/purchase_accounts.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/store/src/web_backend.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/store/src/web_payments_tests.rs
  - portal-rs/crates/wallet/src/domain.rs
  - portal-rs/crates/wallet/src/eligibility.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/wallet/src/lib.rs
  - portal-rs/crates/wallet/src/wire.rs
  - portal-rs/crates/wallet/tests/coin_http/mod.rs
  - portal-rs/crates/wallet/tests/postgres.rs
  - portal-rs/migrations/20261115120017_fix_a2-l6_value_eligibility.sql
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: installed Rust 1.97 repository pin
  - command: cargo clippy --locked --all-targets for all nine touched crates plus oteryn-db -- -D warnings
    result: PASS
    evidence: full check followed by final portal check after the test-only assertion correction
  - command: cargo test --locked for all nine touched crates plus oteryn-db
    result: PASS
    evidence: latest complete per-crate suites; final minigames/payments/portal rerun and final complete portal suite; 934 passed, zero failed, three existing browser ignores
  - command: PostgreSQL regression red/green verification
    result: PASS
    evidence: original implementation failures retained in local logs; final measured PostgreSQL durations recorded below
blockers:
  - none
next_action: Coordinator reviews the requested fix branch after publication
```

Partial account fixtures now use real migrations. Their duplicate session DDL
was removed; the old unknown-status fixture now checks both service refusal of
suspended accounts and database refusal of the unknown status. No assertions
were removed. Validation batches were serialized after concurrent schema cleanup
exhausted the local PostgreSQL lock table. Three unchanged browser checks retain
their existing optional Chromium/browser-module ignore attributes.

## PostgreSQL regression evidence

| Test | Duration (seconds) |
| --- | ---: |
| `l6_01_pending_termination_refuses_campaign_rewards` | 1.430 |
| `l6_01_pending_termination_refuses_store_purchase` | 1.724 |
| `l6_01_production_value_routes_restrict_sanctions_and_typed_bans` | 28.323 |
| `l6_01_refund_retains_identity_rules_while_acquisition_is_restricted` | 1.709 |
| `l6_02_production_value_replays_check_current_transfer_permission` | 5.304 |
| `l6_03_browser_checkout_refund_refused_twice_after_credit` | 1.816 |
| `l6_03_undelivered_browser_checkout_can_still_be_refunded` | 2.116 |
| `l6_04_rejected_paid_rename_releases_suspended_holder_once` | 1.491 |
| `l6_04_suspended_leader_can_be_outbid_and_exact_hold_released` | 2.257 |
| `l6_04_wallet_safe_release_allows_suspended_holder` | 1.799 |
| `l6_05_minigame_wallet_bridges_have_readable_store_history` | 1.686 |
| `paid_topup_webhook_outbox_worker_balance` | 2.030 |
| `postgres_delivery_refusal_allows_return_and_refunds_fenced_coins` | 0.344 |
