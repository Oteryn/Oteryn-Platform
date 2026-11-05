---
task_id: OTERYN-20261105-portal-rs-pkg6
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261105-portal-rs-pkg6

## Goal

Assemble package 6 on the package 4 branch under the owner's explicit integration instructions. Governing Platform workstream: https://github.com/Oteryn/Oteryn-Platform/issues/530. No PR requested.

## Acceptance criteria

- Preserve all integrable source branches with ordered non-fast-forward merges.
- Preserve module behavior; resolve shared registrations and dependency conflicts.
- Verify unique migration versions, formatting, strict clippy, locked tests against PostgreSQL 18.6, and legacy token ratchet.
- Push the dedicated branch normally and verify remote head.

## Ownership

```yaml
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261105-portal-rs-pkg6.md
modules:
  - package-6-integration
dependencies:
  - package-4
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-05T17:35:24Z
head: 7143b0c9
branch: codex/portal-rs-pkg6
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/**
proven:
  - Nine retained branches merged in the requested order with two-parent merge commits; T06 excluded after a PostgreSQL lock failure.
  - Numeric migration versions are unique; no renumbering required.
derived: []
unknown: []
conflicts:
  - Shared dependency tables and module/router/worker registrations retained additively and deduplicated.
first_failure:
  marker: T06_MFA_LOCK_WAIT
  evidence: MFA authorization holds an account FOR UPDATE lock while session rotation on another connection waits for the foreign-key lock.
rejected_hypotheses: []
changed_paths:
  - portal-rs/**
validation:
  - command: cargo clippy --locked --all-targets -- -D warnings
    result: PASS
    evidence: Exact material candidate 7143b0c9; all targets passed with warnings denied.
  - command: cargo fmt --all --check
    result: PASS
    evidence: Exact material candidate 7143b0c9.
  - command: cargo test --locked
    result: PASS
    evidence: PostgreSQL 18.6; 472 reported passing, 0 failed, 1 ignored; 274 tests actually used PostgreSQL.
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: Printed ratchet ok.
blockers:
  - none
next_action: Publish normally and retain the package branch for downstream integration.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requested package branch publication without a PR; retained for subsequent integration.
source_branch_evidence: Owner instruction in integration task.
```

## Notes

Jira mapping is unknown; synchronization remains pending for the programme coordinator. No external messages sent. Product runtime E2E is not applicable to this assembly task; module behavior is covered by the requested workspace tests.

## Conflict resolution ledger

Paths below are relative to `portal-rs/`.

| Merge | Conflicted files | Resolution |
| --- | --- | --- |
| T05 | Cargo.lock; bin/oteryn-portal/Cargo.toml; bin/oteryn-portal/src/main.rs; crates/identity/Cargo.toml | Retained reset route alongside existing routes; added reset dependencies in the correct dependency scope; preserved wiki/world tables. |
| T06 | Cargo.lock; bin/oteryn-portal/Cargo.toml; bin/oteryn-portal/src/main.rs; crates/identity/src/lib.rs | Attempted additive resolution with reset/MFA modules and dependencies, removing duplicate subtle declaration. DROPPED: reset to before T06 and replayed later merges after the PostgreSQL browser test blocked. |
| T07 | Cargo.lock; bin/oteryn-portal/Cargo.toml; bin/oteryn-portal/src/main.rs; crates/identity/src/lib.rs | Added passkey modules/routes and dependencies; kept equivalent serde entries once. Initial discarded assembly also conflicted in worker.rs and identity/Cargo.toml; both cleanup calls and dependency tables were complete. Final replay merged these files cleanly. |
| T16 | Cargo.lock; bin/oteryn-portal/src/main.rs; crates/gameauth/Cargo.toml; crates/gameauth/src/lib.rs | Kept key/scope modules and dependencies alongside OAuth; retained all existing routes and OAuth login middleware. |
| T14b | bin/oteryn-portal/src/main.rs | Retained all routes and mounted entities. Removed automatically merged duplicate world dependency from wiki manifest. |
| T33 | Cargo.lock; bin/oteryn-portal/Cargo.toml; bin/oteryn-portal/src/main.rs; crates/characters/src/lib.rs; crates/store/Cargo.toml | Retained existing calculator/bootstrap/store declarations; added support dependency at root dependency scope and mounted support routes. |
| T08c | Cargo.lock; bin/oteryn-portal/Cargo.toml; bin/oteryn-portal/src/main.rs; crates/identity/Cargo.toml; crates/identity/src/lib.rs | Added credentials module/router and dependencies; kept one login mount and one serde dependency. Preserved notify dependency table body. |
| T09 | bin/oteryn-portal/Cargo.toml; bin/oteryn-portal/src/main.rs; crates/identity/Cargo.toml; crates/identity/src/lib.rs | Added export module/router; retained equivalent existing identity and axum dependencies once. |
| T09b | bin/oteryn-portal/Cargo.toml; crates/identity/src/lib.rs | Added termination module; retained existing identity dependency once and preserved automatic worker registration. |
| T11d | bin/oteryn-portal/Cargo.toml; bin/oteryn-portal/src/main.rs; crates/content/Cargo.toml; crates/content/src/lib.rs | Kept all content modules once; retained equivalent content/askama dependencies once; added today router/banner middleware. Restored closing export call and complete pages call; removed automatically merged duplicate news mount. |

For every lockfile conflict, retained the current package base side and regenerated with `cargo metadata --format-version 1`. Also ran metadata after clean lockfile merges. No hand edits to Cargo.lock. No SQLx offline cache exists. No migration version collision exists. T06 MFA dropped; all other requested modules retained. Formatting only reorders module declarations and formats shared integrations.

## Dropped module evidence

T06 MFA: `mfa_http::tests::postgres_mfa_browser_csrf_limiter_and_login_challenge` blocked during session establishment. PostgreSQL showed one connection idle in a transaction after validating an MFA grant while holding the account row lock, and another connection waiting on a transactionid lock during `UPDATE web_sessions ... account_id=...`. `Mfa::authorize_session` intentionally holds `FOR UPDATE` on the account until `SessionAuthorization::finish`; `establish` awaits `session::rotate` through a separate pool connection before calling `finish`. The session account foreign key makes the second connection wait for the first transaction. Fixing transaction ownership or lock semantics changes authentication concurrency behavior and is outside conflict resolution/minimal compile fixes. The blocked test process was terminated, the branch reset to immediately before T06, and all later requested merges replayed without T06. No behavioral redesign was attempted.

The final source tree was compared with the first assembly: only the T06 module, its dependencies, migration, templates and registrations are absent (plus task evidence).

## Final acceptance summary

Material candidate: `7143b0c9`. Executed the requested chain with `PORTAL_TEST_DATABASE_URL` set transiently to local PostgreSQL 18.6 (credentials never printed or committed):

```text
cd portal-rs && cargo fmt --all --check && cargo clippy --locked --all-targets -- -D warnings && PORTAL_TEST_DATABASE_URL=<local PG 18.6 URL> cargo test --locked
PASS: formatting
PASS: clippy, all targets, warnings denied
PASS: cargo test — 472 reported passing, 0 failed, 1 ignored
PostgreSQL-backed tests actually run: 274
python3 tools/validation/legacy_token_ratchet.py
ratchet ok
```

The count matches successful test names to database fixtures, including macro-generated online tests and tests using multithreaded runtimes. It excludes the explicitly ignored Chromium account-layout test, the real pinned-content import that returned early because its external checkout is absent, and the asset compatibility smoke test that returned early because assets are unset. Both early-return tests were checked directly using the already-built test binaries with output capture disabled; they finished in 0.00s and printed explicit skip notices. No database-URL skip was counted as execution.

44 unique migration versions; no migration renumbered. No SQLx offline cache. No workflow changes. No PR created. Final conflict resolutions alter only shared declarations and registration composition; no module behavior redesigned. Heavy acceptance ran again after dropping T06 because that changed the material tree. The subsequent task-evidence commit preserves the exact tested `portal-rs` tree.

Retained merge order: T05 (password reset), T07 (passkeys), T16 (OAuth), T14b (wiki entities), T33 (support tickets), T08c (password changes/sudo), T09 (data export), T09b (account termination), T11d (announcements/digest).
