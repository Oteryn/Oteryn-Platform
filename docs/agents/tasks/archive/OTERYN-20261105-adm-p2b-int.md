---
task_id: OTERYN-20261105-adm-p2b-int
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# ADM-P2B-INT

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530 (open Platform programme). Owner task ADM-P2B-INT supplies the bounded scope and authorizes branch publication without a PR.

## Goal

Connect control evidence to identity MFA and the durable request store with transactional audit; move control module declarations under control while retaining public aliases. Base: 0c54f21e4053920ca77336f70d90d277c5380d92.

## Ownership

```yaml
owned_paths:
  - portal-rs/crates/admin/src/control/*
  - portal-rs/crates/admin/src/lib.rs
  - portal-rs/crates/admin/Cargo.toml
  - portal-rs/Cargo.lock
  - portal-rs/crates/identity/src/mfa*
modules:
  - admin control evidence
  - identity MFA verifier
dependencies:
  - ADM-P1A/P2B/P2C
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-05T21:21:35Z
head: 0c54f21e4053920ca77336f70d90d277c5380d92
branch: codex/portal-rs-adm-p2b-int
pr: none
status: completed
context_routes: []
owned_paths: []
proven:
  - exact owner-specified base and clean workspace verified
  - open Issue 530 and open PR ownership checked; no task overlap found
  - all five new security regressions and the complete locked workspace suite passed
  - crate-root aliases compile across all workspace targets
derived:
  - existing schema can retain request-linked evidence without a migration
unknown: []
conflicts: []
first_failure:
  marker: resolved-test-fixtures
  evidence: session revocation, rotated-cookie and single-connection test fixtures corrected without changing assertions; local PostgreSQL capacity and shared-schema contamination resolved using a fresh database and max_locks_per_transaction=512
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261105-adm-p2b-int.md
  - portal-rs/Cargo.lock
  - portal-rs/crates/admin/Cargo.toml
  - portal-rs/crates/admin/src/control/controls.rs
  - portal-rs/crates/admin/src/control/controls_http.rs
  - portal-rs/crates/admin/src/control/controls_tests.rs
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/integration.rs
  - portal-rs/crates/admin/src/control/integration_tests.rs
  - portal-rs/crates/admin/src/control/mod.rs
  - portal-rs/crates/admin/src/control/store.rs
  - portal-rs/crates/admin/src/lib.rs
  - portal-rs/crates/identity/src/mfa.rs
  - portal-rs/crates/identity/src/mfa_tests.rs
validation:
  - command: cargo clippy --locked --workspace --all-targets -- -D warnings
    result: PASS
    evidence: full workspace completed successfully
  - command: cargo fmt --all --check
    result: PASS
    evidence: no formatting differences
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no enforced token count increased
  - command: cargo test --locked --workspace
    result: PASS
    evidence: 950 passed, zero failed, three pre-existing browser-only ignores; all 77 unit/integration/doc suites succeeded against PostgreSQL 18.6 through PORTAL_TEST_DATABASE_URL
blockers:
  - none
next_action: Owner reviews the retained codex/portal-rs-adm-p2b-int branch; no PR is requested under this task
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Codex lane integration branch published without a PR under the owner's instructions; its work reached main through later portal-rs PRs and the branch belongs to the Codex lane, so this session does not delete it
source_branch_evidence: live branch lookup on 2026-11-11 shows refs/heads/codex/portal-rs-adm-p2b-int present; archived 2026-11-11 after the owner made portal-rs the main platform
```

## Implementation

- Identity consumes a fresh TOTP counter using its existing encrypted-secret and replay window implementation. A read-only transaction verifier retains the account lock through domain/audit commit and reuses grant expiry, active/verified account, generation and credential checks.
- Server-side step-up proofs bind the current MFA grant and staff account to one request UUID, expire after five minutes, and cannot authorize another request. Repeated evidence attachment is refused under the request row lock.
- ControlEvidence accepts only stored request/preview identifiers. It derives policy input from locked request rows, validates normalized approvals and exact signed canonical bytes, and appends ADM-P2C structured audit and evidence atomically. R3/R4 delivery waits for committed evidence.
- Successful stored dry-run results supply preview material. Preview digests are SHA-256 over canonical dry_run_preview bytes. Dry-run and execution have distinct store idempotency IDs; their operation/version, kind, target and parameters must match. An approval UUID can attach to only one execution through the existing admin_approvals primary key.
- Public crate-root control paths remain aliases of the nested modules. Flags remain off by default. No migration, external Game connection or PR is introduced.
- Existing policy regression helpers now live only in the test module; the production request-builder boundary is removed. The form regression additionally checks the stored-request path.

## Validation environment and delivery

The final workspace command used a new disposable local database on PostgreSQL 18.6 (server_version_num 180006), with max_locks_per_transaction raised from 64 to 512 for concurrent full-schema cleanup. The shared Laravel fixture database was preserved. No test concurrency or assertion was reduced, and no new ignore was introduced. The default suite's three existing Chromium-dependent ignores are the payments account layout, store browser layout and web search accessibility cases.

Final source checks: cargo fmt --all --check; cargo clippy --locked --workspace --all-targets -- -D warnings; cargo test --locked --workspace. The admin suite passed 93 tests; identity passed 59 unit tests plus 12 registration integration tests. Five new security regression tests cover the real provider, stale/foreign/replayed proofs, self/expired approvals, transactional rollback and read-only grant locking.

The source branch is retained for the task requester, with no PR or integration action. Active write ownership is released after publication. Changed files are enumerated in the checkpoint above.
