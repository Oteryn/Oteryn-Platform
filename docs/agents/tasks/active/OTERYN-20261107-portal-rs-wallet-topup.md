---
task_id: OTERYN-20261107-portal-rs-wallet-topup
governing_issue: 530
required_reads:
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/contracts/OTERYN_V2_ADMIN_ANALYTICS_CONTRACT_PROPOSAL.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/GOVERNANCE_CONTRACT.json
# Governing AGENTS/bootstrap instructions are loaded by the bounded bootstrap.
# Add only task-routed architecture, contract, security, programme, validation or handoff files not already required by higher-priority instructions.
search_first:
  - governing Issue 530, open PRs and active economy task
  - payments/store checkout, ledger and worker registration
  - admin analytics economy source gap
optional_reads: []
---

# OTERYN-20261107-portal-rs-wallet-topup

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Deliver checkout-bound, idempotent paid top-up coin credits and aggregate Platform purchased analytics. No deploy, PR, Game writes or real payment credentials.

## Acceptance criteria

- [x] Part A: atomic classified credit, exact checkout/intent/job match, eligibility/flags, durable audit and retry behavior.
- [x] Part B: Platform-only UTC daily purchased counts, coins and paid minor units by currency.
- [x] Requested PostgreSQL 18.6, Rust, migration, browser and ratchet validation; push task branch without PR.

## Ownership

```yaml
owned_paths:
  - portal-rs/bin/oteryn-portal/src/wallet_topup*
  - portal-rs/bin/oteryn-portal/src/worker*
  - portal-rs/crates/wallet/**
  - portal-rs/crates/admin/src/analytics/**
  - portal-rs/migrations/20261107180001_wallet_topup.*
  - portal-rs/e2e/**
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/Cargo.toml
  - portal-rs/Cargo.lock
  - portal-rs/crates/payments/src/core.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/templates/admin/analytics_economy.html
  - docs/agents/tasks/active/OTERYN-20261107-portal-rs-wallet-topup.md
modules:
  - payments/wallet/store/admin analytics
dependencies:
  - codex/portal-rs-admin-analytics-economy merged at 1c5193c40769ab1467e4244d557579881463a783
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

`checkpoint_version` remains structural version 1. Policy revision 4 preserves revision 2's accepted statuses `waiting` and `completed` plus validation result `NOT_APPLICABLE`, revision 3's live PR/branch liveness, and adds fail-closed live governing-Issue liveness without invalidating existing checkpoint structure. Validate the completed checkpoint with `python tools/agents/checkpoint.py <task-path> --require-checkpoint`.

The checkpoint is a durable context/evidence/ownership/handoff mirror. Its lifecycle and PR fields must be reconciled to the governing live GitHub Issue and live PR when those are newer; it is not an independent competing mutable lifecycle authority. The active packet must leave `docs/agents/tasks/active/` when its governing Issue becomes terminal.

`ROTATE` is a terminal invocation result, never a task status. Before returning `ROTATE`, persist `ready`, `waiting` or `blocked` with one concrete `next_action`.

```yaml
checkpoint_version: 1
updated_at: 2026-11-07T18:21:48Z
head: 7b342713e7ff24bd3968d678d5d5b8b6c2c5113b
branch: claude/portal-rs-wallet-topup
pr: none
status: ready
context_routes:
  - payments
  - database
  - testing
owned_paths:
  - portal-rs/bin/oteryn-portal/src/wallet_topup*
  - portal-rs/bin/oteryn-portal/src/worker*
  - portal-rs/crates/wallet/**
  - portal-rs/crates/admin/src/analytics/**
  - portal-rs/migrations/20261107180001_wallet_topup.*
  - portal-rs/e2e/**
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/Cargo.toml
  - portal-rs/Cargo.lock
  - portal-rs/crates/payments/src/core.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/templates/admin/analytics_economy.html
  - docs/agents/tasks/active/OTERYN-20261107-portal-rs-wallet-topup.md
proven:
  - checkout package coins are credited once for the paid intent; mismatches and ineligible accounts deny
  - Platform purchased totals preserve independent coin and payment-currency units
  - browser top-up and admin analytics flows pass in both locales and viewports
  - empty rollback/reapply and populated rollback refusal pass on PostgreSQL 18.6
derived: []
unknown: []
conflicts: []
first_failure:
  marker: TOPUP-CREDIT-MISSING
  evidence: red PostgreSQL webhook/outbox/worker regression returned 0 coins instead of 1234
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261107-portal-rs-wallet-topup.md
  - portal-rs/Cargo.lock
  - portal-rs/bin/oteryn-portal/Cargo.toml
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/wallet_topup.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/admin/src/analytics/README.md
  - portal-rs/crates/admin/src/analytics/economy.rs
  - portal-rs/crates/admin/src/analytics/mod.rs
  - portal-rs/crates/admin/src/analytics/purchased.rs
  - portal-rs/crates/admin/src/analytics/tests.rs
  - portal-rs/crates/payments/src/core.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/wallet/README.md
  - portal-rs/crates/wallet/src/domain.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/e2e/README.md
  - portal-rs/e2e/local.mjs
  - portal-rs/e2e/specs/admin-analytics.spec.mjs
  - portal-rs/e2e/specs/wallet-topup.spec.mjs
  - portal-rs/migrations/20261107180001_wallet_topup.down.sql
  - portal-rs/migrations/20261107180001_wallet_topup.up.sql
  - portal-rs/templates/admin/analytics_economy.html
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: exit 0
  - command: cargo clippy --locked --workspace --all-targets -- -D warnings
    result: PASS
    evidence: exit 0, all workspace targets
  - command: cargo test --locked --no-fail-fast
    result: PASS
    evidence: exit 0; 1143 passed, 0 failed, 3 optional browser tests ignored by default; PostgreSQL 18.6, max_locks_per_transaction=1024, max_connections=300; RUST_TEST_THREADS=1
  - command: cargo test --locked -p oteryn-web postgres_browser_search_keyboard_and_accessibility -- --ignored
    result: PASS
    evidence: exit 0; 1 passed; Chromium, PL/EN, light/dark, 360/1280px, axe zero violations
  - command: cargo test --locked -p oteryn-payments -p oteryn-store -- --ignored
    result: PASS
    evidence: exit 0; remaining 2 optional browser tests passed explicitly; all three default-ignored cases covered
  - command: node run.mjs /tmp/oteryn-wallet-topup-e2e/env.json test specs/wallet-topup.spec.mjs
    result: PASS
    evidence: 4 passed, Polish/English at desktop/mobile, 46.9 seconds
  - command: node run.mjs /tmp/oteryn-wallet-topup-e2e/env.json test specs/admin-analytics.spec.mjs
    result: PASS
    evidence: 4 passed, Polish/English at desktop/mobile, 1 minute
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: exit 0, historical-token counts remain report-only
  - command: migration rollback and reapply
    result: PASS
    evidence: PostgreSQL focused test undoes/reapplies the empty migration and refuses populated top_up rollback
blockers:
  - none
next_action: Owner review of the retained task branch; no PR or deployment is authorized.
```

## Source branch closeout

Every task branch must have an intentional terminal disposition. This section is closeout evidence and does not add fields to the shared Context checkpoint contract.

```yaml
source_branch_disposition: retain
source_branch_reason: task requester owns the retained review branch; revisit on their next review or integration instruction; user requests no PR
source_branch_evidence: owner explicitly requests push to codex/portal-rs-wallet-topup without a PR
```

- Use `auto_delete_after_merge` for the ordinary same-repository PR path and verify the source ref disappears after merge.
- Before intentionally closing a same-repository PR without merge, put exactly one `Branch-Disposition: delete` or `Branch-Disposition: retain` marker in the PR body and add a non-empty `Branch-Disposition-Reason: ...`.
- `delete_on_close` requires the trusted-main terminal branch lifecycle to prove exact-head terminal state and verify deletion.
- `retain` requires a concrete owner, purpose and review trigger; unexplained retained refs are not terminal.
- Never merge abandoned, superseded, diagnostic, temporary, backup or recovery-only work merely to trigger automatic branch deletion.

## Notes

Keep this section concise. Durable continuation state belongs in the checkpoint above. Do not paste secrets, full logs, or full diffs. Structural checkpoint validation does not replace live GitHub Issue, Git, PR, CI or source-branch closeout verification.

## Admission evidence

Latest main: bba17b50d29df70a0be4116b6bc55d15c86d81a3. Governing Issue #530 live OPEN. No overlapping open PR; economy branch explicitly requested as dependency. `portal-rs/AGENTS.md` is absent from main and dependency branch. Root guidance and immutable META binding resolved. Single writer; no delegation. Jira mapping unknown, sync pending.

Threat assumptions: only authenticated payment success may enqueue; checkout package fixes conversion. Consumer independently binds account/amount/currency/state, locks authoritative eligibility, and uses ledger claim serialization. Missing checkout purpose denies value. No real provider or production activation authorized.

## Files by acceptance item

All paths below are relative to `portal-rs/` unless stated otherwise.

| Item | Delivered files / evidence |
| --- | --- |
| A1 credit, eligibility, flags, audit, replay | `bin/oteryn-portal/src/wallet_topup.rs`, `worker.rs`, `main.rs`; `bin/oteryn-portal/Cargo.toml`, `Cargo.lock`; `crates/wallet/src/domain.rs`, `ledger.rs`; `crates/payments/src/core.rs` |
| A2 classified reversible schema | `migrations/20261107180001_wallet_topup.up.sql`, `.down.sql`; `crates/wallet/README.md`. Entry kind is inherited from its operation; entries have no separate kind CHECK. |
| A3 other payment purpose refuses credit | `wallet_topup.rs` requires the matching checkout row; `worker_tests.rs` covers absent checkout/intent, exact three-way matches and refusal audit/dead-letter. |
| A4 tests and test-provider browser path | `bin/oteryn-portal/src/worker_tests.rs`; `crates/store/src/web_payments.rs`; `e2e/local.mjs`, `specs/wallet-topup.spec.mjs`, `README.md`. Existing commerce refund implementation remains unchanged. |
| B5 Platform purchased | `crates/admin/src/analytics/purchased.rs`, `economy.rs`, `mod.rs`, `tests.rs`, `README.md`; `templates/admin/analytics_economy.html`; `e2e/specs/admin-analytics.spec.mjs`. Dependency branch merge retains the revision-3 consumer/wire fixtures and economy migration. |
| Governance | `docs/agents/tasks/active/OTERYN-20261107-portal-rs-wallet-topup.md` from the repository template, governing Issue 530. |

## Validation notes

The initial red regression used a refusing handler stub and proved the defect:
0 coins credited instead of the checkout's 1,234 coins. The subsequent green
webhook/outbox/worker test passed. It now also delivers two distinct jobs for the
same intent concurrently on first credit and replays after acknowledgement;
there is one ledger entry and one credit audit. The purchased aggregate still
counts one credit. Negative cases independently mismatch checkout, intent and
job values; missing/closed/suspended/unverified accounts, flags off, non-settled
intents and credit-audit outage all fail closed. Refusals retain audit and use the
existing outbox finish path.

The local browser launcher owns only its disposable databases, certificates and
synthetic producer processes. Its serve/worker were stopped after browser proof
before the full Rust suite to release the composed runtime ports. The preexisting
local PostgreSQL container/data is preserved. Two task-owned failed-test schemas
from red/refusal experimentation were verified by their synthetic checkout and
removed. Final cleanup verified both launcher-owned databases and all launcher
processes gone. Browser scratch (including ephemeral keys/certificates) was
removed. With zero remaining client backends, the shared PostgreSQL instance was
returned to its original effective 64/100 settings and stopped, preserving its
existing data. The required 1024/300 settings applied throughout validation.

Feature defaults remain off. No workflows, deploy, PR, Game repository reads or
writes, real provider credentials, or production operations are part of delivery.

## Final evidence

All requested validation completed without failures. The 3 browser tests ignored
by the default full Cargo run were each executed explicitly and passed. The full
suite includes the existing `commerce::tests::postgres_payments_review_and_refund_queue_without_forged_settlement`
regression and the new top-up/audit/migration/aggregate tests. No implementation
or validation gap remains for this task; real providers and production
qualification are outside the requested scope.

Tested Portal material index digest (`git ls-files -s -- portal-rs | sha256sum`):
`d59d92845c3d25a407962a81588fbc4ab0c23014a7cf7ff50b416da553c18972`.
The checkpoint head is the admitted main/economy merge predecessor; the final
publication adds this bounded tested delta. Local safe command logs are retained
under `/tmp/topup-*.log`; no browser authentication artifacts or keys are published.
