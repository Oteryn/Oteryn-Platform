---
task_id: OTERYN-20261111-nwave-review-fixes
governing_issue: null
required_reads:
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/contracts/OTERYN_V2_REFERRAL_CLIENT_CONTRACT.md
  - docs/contracts/OTERYN_V2_SANCTION_EXPORT_CONTRACT.md
search_first:
  - portal-rs/crates/community/src/referrals
  - portal-rs/crates/store/src/loyalty_stream.rs
optional_reads: []
---

# OTERYN-20261111-nwave-review-fixes

## Goal

Fix the five owner-supplied N-wave review findings in existing PR #1800.
The live PR records the owner exception: no parent Issue (2026-11-11).
No Git writes, publication, integration, production or external repository work.

## Acceptance criteria

- [x] Regression coverage for loyalty credits, batch progress, signup/purchase ordering,
  sanction reactivation and session-end activity.
- [x] Smallest fixes; only the owner-listed unmerged migrations may change.
- [x] Owner-authorized offline fmt, per-crate clippy and filtered library tests green.

## Ownership

```yaml
owned_paths:
  - portal-rs/crates/store/src/loyalty*
  - portal-rs/crates/community/src/referrals/*
  - portal-rs/crates/support/src/sanctions/*
  - portal-rs/migrations/2026111114000[245]*
  - docs/contracts/OTERYN_V2_*LOYALTY*
  - docs/contracts/OTERYN_V2_REFERRAL_CLIENT_CONTRACT.md
  - docs/contracts/OTERYN_V2_SANCTION_EXPORT_CONTRACT.md
modules: [store, community, support]
dependencies: [PR-1800]
blockers: [none]
cross_repository_tasks: [none]
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-11T15:51:55Z
head: 4cfe5eb4dbaa6fa72f444ca49ef35f68c7d2915f
branch: pc-nwave
pr: 1800
status: waiting
context_routes: [database]
owned_paths: ["portal-rs/crates/store/src/loyalty*", "portal-rs/crates/community/src/referrals/*", "portal-rs/crates/support/src/sanctions/*", "portal-rs/migrations/2026111114000[245]*", "docs/contracts/*"]
proven:
  - Live PR 1800 is open/draft, Platform-only, with no parent Issue by owner decision.
  - Local HEAD matches live PR head; local branch pc-nwave is this existing dedicated worktree.
  - All five reported defects are present in the admitted source.
  - No overlapping N-wave packet found in targeted active-task search.
derived: []
unknown:
  - PostgreSQL runtime correctness remains unexecuted because PORTAL_TEST_DATABASE_URL is unset; owner explicitly permits this skip.
conflicts: []
first_failure:
  marker: regression-test-setup-compile
  evidence: Initial test compilation found an include path and SQLx dynamic-SQL annotation error; both corrected before fixes. Pre-fix regression runs then compiled but skipped database bodies; red runtime failures were not observed.
rejected_hypotheses: []
changed_paths: ["docs/agents/tasks/active/OTERYN-20261111-nwave-review-fixes.md", "docs/contracts/OTERYN_V2_LOYALTY_SNAPSHOT_CONTRACT.md", "docs/contracts/OTERYN_V2_REFERRAL_CLIENT_CONTRACT.md", "docs/contracts/OTERYN_V2_SANCTION_EXPORT_CONTRACT.md", "docs/contracts/fixtures/loyalty-stream-v1/credit-response.json", "portal-rs/crates/community/src/referrals/campaign.rs", "portal-rs/crates/community/src/referrals/tests.rs", "portal-rs/crates/store/src/loyalty.rs", "portal-rs/crates/store/src/loyalty_stream.rs", "portal-rs/crates/store/src/loyalty_stream_tests.rs", "portal-rs/crates/store/src/loyalty_tests.rs", "portal-rs/crates/support/src/sanctions/export_tests.rs", "portal-rs/crates/support/src/sanctions/store.rs", "portal-rs/migrations/20261111140004_n012_campaign.sql", "portal-rs/migrations/20261111140005_n014_loyalty_stream.sql"]
validation:
  - command: SQLX_OFFLINE=true cargo fmt --all
    result: PASS
    evidence: Exit 0, no output.
  - command: SQLX_OFFLINE=true cargo clippy --locked -p oteryn-store --all-targets -- -D warnings
    result: PASS
    evidence: Finished dev profile in 6.95s.
  - command: SQLX_OFFLINE=true cargo clippy --locked -p oteryn-community --all-targets -- -D warnings
    result: PASS
    evidence: Finished dev profile; final test-setup correction rechecked.
  - command: SQLX_OFFLINE=true cargo clippy --locked -p oteryn-support --all-targets -- -D warnings
    result: PASS
    evidence: Finished dev profile in 2.15s.
  - command: SQLX_OFFLINE=true cargo test --locked -p oteryn-store --lib loyalty
    result: PASS
    evidence: 22 passed, 0 failed, 0 ignored, 68 filtered out; PostgreSQL bodies early-return without URL.
  - command: SQLX_OFFLINE=true cargo test --locked -p oteryn-community --lib referrals::campaign::tests
    result: PASS
    evidence: 18 passed, 0 failed, 0 ignored, 37 filtered out; PostgreSQL bodies early-return without URL.
  - command: SQLX_OFFLINE=true cargo test --locked -p oteryn-support --lib sanctions::
    result: PASS
    evidence: 28 passed, 0 failed, 0 ignored, 27 filtered out; PostgreSQL bodies early-return without URL.
  - command: Product actor-to-result PostgreSQL execution
    result: NOT_RUN
    evidence: Regression covers real session rotation, first store purchase and exported totals, but no database URL is configured; no server allowed by owner check scope.
blockers: [none]
next_action: Owner reviews the completed local diff and handles publication for PR 1800.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requested local changes only on existing PR worktree; publication remains with owner.
source_branch_evidence: Current task instructions prohibit commit and push.
```

## Notes

Single agent. No optional skills authorized or used. Jira sync pending/unknown;
this worker does not load the Jira connector. Validation here is local review
coverage only, not PR readiness or integration qualification.

The complete changed-file list and diff were reviewed. Only unmerged N012 and N014
migrations changed in place; no new migrations or default-on flags. Sanction
reactivation is refused by the service rather than changing sticky export semantics.
