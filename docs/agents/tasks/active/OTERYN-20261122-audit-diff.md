---
task_id: OTERYN-20261122-audit-diff
governing_issue: 2002
required_reads: []
search_first: []
optional_reads: []
---

# Staff audit before and after values

Governing issue https://github.com/Oteryn/Oteryn-Platform/issues/2002. Deliver F6 on codex/feat-audit-diff.

The staff console renders localized, escaped changes from existing audit JSONB for staff with audit.view. The display allowlist omits credentials and masks email addresses. A separate content fix records the absent state after editorial page deletion.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T12:50:42Z
head: 4a608fc30a65d36e6af3397d2cfc1f1a0c46db24
branch: codex/feat-audit-diff
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/admin/src/console
  - portal-rs/crates/content/src/pages
  - portal-rs/e2e/specs/staff-rbac-audit.spec.mjs
  - portal-rs/e2e/staff-rbac-helpers.mjs
proven:
  - audit_log already stores JSONB before and after values
derived: []
unknown:
  - Jira synchronization status
conflicts: []
first_failure:
  marker: MFA recovery code capture raced the confirmation response
  evidence: Staff helper now waits for the rendered recovery code controls
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/admin/src/console/access.en.ftl
  - portal-rs/crates/admin/src/console/access.pl.ftl
  - portal-rs/crates/admin/src/console/access.rs
  - portal-rs/crates/admin/src/console/audit.rs
  - portal-rs/crates/admin/src/console/players_tests.rs
  - portal-rs/crates/admin/src/console/templates/read.html
  - portal-rs/crates/content/src/pages/staff.rs
  - portal-rs/crates/content/src/pages/tests.rs
  - portal-rs/e2e/specs/staff-rbac-audit.spec.mjs
  - portal-rs/e2e/staff-rbac-helpers.mjs
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Rust 1.97
  - command: cargo clippy --locked --all-targets -p oteryn-admin -p oteryn-content -- -D warnings
    result: PASS
    evidence: Both affected crates
  - command: Focused Rust audit console and editorial page regression tests
    result: PASS
    evidence: Four redaction and diff unit tests, PostgreSQL handler rendering and PostgreSQL deletion audit regression
  - command: Playwright staff-rbac-audit.spec.mjs
    result: PASS
    evidence: All four projects passed against the real local portal after the fixture synchronization fix
  - command: harness.mjs --seed-plan with the account seed command
    result: PASS
    evidence: Portal, worker and gateway reached readiness and the seed CLI created the synthetic account
blockers: []
next_action: Publish the requested branch
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requested branch publication for F6
source_branch_evidence: The final response records the exact remote commit
```

The checkpoint head records the admitted base.
