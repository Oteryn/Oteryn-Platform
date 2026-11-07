---
task_id: OTERYN-20261107-portal-rs-admin-analytics-economy
governing_issue: 530
branch: codex/portal-rs-admin-analytics-economy
required_reads:
  - docs/contracts/OTERYN_V2_ADMIN_ANALYTICS_CONTRACT_PROPOSAL.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/GOVERNANCE_CONTRACT.json
# Governing AGENTS/bootstrap instructions are loaded by the bounded bootstrap.
# Add only task-routed architecture, contract, security, programme, validation or handoff files not already required by higher-priority instructions.
search_first: []
optional_reads: []
---

# OTERYN-20261107-portal-rs-admin-analytics-economy

## Goal

Governing GitHub Issue: 530 — https://github.com/Oteryn/Oteryn-Platform/issues/530 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Extend the existing consumer to accepted contract revision 3, economy only; push the owner-selected branch without a PR or deployment.

## Acceptance criteria

- [x] Five economy members and all §11.2 validation rules.
- [x] Economy low_n values and rev-2 suppression remain separate.
- [x] Existing fence/epoch/idempotency/watermark paths preserved.
- [x] New reversible aggregate-only migration pair; no wallet schema changes.
- [ ] Platform purchased aggregation and empty-day qualification: missing purchase-credit classification/currency mapping.
- [x] One guarded economy page, independent source gaps, no account search/export.
- [x] Existing switches remain default off.
- [x] Four canonical §11.6 examples with SHA-256 assertions.
- [x] Required independent validation; candidate prepared for the owner-requested branch publication.
- [ ] Full task acceptance remains blocked on scope item 5.

## Ownership

```yaml
owned_paths:
  - portal-rs/crates/admin/src/analytics/**
  - portal-rs/crates/admin/tests/fixtures/admin_analytics/economy/**
  - portal-rs/migrations/20261107170001_admin_analytics_economy.*.sql
  - portal-rs/templates/admin/analytics*.html
  - portal-rs/e2e/specs/admin-analytics.spec.mjs
  - docs/agents/tasks/active/OTERYN-20261107-portal-rs-admin-analytics-economy.md
modules:
  - admin analytics
dependencies:
  - accepted analytics contract revision 3
blockers:
  - qualified wallet purchase-credit classification, currency mapping and capture coverage are missing
cross_repository_tasks:
  - none
```

## Context checkpoint

`checkpoint_version` remains structural version 1. Policy revision 4 preserves revision 2's accepted statuses `waiting` and `completed` plus validation result `NOT_APPLICABLE`, revision 3's live PR/branch liveness, and adds fail-closed live governing-Issue liveness without invalidating existing checkpoint structure. Validate the completed checkpoint with `python tools/agents/checkpoint.py <task-path> --require-checkpoint`.

The checkpoint is a durable context/evidence/ownership/handoff mirror. Its lifecycle and PR fields must be reconciled to the governing live GitHub Issue and live PR when those are newer; it is not an independent competing mutable lifecycle authority. The active packet must leave `docs/agents/tasks/active/` when its governing Issue becomes terminal.

`ROTATE` is a terminal invocation result, never a task status. Before returning `ROTATE`, persist `ready`, `waiting` or `blocked` with one concrete `next_action`.

```yaml
checkpoint_version: 1
updated_at: 2026-11-07T17:42:27Z
head: c25414d8ff0fbf20a1f6059265c62bb7f9501353
branch: codex/portal-rs-admin-analytics-economy
pr: none
status: blocked
context_routes:
  - accepted analytics contract §§5,7,10,11,12
owned_paths:
  - portal-rs/crates/admin/src/analytics/**
  - portal-rs/crates/admin/tests/fixtures/admin_analytics/economy/**
  - portal-rs/migrations/20261107170001_admin_analytics_economy.*.sql
  - portal-rs/templates/admin/analytics*.html
  - portal-rs/e2e/specs/admin-analytics.spec.mjs
  - docs/agents/tasks/active/OTERYN-20261107-portal-rs-admin-analytics-economy.md
proven:
  - all five economy members ingest and render through existing security and fence boundaries
  - 15 focused analytics tests pass including actual migration rollback/reapply
  - final full suite passes 1140 tests with 3 existing optional Chromium tests ignored
  - final browser qualification passes all 4 projects on the final material tree
  - formatting Clippy legacy token ratchet and diff checks pass
derived: []
unknown:
  - Wallet purchase-credit classification and currency are absent in the base ledger.
conflicts: []
first_failure:
  marker: wallet-purchased-source-gap
  evidence: wallet operation kinds lack a real-money purchase credit and currency; payments TopUpJob documents a future handler
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261107-portal-rs-admin-analytics-economy.md
  - portal-rs/crates/admin/src/analytics/README.md
  - portal-rs/crates/admin/src/analytics/economy.rs
  - portal-rs/crates/admin/src/analytics/tests.rs
  - portal-rs/crates/admin/src/analytics/web.rs
  - portal-rs/crates/admin/src/analytics/wire.rs
  - portal-rs/crates/admin/tests/fixtures/admin_analytics/economy/circulation.json
  - portal-rs/crates/admin/tests/fixtures/admin_analytics/economy/commit-market.json
  - portal-rs/crates/admin/tests/fixtures/admin_analytics/economy/market.json
  - portal-rs/crates/admin/tests/fixtures/admin_analytics/economy/watermark-market.json
  - portal-rs/e2e/specs/admin-analytics.spec.mjs
  - portal-rs/migrations/20261107170001_admin_analytics_economy.down.sql
  - portal-rs/migrations/20261107170001_admin_analytics_economy.up.sql
  - portal-rs/templates/admin/analytics.html
  - portal-rs/templates/admin/analytics_economy.html
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: exit 0 with pinned Rust 1.97.0
  - command: cargo clippy --locked --workspace --all-targets -- -D warnings
    result: PASS
    evidence: exit 0 with pinned Rust 1.97.0; final run 11.72 seconds
  - command: cargo test --locked --no-fail-fast
    result: PASS
    evidence: exit 0; 1140 passed, 0 failed, 3 existing optional Chromium tests ignored, 85 targets; PostgreSQL 18.6, max_locks_per_transaction=1024, max_connections=300, RUST_TEST_THREADS=4; isolated fresh task database
  - command: cargo test --locked -p oteryn-admin analytics::tests
    result: PASS
    evidence: exit 0; 15 tests including postgres_economy_migration_rollback_reapply and fence/retention/release/panel/security coverage
  - command: node run.mjs /tmp/portal-economy-e2e-final/env.json test specs/admin-analytics.spec.mjs
    result: PASS
    evidence: exit 0; 4 projects passed, Polish/English at 1440px/390px, 88.6 seconds; real staff registration/login/MFA, unavailable and populated economy states; private scratch removed afterward
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: exit 0; no baseline or workflow changes
  - command: wallet purchased aggregation and empty-day test
    result: BLOCKED
    evidence: no qualified purchase-credit classification, coin currency mapping or capture coverage exists at this base; clarification pending
blockers:
  - qualified wallet purchase-credit classification, currency mapping and capture coverage are missing
next_action: Obtain the qualified wallet purchase-credit classification, currency mapping and capture coverage, then implement the read-only purchased aggregate and its empty-day test.
```

## Source branch closeout

Every task branch must have an intentional terminal disposition. This section is closeout evidence and does not add fields to the shared Context checkpoint contract.

```yaml
source_branch_disposition: retain
source_branch_reason: owner requested this branch; review resumes when the qualified wallet purchase mapping is supplied
source_branch_evidence: explicit owner instruction to push this branch without a PR; exact publication head is recorded in the turn closeout
```

- Use `auto_delete_after_merge` for the ordinary same-repository PR path and verify the source ref disappears after merge.
- Before intentionally closing a same-repository PR without merge, put exactly one `Branch-Disposition: delete` or `Branch-Disposition: retain` marker in the PR body and add a non-empty `Branch-Disposition-Reason: ...`.
- `delete_on_close` requires the trusted-main terminal branch lifecycle to prove exact-head terminal state and verify deletion.
- `retain` requires a concrete owner, purpose and review trigger; unexplained retained refs are not terminal.
- Never merge abandoned, superseded, diagnostic, temporary, backup or recovery-only work merely to trigger automatic branch deletion.

## Notes

Keep this section concise. Durable continuation state belongs in the checkpoint above. Do not paste secrets, full logs, or full diffs. Structural checkpoint validation does not replace live GitHub Issue, Git, PR, CI or source-branch closeout verification.

## Implementation boundary

Base is the exact owner-selected c25414d8ff0fbf20a1f6059265c62bb7f9501353. Live Issue #530 was OPEN; the only open PR was unrelated #1643. The rev-2 packet describes merged work, not a concurrent writer. Main has advanced; no rebase or unrelated main changes were taken. `portal-rs/AGENTS.md` does not exist at this base. Root AGENTS/bootstrap and bound META organization policy were read. Jira synchronization is unknown/pending; no connector was loaded.

Wallet inspection proves no purchase-credit classification or currency field exists. `store_purchase` is a negative debit, not a real-money credit; `TopUpJob` documents a future handler. A clarification was requested during work. Until a qualified mapping/capture exists, purchased is explicitly incomplete, even on an empty day. No payment intent, operator adjustment or browser return is counted. Scope item 5 remains incomplete; this candidate must not be described as full task acceptance.

No Game repository was accessed, no workflow was edited, and no PR/deployment was created. The resync/activation blocker from rev 2 remains intact. Rollback preserves fences and rev-2 rows; the down migration refuses low_n-bearing payloads rather than erasing or silently relabelling them. PostgreSQL tests prove that refusal is transactional.

## Validation recovery and closeout evidence

The contract byte-vector test was recorded red against the unchanged consumer (circulation rejected), then green after the additive parser. The initial full run exited 101 (1135 passed, 5 failed, 3 ignored): one analytics retention test still used the generic rev-2 route for an economy selector; four Bazaar migration tests found existing wallet tables in the supplied pre-existing test database. The selector was corrected and the final full run used a fresh task-only database; all five now pass without changes to Bazaar or Wallet. The final task database is removed after the suite.

A first Clippy invocation from the repository root selected Rust 1.99 and reported new lints in unchanged Identity/GameAuth source. Required validation was rerun inside portal-rs with its pinned Rust 1.97. One unnecessary clone in a new test was fixed. The first browser run found the old single-element unavailable locator now matching five economy groups; the assertion was corrected to require all five, and the final four-project matrix passed against the final material binary. No unrelated source was changed to address those setup/test issues.

All 15 changed files were reviewed, including the migration inverse and the four exact fixture bodies. Neither the existing store transaction implementation nor wallet schema/ledger code changed. Ingestion and presentation remain off by default, baseline qualification remains pending, export has no route, and scope item 5 is explicitly not claimed complete.
