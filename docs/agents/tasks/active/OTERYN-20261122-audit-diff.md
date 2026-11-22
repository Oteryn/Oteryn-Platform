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
updated_at: 2026-11-22T14:23:10Z
head: e24826bc0d39b43a7925539c9ed9f25cfd8dbe99
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
validation:
  - command: cargo fmt --all --check and Clippy with warnings denied in admin, content and support
    result: PASS
    evidence: Source candidate e24826bc0d39b43a7925539c9ed9f25cfd8dbe99
  - command: Focused Rust audit, CMS and support tests
    result: PASS
    evidence: Eight audit unit tests and four PostgreSQL tests passed on e24826bc0d39b43a7925539c9ed9f25cfd8dbe99
  - command: Playwright staff-rbac-audit.spec.mjs in pl-1440 and en-390
    result: PASS
    evidence: Both projects passed against e24826bc0d39b43a7925539c9ed9f25cfd8dbe99
  - command: Related Playwright news, role and support cases in pl-1440 and en-390
    result: PASS
    evidence: Eight cases passed against e24826bc0d39b43a7925539c9ed9f25cfd8dbe99
blockers: []
next_action: Independently re-review the F6 fixes on the published branch using the pinned validation evidence
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requested branch publication for F6
source_branch_evidence: The F6 branch was first published at d1bfb1c1c0097609a16c325df27732c2f6673401
```

The checkpoint head identifies the source candidate. The documentation-only checkpoint commit follows it. [Reviewed F6 commit](https://github.com/Oteryn/Oteryn-Platform/commit/d1bfb1c1c0097609a16c325df27732c2f6673401) · [Review fixes](https://github.com/Oteryn/Oteryn-Platform/commit/e24826bc0d39b43a7925539c9ed9f25cfd8dbe99)

The separate admin navigation sweep hit its time limit after successful staff authentication. The companion run uses real staff sessions created through the browser harness.
