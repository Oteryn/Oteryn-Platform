---
task_id: OTERYN-20261115-audit-a3-l6
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit wave 3 L6 repairs

Governing issue: https://github.com/Oteryn/Oteryn-Platform/issues/536 (open,
verified 2026-11-15). Owner requests a single agent, no subagents, and a push
only to claude/happy-davinci-ijz3wy-fix-a3-l6, with no PR or merge.
Base: 90234d8d2d0a51a654b8ff6947d38372a740acba, fetched from the specified
integration branch before implementation. All four findings confirmed.

Owned changes: Bazaar coin payout capacity, wallet credit capacity and safe
return preflight, commerce acquisition authorization, premium gift-code
composition and regression tests. Parallel RBAC-owned paths remain untouched.
Jira synchronization is pending coordinator handling.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T21:32:43Z
head: 90234d8d2d0a51a654b8ff6947d38372a740acba
branch: claude/happy-davinci-ijz3wy-fix-a3-l6
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/bin/oteryn-portal/src/code_routes.rs
  - portal-rs/bin/oteryn-portal/src/code_routes_tests.rs
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/crates/bazaar/src/game/coin_ports.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/store/src/codes_tests.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/wallet/
  - portal-rs/migrations/20261115225000_fix_a3-l6_incoming_capacity.sql
proven:
  - all findings confirmed against requested base
  - repository identity and open governing issue verified
unknown:
  - coordinator review and integration outcome
derived:
  - capacity reservations must cover the full seller net despite competing debt repayment
conflicts: []
changed_paths:
  - portal-rs/bin/oteryn-portal/src/code_routes.rs
  - portal-rs/bin/oteryn-portal/src/code_routes_tests.rs
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/coin_ports.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/store/src/codes_tests.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/wallet/src/lib.rs
  - portal-rs/crates/wallet/src/wire.rs
  - portal-rs/crates/wallet/tests/coin_http/mod.rs
  - portal-rs/migrations/20261115225000_fix_a3-l6_incoming_capacity.sql
  - docs/agents/tasks/active/OTERYN-20261115-audit-a3-l6.md
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Rust 1.97 pinned formatter
  - command: focused PostgreSQL a3_l6 regressions
    result: PASS
    evidence: all five executed; final durations 1.050 to 1.365 seconds
  - command: regressions with relevant base behavior restored
    result: PASS
    evidence: all five negative controls failed with the reported defects
  - command: cargo clippy --locked --all-targets -p each touched crate -- -D warnings
    result: PASS
    evidence: admin, bazaar, community, store, wallet and portal; final recheck passed
  - command: cargo test --locked -p each touched crate -- --test-threads=1 --nocapture
    result: PASS
    evidence: 794 passed, 0 failed, 2 pre-existing optional Chromium tests ignored; no PostgreSQL skip markers
blockers:
  - none
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
next_action: coordinator reviews the retained requested fix branch for integration
```

## Final validation evidence

All four findings fixed. Final PostgreSQL regressions:
- a3_l6_01_coin_payout_capacity_precedes_item_and_character_handoff: 1.154 s
- a3_l6_02_staff_routes_refuse_restricted_acquisitions: 1.050 s
- a3_l6_02_premium_code_route_refuses_restricted_acquisitions: 1.365 s
- a3_l6_04_cancellation_returns_suspended_holder_once: 1.308 s
- a3_l6_05_legacy_claim_without_qualification_denied: 1.079 s

Negative controls restored each relevant base behavior, retaining only test
support helpers: all five tests failed with the verified defect. Fixes were
restored before final checks. The final suites used a fresh task-owned local
PostgreSQL database and serial execution: the prepared database's default
schema contained untracked historical wallet tables, and parallel fixture
teardown exceeded the service's shared-lock budget. No existing database
contents or tests were changed to resolve these environment issues.

The HTTP regressions use the configured domain policies and real session,
CSRF/staff authorization or verified internal peer paths as applicable. Bazaar
payout, immutable receipt and compensation regressions also passed. Game
repository access, external Game execution, deployment and merge are outside
this task. Contract bytes and existing flags remain unchanged.

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner requested this branch for coordinator audit integration review
source_branch_evidence: explicit task instruction; no PR or merge requested
```
