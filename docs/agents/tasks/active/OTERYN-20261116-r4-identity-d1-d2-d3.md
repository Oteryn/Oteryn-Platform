---
task_id: OTERYN-20261116-r4-identity-d1-d2-d3
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261116-r4-identity-d1-d2-d3

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4 review claims D1, D2 and D3 on main a6b1b05e, each reproduced with a
red test and fixed:

- D1: a pending email change survived a password reset or a recovery-key
  recovery, so a confirmation link issued before the owner recovered the
  account still installed the requested address.
- D2: duplicate registration returned no new session cookie, while a new
  registration did, revealing whether the address exists.
- D3: a passkey ceremony consumed while valid was accepted after it expired
  during the account lock wait, for authentication and registration.

## Acceptance criteria

- [x] Password reset and recovery-key recovery cancel every unapplied email change in the same transaction, with the account row locked
- [x] Duplicate and new registration both issue a fresh session cookie; the duplicate session stays anonymous
- [x] Passkey authentication and registration refuse a ceremony that expired during the account lock wait
- [x] Each case has a test that was red on a6b1b05e and is green with the fix

## Ownership

```yaml
owned_paths:
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
  - portal-rs/crates/identity/src/passkeys.rs
  - portal-rs/crates/identity/src/passkeys_tests.rs
  - portal-rs/crates/identity/src/recovery_key.rs
  - portal-rs/crates/identity/src/reset.rs
  - portal-rs/crates/identity/tests/registration.rs
  - portal-rs/crates/web/src/routes/register.rs
  - portal-rs/crates/web/src/session.rs
  - docs/agents/tasks/active/OTERYN-20261116-r4-identity-d1-d2-d3.md
modules:
  - identity
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
updated_at: 2026-11-16T23:40:00Z
head: a6b1b05eed5e95b93ea549d230b531c041e7d9b0
branch: claude/happy-davinci-ijz3wy-r4fix
pr: 1893
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/identity/
  - portal-rs/crates/web/src/routes/register.rs
  - portal-rs/crates/web/src/session.rs
  - docs/agents/tasks/active/OTERYN-20261116-r4-identity-d1-d2-d3.md
proven:
  - Base is origin/main a6b1b05eed5e95b93ea549d230b531c041e7d9b0
  - No migration, contract, fixture, flag or wire byte change
  - No Game repository, production service or production data accessed
  - Without the product change the five new or extended tests fail with the asserted refusal missing
  - With the product change the full identity and web crate suites pass on PostgreSQL 18
derived:
  - Confirmation locks the account before the change row, as reset and recovery now do, so no new lock order
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Cancelling in revoke_authorizations was rejected because sign-out-all, password change and email-change revert also call it
changed_paths:
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
  - portal-rs/crates/identity/src/passkeys.rs
  - portal-rs/crates/identity/src/passkeys_tests.rs
  - portal-rs/crates/identity/src/recovery_key.rs
  - portal-rs/crates/identity/src/reset.rs
  - portal-rs/crates/identity/tests/registration.rs
  - portal-rs/crates/web/src/routes/register.rs
  - portal-rs/crates/web/src/session.rs
  - docs/agents/tasks/active/OTERYN-20261116-r4-identity-d1-d2-d3.md
validation:
  - command: cargo test --locked -p oteryn-identity --lib -- d1_ d3_ (product files reverted)
    result: PASS
    evidence: expected red, 0 passed 4 failed at the refusal assertions
  - command: cargo test --locked -p oteryn-identity --test registration duplicate_email_has_same_response (product files reverted)
    result: PASS
    evidence: expected red, cookie issuance differs between new and duplicate registration
  - command: cargo test --locked -p oteryn-identity --no-fail-fast
    result: PASS
    evidence: lib 162 passed, registration 13 passed, 0 failed, PostgreSQL 18 enabled, no SKIPPED line
  - command: cargo test --locked -p oteryn-web --no-fail-fast
    result: PASS
    evidence: 42 plus 5 plus 14 passed, 0 failed, 1 ignored browser test unchanged
  - command: cargo fmt --all --check
    result: PASS
    evidence: exit 0
  - command: cargo clippy --locked --all-targets -- -D warnings
    result: PASS
    evidence: exit 0 on the whole workspace
blockers:
  - none
next_action: PR review and merge, then archive this packet
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
