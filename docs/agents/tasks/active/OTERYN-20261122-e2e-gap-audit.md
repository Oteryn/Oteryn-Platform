---
task_id: OTERYN-20261122-e2e-gap-audit
governing_issue: 2002
required_reads: []
search_first: []
optional_reads: []
---

# Staff panel account menu

[Governing issue #2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002)

The account menu exposes `/admin` on the enabled staff panel's configured host
for active, email-verified accounts holding `admin.access` and using a session
with a current MFA grant. The panel enforces its own server-side authorization.
The scope removal commit `0f0e01d6` moves the general crawl and unrelated
product changes out of this branch's diff.

The PostgreSQL regression seeds an unexpired session for each account state and
exercises MFA grant expiry. The Playwright regression checks that a staff role
alone gives no link and that the panel returns its designed MFA refusal. It
signs in through MFA, follows the staff link, and checks the destination's HTML
and visible localized heading. It then suspends the account and checks normal
session revocation.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T17:58:58Z
head: 75a4bc00e152fb795978595b6a377ac36e84d5a2
branch: codex/e2e-gap-audit
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/large_pages_tests.rs
  - portal-rs/crates/admin/src/web.rs
  - portal-rs/crates/identity/src/consents_http.rs
  - portal-rs/crates/ui/src/lib.rs
  - portal-rs/crates/ui/i18n/
  - portal-rs/crates/web/src/chrome.rs
  - portal-rs/crates/web/src/session.rs
  - portal-rs/crates/web/src/session_tests.rs
  - portal-rs/templates/components/thais-account-links.html
  - portal-rs/e2e/helpers.mjs
  - portal-rs/e2e/staff-rbac-helpers.mjs
  - portal-rs/e2e/specs/staff-menu.spec.mjs
proven:
  - The existing staff host guard and shared menu use the same Host matcher
  - The product fix only changes the staff link visibility predicate
  - The disposable PostgreSQL 18.6 fixture uses the normal account state triggers
derived: []
unknown: []
conflicts: []
first_failure:
  marker: independent review
  evidence: A suspended staff account with a retained role and session could still see the staff link
rejected_hypotheses: []
changed_paths: []
validation:
  - command: cargo test --locked -p oteryn-web --lib staff_navigation_requires_eligible_account_on_enabled_panel_host
    result: PASS
    evidence: Disabled panel, host and port mismatch, expired MFA, suspended account, and unverified account were covered with PostgreSQL.
  - command: focused web and admin tests and the UI library suite
    result: PASS
    evidence: Sixteen Rust tests passed in total, including the staff visibility regression, against the disposable PostgreSQL environment.
  - command: node run.mjs /absolute/scratch/env.json test specs/staff-menu.spec.mjs --workers=2 --trace=off
    result: PASS
    evidence: All four Polish and English desktop and mobile projects passed against the standard local launcher after the MFA fix.
  - command: node run.mjs /absolute/scratch/env.json test --list
    result: PASS
    evidence: Each project discovers the focused staff-menu spec. The general crawl spec was removed from this branch.
  - command: cargo build --locked -p oteryn-portal
    result: PASS
    evidence: The native binary used by the launcher built successfully.
blockers: []
next_action: Push the validated branch and complete the owner's post-push review
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: The existing audit branch is the requested deliverable under issue 2002.
source_branch_evidence: The previous published commit is preserved in the branch history.
```
