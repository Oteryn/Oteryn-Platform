---
task_id: OTERYN-20261122-panels-foundation
governing_issue: 2010
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261122-panels-foundation

## Goal

Implement the Foundation row of the panel capability matrix for issue [#2010](https://github.com/Oteryn/Oteryn-Platform/issues/2010).

## Acceptance criteria

- [x] Shared, permission-scoped navigation for the player, admin and GM panels.
- [x] Typed staff-action helper with CSRF, durable MFA, a bounded reason, and transactional audit, exercised by role grant.
- [x] Append-only permission catalog and additive migration.
- [x] Rust and Playwright verification.

## Ownership

```yaml
owned_paths:
  - portal-rs/crates/ui/src/panels/**
  - portal-rs/crates/admin/src/staff_actions/**
  - portal-rs/e2e/specs/panels/foundation.spec.mjs
modules:
  - portal-rs UI and route integration
  - admin role grants
dependencies:
  - panel capability matrix from codex/docs-panel-matrix
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T15:21:41Z
head: UNKNOWN
branch: codex/feat-panel-foundation
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/ui/src/panels/**
  - portal-rs/crates/admin/src/staff_actions/**
  - portal-rs/e2e/specs/panels/foundation.spec.mjs
proven:
  - The requested scope is the Foundation row on the matrix branch.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/ui/src/panels/mod.rs
  - portal-rs/crates/admin/src/staff_actions/mod.rs
  - portal-rs/e2e/specs/panels/foundation.spec.mjs
validation:
  - command: Focused Rust tests on PostgreSQL 18.6
    result: PASS
    evidence: Registry, staff-action guards and audit, role grant, GM gates, catalog seeds, and historical analytics rollback.
  - command: Pinned Rustfmt and workspace Clippy with warnings denied
    result: PASS
    evidence: Rust 1.97.0 formatting and all-targets lint checks.
  - command: Playwright foundation.spec.mjs
    result: PASS
    evidence: pl-1440, pl-390, en-1440, and en-390 passed.
blockers:
  - none
next_action: Await owner review of the Foundation branch.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Foundation task branch publication was requested without integration.
source_branch_evidence: governing Issue remains open
```
