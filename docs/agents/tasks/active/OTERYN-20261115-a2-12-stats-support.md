---
task_id: OTERYN-20261115-a2-12-stats-support
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit wave 2: statistics and support

Governing workstream: https://github.com/Oteryn/Oteryn-Platform/issues/536 (OPEN, verified live 2026-11-15). The owner specifies six findings, exact base/branch, validation and branch publication without a PR. Single agent; no delegation. Existing active packets and open PRs have no overlapping implementation ownership. Jira synchronization remains with the coordinator.

Base: c6fdc5438a4afa91f32d547b7cfc61f22bf21139. Branch: claude/happy-davinci-ijz3wy-fix-a2-12-stats-support.

All six findings confirmed against the base. Scope: fair statistics batches; staff message pagination; displayed-reply acknowledgement; account-before-RBAC ordering including legacy sanction services; committed report replay budgets; nullable permanent expiry refusal. No migration, contract or feature-flag changes.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T17:22:46Z
head: c6fdc5438a4afa91f32d547b7cfc61f22bf21139
branch: claude/happy-davinci-ijz3wy-fix-a2-12-stats-support
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/stats
  - portal-rs/crates/support
  - portal-rs/crates/admin
proven:
  - All six findings are present on the admitted base
  - SW-01 PostgreSQL regression fails with 100 of 205 subjects and passes after fair scheduling
  - Every finding reproduced with old behavior restored individually, including both SW-03 paths and legacy sanction paths
  - Configured staff-router regressions cover pagination, moderation concurrency and permanent expiry
  - Rust 1.97 formatting, warnings-denied Clippy and all 297 tests pass; no failed or ignored tests
  - All seven new PostgreSQL regressions executed and took more than 0.05 seconds
unknown:
  - Coordinator Jira synchronization and integration disposition
derived: []
conflicts: []
first_failure:
  marker: none
  evidence: expected pre-fix failures retained in local logs
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/admin/src/support.rs
  - portal-rs/crates/admin/src/support_pagination_tests.rs
  - portal-rs/crates/admin/src/support_tests.rs
  - portal-rs/crates/admin/src/support_web.rs
  - portal-rs/crates/stats/src/tests.rs
  - portal-rs/crates/stats/src/worker.rs
  - portal-rs/crates/support/src/reports.rs
  - portal-rs/crates/support/src/reports/tests.rs
  - portal-rs/crates/support/src/sanctions/mod.rs
  - portal-rs/crates/support/src/sanctions/store.rs
  - portal-rs/crates/support/src/sanctions/tests.rs
  - portal-rs/crates/support/src/tickets/mod.rs
  - portal-rs/crates/support/src/tickets/staff.rs
  - portal-rs/crates/support/src/tickets/staff_tests.rs
  - portal-rs/crates/support/src/tickets/tests.rs
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Installed repository pin Rust 1.97; no tool installation
  - command: cargo clippy --locked --all-targets -p oteryn-admin -p oteryn-stats -p oteryn-support -- -D warnings
    result: PASS
    evidence: All three touched crates, final implementation
  - command: cargo test --locked -p oteryn-admin -p oteryn-stats -p oteryn-support -- --nocapture --test-threads=1
    result: PASS
    evidence: 297 passed (admin 151, stats 55, support 91), zero failed or ignored; real local PostgreSQL, no skipped tests reported
  - command: Seven before/after PostgreSQL regressions
    result: PASS
    evidence: Stats batch 0.906s; staff pagination 1.560s; read acknowledgement 0.244s; staff lock order 1.677s; legacy lock order 0.247s; report replay budget HTTP 0.132s; permanent expiry HTTP 1.204s
  - command: Product actor-to-result validation
    result: PASS
    evidence: Configured production staff router exercises authenticated, MFA-guarded SSR pagination and CSRF-protected moderation/enforcement mutations; report router exercises replay 409 then 429; PostgreSQL exercises polling and owner read boundaries
blockers: []
next_action: Publish the requested branch for coordinator review; no PR, merge or deployment authorized
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner-requested fix branch for coordinator review; no PR or merge requested
source_branch_evidence: Exact publication SHA and push result will be reported at closeout
```

All implementation paths are within the owner-listed crates. The only extra path is this required repository task packet. No migration, contract, workflow, dependency or feature-flag changes. No tests were weakened, skipped, ignored or removed. The checkpoint head records the admitted base; validation covers its final candidate. Git publication reports the resulting commit SHA, avoiding a self-referential packet SHA. The owner requests a branch push without a PR, so no PR exact-head CI gate or merge readiness is claimed. Retain the source branch for coordinator review until the coordinator decides its integration disposition.
