---
task_id: OTERYN-20261122-staff-invite
governing_issue: 2002
required_reads: []
search_first: []
optional_reads: []
---

# Staff invitations

Governing issue [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002)

Allow staff role managers to send and revoke email invitations. Accept only from the matching verified account after MFA. Keep the one-use credential digest, role changes and audit evidence in PostgreSQL.

## Acceptance criteria

- Role delegation requires current membership in the selected roles and rechecks the inviter's authority at acceptance.
- Invitations expire after 48 hours. Issuance, revocation and acceptance produce audit entries.
- Both locales support registered and existing invitees through the existing mail delivery and MFA enrollment flows.
- PostgreSQL tests cover denial and replay. Browser tests use the local mail catcher in all four projects.

## Context checkpoint

The checkpoint head records the tested portal revision immediately before this task-tracking commit.

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T18:39:37Z
head: 7fc8e0aa7e7bf63211dd44c7aa6c34ae153ea094
branch: codex/feat-staff-invite
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/admin/src/staff_invites/
  - portal-rs/templates/admin/staff-invite*.html
  - portal-rs/migrations/20261122150000_staff_invites.sql
  - portal-rs/migrations/20261122150100_staff_invite_safeguards.sql
  - portal-rs/e2e/specs/staff-invite.spec.mjs
proven:
  - The existing roles page requires admin.roles.manage and admin.access.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/bin/oteryn-portal/src/
  - portal-rs/crates/admin/src/
  - portal-rs/crates/identity/src/
  - portal-rs/crates/web/src/ratelimit.rs
  - portal-rs/e2e/
  - portal-rs/migrations/20261122150000_staff_invites.sql
  - portal-rs/migrations/20261122150100_staff_invite_safeguards.sql
  - portal-rs/templates/admin/
validation:
  - command: cargo check --locked -p oteryn-portal
    result: PASS
    evidence: The composed portal compiles.
  - command: cargo test --locked -p oteryn-admin staff_invites
    result: PASS
    evidence: Three PostgreSQL invite tests passed. The existing localized template and authorization/CSRF tests passed separately.
  - command: node run.mjs <private-environment> test specs/staff-invite.spec.mjs
    result: PASS
    evidence: Polish and English desktop and mobile each passed with the existing mail catcher. Four passed with zero retries.
  - command: node harness.mjs --seed-plan <private-account-plan> --exit-after-ready
    result: PASS
    evidence: The existing cross-repository harness started and the seed CLI created an active account.
  - command: cargo clippy --locked -p oteryn-admin -p oteryn-portal --all-targets -- -D warnings
    result: FAIL
    evidence: Existing result_large_err findings in oteryn-store. Strict focused lint also reports chunks_exact_to_as_chunks in admin commerce.
  - command: Focused PostgreSQL invite, transaction guard, rate policy and composed route boundary tests
    result: PASS
    evidence: All ten tests passed after the review fixes.
  - command: Playwright staff-invite.spec.mjs in pl-1440 and en-390
    result: PASS
    evidence: Both projects passed with the real mail catcher. The spec also checks sign-out for MFA reauthentication and rejects a copy of the pre-acceptance cookie.
  - command: Staff invite, rate-limit and disabled-worker regressions
    result: PASS
    evidence: Seven invite tests, fifteen rate-limit tests and two worker tests passed. Reclassification requires approval in listing, delivery and redemption. Disabled delivery preserves queued work with zero attempts.
  - command: Staff invite Playwright in pl-1440 and en-390 after merging main
    result: PASS
    evidence: Both browsers saw HTTP 429 on management and redemption across forwarded source addresses. The launcher started from /tmp using a portal-root-relative binary path.
  - command: Focused invite, migration, transaction guard and registration delivery tests
    result: PASS
    evidence: Twenty Rust tests passed. The stale-approval regression replaces an approver who lost the invited role while retaining management permissions. The PostgreSQL upgrade regression rejects renewal and acknowledgement from the original claim.
  - command: Staff invite and login throttle Playwright in pl-1440 and en-390
    result: PASS
    evidence: Six cases passed. Both invite cases matched the UTC expiry against captured mail and proved account-wide HTTP 429 using distinct loopback peers. The launcher ran with its default proxy trust behavior.
  - command: Focused PostgreSQL invitation, admin web, transaction guard, identity MFA, delivery, flag boundary and rate-limit tests
    result: PASS
    evidence: 55 tests passed. The regressions preserve the accepting grant, clear other MFA credentials, refuse delivery after issuer authority is lost, and reject display-name recipients before creating an invitation or job.
  - command: node harness.mjs --seed-plan <private-account-plan> --exit-after-ready
    result: PASS
    evidence: The composed portal, worker and gateway reached readiness and the seed CLI created an active account on this revision.
  - command: Playwright staff-invite, auth-mfa and auth-throttle in pl-1440 and en-390
    result: FAIL
    evidence: All ten cases passed with zero report errors. The launcher returned exit 1 after the report.
  - command: Playwright staff-invite in pl-1440 and en-390
    result: PASS
    evidence: Both cases passed with exit 0 and zero report errors, retries or skips. Both verified the selected console locale through the form, redirect and captured email. A remembered browser from before promotion needed a fresh MFA challenge. The launcher resolved the binary relative to the portal root while running from /tmp.
blockers: []
next_action: Review the pushed staff invitation branch for issue 2002.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue 2002 remains open and the owner requested branch-only delivery.
source_branch_evidence: pending verified push
```
