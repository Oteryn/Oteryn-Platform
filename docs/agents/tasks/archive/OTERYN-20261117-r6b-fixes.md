---
task_id: OTERYN-20261117-r6b-fixes
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-r6b-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Review round 6 lens findings, each confirmed by a read-only check, fixed by
Codex Cloud with a red-then-green test and integrated here:

- l1 admin CSRF: the admin panel keeps its own host as the trusted CSRF origin when it is mounted under the public router's CSRF extension.
- l2 email verification: consuming a verify link logs out only the verified account's session (or an already-revoked one), not an unrelated browser session.
- l3 profile B: the per-peer budget is charged before body validation, so malformed requests cannot bypass it.

## Acceptance criteria

- [x] Each fix has a test that fails with its source reverted to main and passes with it
- [x] Focused crate tests pass on the integrated tree
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-r6b-fixes.md
  - portal-rs/crates/admin/src/analytics/web.rs
  - portal-rs/crates/admin/src/web.rs
  - portal-rs/crates/admin/src/web_tests.rs
  - portal-rs/crates/identity/src/reset_tests.rs
  - portal-rs/crates/identity/src/verify.rs
  - portal-rs/crates/identity/tests/registration.rs
  - portal-rs/crates/store/src/premium_profile_b_http.rs
  - portal-rs/crates/store/src/premium_profile_b_tests.rs
  - portal-rs/crates/web/src/routes/verify.rs
  - portal-rs/crates/web/src/routes/verify_budget_tests.rs
modules:
  - admin
  - identity
  - store
  - web
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-17T18:14:00Z
head: 1d68385b92b28c6e1eb7a322844b73f66fd762fa
branch: claude/happy-davinci-ijz3wy-r6b-fixes
pr: 1942
status: completed
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-r6b-fixes.md
  - portal-rs/crates/admin/src/analytics/web.rs
  - portal-rs/crates/admin/src/web.rs
  - portal-rs/crates/admin/src/web_tests.rs
  - portal-rs/crates/identity/src/reset_tests.rs
  - portal-rs/crates/identity/src/verify.rs
  - portal-rs/crates/identity/tests/registration.rs
  - portal-rs/crates/store/src/premium_profile_b_http.rs
  - portal-rs/crates/store/src/premium_profile_b_tests.rs
  - portal-rs/crates/web/src/routes/verify.rs
  - portal-rs/crates/web/src/routes/verify_budget_tests.rs
proven:
  - Three read-only verifications confirmed the lens findings with file:line evidence
derived:
  - none
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-r6b-fixes.md
  - portal-rs/crates/admin/src/analytics/web.rs
  - portal-rs/crates/admin/src/web.rs
  - portal-rs/crates/admin/src/web_tests.rs
  - portal-rs/crates/identity/src/reset_tests.rs
  - portal-rs/crates/identity/src/verify.rs
  - portal-rs/crates/identity/tests/registration.rs
  - portal-rs/crates/store/src/premium_profile_b_http.rs
  - portal-rs/crates/store/src/premium_profile_b_tests.rs
  - portal-rs/crates/web/src/routes/verify.rs
  - portal-rs/crates/web/src/routes/verify_budget_tests.rs
validation:
  - command: cargo fmt --all --check; cargo clippy --locked -p oteryn-admin -p oteryn-identity -p oteryn-web -p oteryn-store --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
  - command: cargo test --locked -p oteryn-admin -p oteryn-identity -p oteryn-web -p oteryn-store (each)
    result: PASS
    evidence: admin 208, identity 181, web 62 (1 existing ignore), store 132 (1 existing ignore); PostgreSQL 18 set
  - command: red check, each fix's source reverted to origin/main
    result: PASS
    evidence: database_navigation_authorization_csrf_and_owner, verification_preserves_unrelated_session_and_logs_out_own_account and malformed_requests_consume_peer_budget each fail
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: narrow server-side fixes covered by tests through the real routers
blockers:
  - none
next_action: none; delivered by PR 1942 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1942 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1942 merged as 1d68385b92b28c6e1eb7a322844b73f66fd762fa
```
