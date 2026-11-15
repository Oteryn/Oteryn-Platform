---
task_id: OTERYN-20261115-audit-ann
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Staff announcement administration (14-F7)

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/536 (verified open).
Owner-requested branch: claude/happy-davinci-ijz3wy-fix-ann.
Base: 80fa5bf0557b2dc55f0aa71ae53b9dac0e693812.

Verified missing production caller for Announcements::save. Add shared guarded staff
list/create/edit routes, English and Polish fields, explicit draft/published state,
optimistic lock_version, and existing translation publication rules. Mount next to
news. No migration, contract change, external game access or production operation.
Owned paths: announcement service/tests/staff module, content templates and portal
main mounting. Live open PR preflight found no announcement owner overlap.

Validation: all required final candidate checks pass. Real local PostgreSQL guarded HTTP
regression covers host/flag, MFA, exact permission, CSRF, publication lifecycle,
stale versions, public /today visibility and translation reapproval.

Source branch disposition: retain for coordinator review; owner requested push
only, no PR or merge. Jira synchronization remains coordinator-owned/unknown.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T14:44:55Z
head: 80fa5bf0557b2dc55f0aa71ae53b9dac0e693812
branch: claude/happy-davinci-ijz3wy-fix-ann
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/content/src/announcements.rs
  - portal-rs/crates/content/src/announcements_staff.rs
  - portal-rs/crates/content/src/announcements_tests.rs
  - portal-rs/templates/content/announcements.html
  - portal-rs/bin/oteryn-portal/src/main.rs
proven:
  - Base announcement save has only test callers
  - Shared guard requires portal.announcements.manage, staff host, MFA and CSRF; flag defaults off
  - Every mutation delegates to Announcements::save with optimistic version
  - Guarded HTTP lifecycle and public today visibility passed against real local PostgreSQL
  - Regression with base news-only staff composition fails with missing endpoint 404 in 0.67 seconds
  - Existing ignored Chromium test was preserved unchanged
  - Live workstream issue 536 is open; no task PR requested or created
  - No game repository, production data or contract changes
  - Repository identity is Oteryn/Oteryn-Platform
  - Rust 1.97 pin is installed and used without environment compiler override
  - Formatting and Clippy pass for both touched crates
  - Parallel portal suite hit 17 local PostgreSQL lock-memory failures; final serial run passed all 185 tests with zero failures and one unchanged ignored Chromium test
derived: []
unknown:
  - Coordinator Jira mapping and synchronization
conflicts: []
first_failure:
  marker: expected-before-fix-regression
  evidence: news-only staff composition returns 404 instead of checked endpoint refusal after PostgreSQL setup
rejected_hypotheses:
  - Base already provides a production announcement mutation entry point
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261115-audit-ann.md
  - portal-rs/crates/content/src/announcements.rs
  - portal-rs/crates/content/src/announcements_staff.rs
  - portal-rs/crates/content/src/announcements_tests.rs
  - portal-rs/templates/content/announcements.html
  - portal-rs/bin/oteryn-portal/src/main.rs
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Rust 1.97
  - command: cargo clippy --locked --all-targets -p oteryn-content -p oteryn-portal -- -D warnings
    result: PASS
    evidence: Both touched crates, Rust 1.97
  - command: cargo test --locked -p oteryn-content -p oteryn-portal -- --nocapture --test-threads=1
    result: PASS
    evidence: 185 passed, zero failed, one pre-existing ignored Chromium test; announcement lifecycle 1.084s and announcement public digest 1.504s with real PostgreSQL
blockers: []
next_action: Publish the requested branch for coordinator review; no PR or merge authorized
```

The checkpoint head is the admitted base; Git reports the publication commit.
No exact-head PR gate is claimed because the owner requested a branch push only.
