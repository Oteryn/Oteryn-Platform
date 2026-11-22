---
task_id: OTERYN-20261122-audit-diff
governing_issue: 2002
required_reads: []
search_first: []
optional_reads: []
---

# Staff audit before and after values

Governing issue https://github.com/Oteryn/Oteryn-Platform/issues/2002. Deliver F6 on codex/feat-audit-diff.

The staff console renders localized, escaped changes from existing audit JSONB for staff with audit.view. The display allowlist omits credentials and masks email addresses. The review follow-up hides unrestricted CMS text, identifies role grants and revocations, compares publication instants, and distinguishes missing historical appeal data.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T15:05:35Z
head: 80681951d06fe8e4124648d67af9bfecf0502cab
branch: codex/feat-audit-diff
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/admin/src/console
  - portal-rs/crates/content/src/pages
  - portal-rs/crates/content/src/news.rs
  - portal-rs/crates/content/src/news_tests.rs
  - portal-rs/crates/support/src/sanctions/store.rs
  - portal-rs/crates/support/src/tickets/staff_tests.rs
  - portal-rs/e2e/specs/admin.spec.mjs
  - portal-rs/e2e/specs/staff-rbac-audit.spec.mjs
  - portal-rs/e2e/staff-rbac-helpers.mjs
  - portal-rs/static/css/components.css
proven:
  - audit_log already stores JSONB before and after values
  - The published F6 candidate d1bfb1c1 received two independent reviews
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
  - portal-rs/crates/content/src/news.rs
  - portal-rs/crates/content/src/news_tests.rs
  - portal-rs/crates/content/src/pages/staff.rs
  - portal-rs/crates/content/src/pages/tests.rs
  - portal-rs/crates/support/src/sanctions/store.rs
  - portal-rs/crates/support/src/tickets/staff_tests.rs
  - portal-rs/e2e/specs/admin.spec.mjs
  - portal-rs/e2e/specs/staff-rbac-audit.spec.mjs
  - portal-rs/e2e/staff-rbac-helpers.mjs
  - portal-rs/static/css/components.css
validation:
  - command: Focused Rust audit rendering and PostgreSQL handler tests
    result: PASS
    evidence: Fourteen tests passed on 80681951d06fe8e4124648d67af9bfecf0502cab
  - command: Formatting and Clippy with warnings denied
    result: PASS
    evidence: Workspace formatting and all admin targets passed on 80681951d06fe8e4124648d67af9bfecf0502cab
  - command: Playwright admin and staff audit specs in pl-1440 and en-390
    result: PASS
    evidence: Both specs passed in both projects on 80681951d06fe8e4124648d67af9bfecf0502cab
  - command: Harness readiness and account seed
    result: PASS
    evidence: Existing harness reached readiness and created a disposable account on 80681951d06fe8e4124648d67af9bfecf0502cab
blockers: []
next_action: Independently re-review the published F6 fixes and the pinned validation evidence
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requested branch publication for F6
source_branch_evidence: The F6 branch was first published at d1bfb1c1c0097609a16c325df27732c2f6673401
```

The source candidate addresses the follow-up review of [2fc7d428](https://github.com/Oteryn/Oteryn-Platform/commit/2fc7d428697a487fb5137928ec0e7359b22c390b). The checkpoint records the [tested source commit](https://github.com/Oteryn/Oteryn-Platform/commit/80681951d06fe8e4124648d67af9bfecf0502cab). The documentation-only checkpoint commit follows it.
