---
task_id: OTERYN-20261115-a3-07-portal-bin
governing_issue: 536
required_reads:
  - portal-rs/README.md
search_first:
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/seed.rs
optional_reads: []
---

# OTERYN-20261115-a3-07-portal-bin

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/536 (open at admission).
Owner instruction: single agent; fix W3-001..004 from integration base
90234d8d2d0a51a654b8ff6947d38372a740acba; push only the assigned task branch.
No PR or integration is authorized. Parallel RBAC-owned files remain untouched.

## Goal and ownership

Preserve disabled outbox consumers' work, supervise admission failures, bound
signing-custody database waits, and replace verification-token cleanup's no-op.
Product ownership: portal binary, db outbox, gameauth admission and trust registry.
No migration or contract changes. Jira synchronization is coordinator-owned.

## Acceptance and evidence

All four findings confirmed on the specified base. Regression tests exercise real
PostgreSQL via PORTAL_TEST_DATABASE_URL, with names and durations recorded at
execution. Required validation: fmt, all-target Clippy and tests for oteryn-portal,
oteryn-db and oteryn-gameauth. Validation and push evidence will be completed below.

Focused regression evidence (Rust 1.97.0, real local PostgreSQL):
- disabled_consumers_preserve_approved_refund_and_resume_once: PASS, 1.498 s.
  Real staff approval, disabled production registry, enabled production registry
  in an isolated child process, one refund dispatch audit.
- configured_token_cleanup_preserves_live_tokens_and_resend_window:
  PASS, 1.283 s; old expired/used rows deleted, live/recent used rows retained.
- production_admission_registration_preserves_termination_cause: PASS;
  production registration names admission and retains both I/O failure and panic.
- custody_deadline_bounds_locked_publication_and_releases_custody:
  PASS, 12.851 s including initial activation; 250 ms publication deadline and
  reopening custody before releasing the competing PostgreSQL row lock.
- custody_deadline_bounds_activation_and_validation_snapshots:
  PASS, 12.799 s including initial activation; stage-specific 250 ms deadlines.
- custody_activation_stages_share_one_deadline: PASS, 0.10 s.
- clock_watchdog_bounds_database_pool_wait: PASS, 1.623 s.

All seven tests also failed with the corresponding defective behaviors temporarily
restored (claim disabled jobs, no-op cleanup, omit admission supervision, omit
asynchronous deadline coverage). The fixed sources were restored afterward.
No tests were weakened, disabled or newly ignored. All new SQL-backed tests
executed. The suite retains one pre-existing ignored Chromium checkout test;
its required @axe-core/playwright module is absent, and no tools were installed.

An initial parallel full-suite run exhausted PostgreSQL shared lock memory during
concurrent migrations (10 gameauth setup errors). Validation was rerun with
RUST_TEST_THREADS=1 rather than weakening tests or altering PostgreSQL.
The environment's installed 1.100 override was cleared so the repository's
installed 1.97 pin applies; no tools were installed or pins changed.


## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T21:25:03Z
head: 90234d8d2d0a51a654b8ff6947d38372a740acba
branch: claude/happy-davinci-ijz3wy-fix-a3-07-portal-bin
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/bin/oteryn-portal
  - portal-rs/crates/db
  - portal-rs/crates/gameauth
proven:
  - All four findings W3-001..004 confirmed on the integration base
  - Seven focused regressions pass and fail with the defective behaviour restored
  - No migration, contract or RBAC-owned file changed
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/bin/oteryn-portal/src/game_login.rs
  - portal-rs/bin/oteryn-portal/src/game_login_tests.rs
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/seed.rs
  - portal-rs/bin/oteryn-portal/src/seed_tests.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/db/src/outbox.rs
  - portal-rs/crates/gameauth/src/admission/http.rs
  - portal-rs/crates/gameauth/src/keys/trust.rs
  - docs/agents/tasks/active/OTERYN-20261115-a3-07-portal-bin.md
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Rust 1.97 repository pin
  - command: cargo clippy --locked --all-targets -p oteryn-portal -p oteryn-db -p oteryn-gameauth -- -D warnings
    result: PASS
    evidence: task run on the integration base
  - command: cargo test --locked -p oteryn-db -p oteryn-gameauth -p oteryn-portal
    result: PASS
    evidence: RUST_TEST_THREADS=1, real local PostgreSQL; 8, 156 and 194 passed, one unchanged browser ignore
blockers:
  - none
next_action: Coordinator reviews the authorized fix branch; no PR or merge requested
```

## Final validation and handoff

- PASS: cargo fmt --all --check.
- PASS: cargo clippy --locked --all-targets -p oteryn-portal -p oteryn-db -p oteryn-gameauth -- -D warnings.
- PASS: cargo test --locked for all three touched crates, RUST_TEST_THREADS=1:
  oteryn-db 8 passed, oteryn-gameauth 156 passed, oteryn-portal 194 passed
  (190 unit and 4 integration), zero failures, one pre-existing browser ignore.
- The portal suite was rerun after the final deadline-race adjustment:
  190 unit tests passed in 285.03 s, all four integration tests passed.
- All seven focused regressions passed again on the final product sources;
  five PostgreSQL timings are listed above. No new PostgreSQL tests skipped.
- All product changes are within the assigned owned paths. This required task
  packet is the sole extra path. No migration or RBAC-owned file changed.
- Ready for the owner's coordinator to review and integrate the assigned branch.
  No PR creation, merge, deployment or lifecycle Issue closure was performed.

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner-assigned audit branch retained for coordinator integration
source_branch_evidence: owner requested push only, no PR; coordinator review on delivery
```
