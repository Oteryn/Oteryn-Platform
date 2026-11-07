---
task_id: OTERYN-20261107-portal-rs-admin-analytics
governing_issue: UNKNOWN
required_reads:
  - docs/contracts/OTERYN_V2_ADMIN_ANALYTICS_CONTRACT_PROPOSAL.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
search_first:
  - live Platform Issues matching T10b and analytics
  - live PR 1768 and open PR ownership
optional_reads: []
---

# T10b admin analytics consumer

Owner task: implement accepted admin-analytics revision 2 on current origin/main,
push `codex/portal-rs-admin-analytics`, no PR, workflow changes or deployment.
Live Issue searches did not identify an allocated implementation Issue; PR 1768
is merged contract acceptance, not the implementation lifecycle. Do not invent an
Issue mapping or reopen that PR. Jira synchronization remains unknown/pending.

## Acceptance criteria

- Strict purpose-isolated ingest, transactional commits and durable fences.
- Separate admin schema, retention, audit-before-read, current MFA/RBAC.
- Five localized panel pages; presentation requires an accepted resync baseline.
- Platform-owned byte-exact fixtures and PostgreSQL/browser qualification.
- Preserve excluded concurrent paths and leave both switches off.

## Ownership

```yaml
owned_paths:
  - portal-rs/crates/admin/src/analytics/**
  - portal-rs/crates/admin/src/rbac/**
  - portal-rs/crates/admin/src/web.rs
  - portal-rs/crates/admin/src/lib.rs
  - portal-rs/crates/admin/Cargo.toml
  - portal-rs/crates/admin/README.md
  - portal-rs/crates/admin/tests/fixtures/admin_analytics/**
  - portal-rs/templates/admin/analytics.html
  - portal-rs/migrations/20261107140001_admin_analytics.sql
  - portal-rs/migrations/rollback/20261107140001_admin_analytics.sql
  - portal-rs/e2e/specs/admin-analytics.spec.mjs
  - portal-rs/bin/oteryn-portal/src/main.rs
modules:
  - admin
dependencies:
  - accepted admin analytics contract revision 2
blockers:
  - production activation waits for contract sections 8 and 10.6-10.8
cross_repository_tasks:
  - none; Game was not accessed
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-07T15:37:54Z
head: UNKNOWN
branch: codex/portal-rs-admin-analytics
pr: none
status: ready
context_routes:
  - admin-rbac
  - database
  - testing
owned_paths:
  - portal-rs/crates/admin/**
  - portal-rs/templates/admin/analytics.html
  - portal-rs/migrations/20261107140001_admin_analytics.sql
  - portal-rs/migrations/rollback/20261107140001_admin_analytics.sql
  - portal-rs/e2e/specs/admin-analytics.spec.mjs
  - portal-rs/bin/oteryn-portal/src/main.rs
proven:
  - contract accepted revision 2 including section 10
  - final frozen material SHA-256 54b2f74d0e00fc65aa22b4b829902b351b5a4c54a475415109ba0b584cae68ff
  - final fmt and workspace Clippy pass with no warnings
  - final full workspace tests pass with 1107 passed and 3 ignored browser tests
  - four Playwright panel cases pass for pl/en at 1440px and 390px
  - explicit migration rollback and reapply both pass
  - PostgreSQL 18.6 configured with max_locks_per_transaction=1024 and max_connections=300
derived:
  - baseline cannot be established by any of the three accepted operations
unknown:
  - allocated implementation Issue and Jira mapping
  - Game resync manifest envelope and production qualification
conflicts: []
first_failure:
  marker: none
  evidence: no unresolved focused-test failure
rejected_hypotheses:
  - a fresh watermark alone certifies a rebuilt database
changed_paths:
  - portal-rs/crates/admin/src/analytics/**
  - portal-rs/crates/admin/src/rbac/**
  - portal-rs/templates/admin/analytics.html
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: final exit 0; no output
  - command: cargo clippy --locked --workspace --all-targets -- -D warnings
    result: PASS
    evidence: final exit 0; no warnings
  - command: cargo test --locked --no-fail-fast
    result: PASS
    evidence: final exit 0; 1107 passed, 0 failed, 3 ignored, 83 targets; PORTAL_TEST_DATABASE_URL on PostgreSQL 18.6
  - command: cargo test --locked -p oteryn-admin analytics::tests -- --nocapture
    result: PASS
    evidence: six tests; PostgreSQL exercised retry/conflict after epoch fencing and unknown-policy higher epoch
  - command: node run.mjs <private-task-env.json> test specs/admin-analytics.spec.mjs
    result: PASS
    evidence: four cases, pl/en and desktop/mobile; real registration, MFA, ten audited page/JSON reads per actor, no export route
  - command: migrate then explicit inverse then forward SQL on disposable database
    result: PASS
    evidence: all three exited 0; production data was not used

blockers:
  - baseline activation requires accepted envelope and retained-set qualification under contract 10.6
next_action: hand off the requested branch for owner review; do not create a PR or deploy
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner explicitly requests a reviewable branch without a PR; owner retains publication control
source_branch_evidence: current owner task
```

## Delivery evidence and activation boundary

Final material changes are confined to the admin analytics module, RBAC catalog,
guard/storage/tests, admin route wiring, the private migration and explicit inverse,
the panel template, 66 byte-exact consumer fixtures, E2E spec and crate READMEs.
The complete changed-file list is available in the branch diff. Excluded concurrent
paths and `.github/workflows` have no changes. No Game repository was accessed.
No PR, merge or deployment was requested or performed.

The three ignored tests are existing optional Chromium tests in Identity account,
Store premium and federated search, requiring `PORTAL_BROWSER_MODULES`; the new
panel has separate real Playwright coverage. Intermediate runs against the
pre-existing shared test database and superseded build candidates are not final
qualification evidence. Final qualification used a task-owned database on
PostgreSQL 18.6 with `max_locks_per_transaction=1024`, `max_connections=300`.

Remaining activation gates are contract §§5.3/10.6 (accepted Game resync envelope,
baseline coverage/absence proof and recovery SLA), §§8/10.7–10.8 (accepted producer,
shared producer/consumer fixture matrix, retained-set replay/capacity, PKI,
clock/freshness, deletion/restore and retention/backup/access-audit procedures,
exact-revision qualification and owner deployment authority). Presentation cannot
bypass the baseline gate. Export remains absent under §10.4. Both switches remain
default OFF.

Task-owned test databases and private E2E scratch are removed after qualification;
reusable pre-existing Rust/browser caches and dependency installations are retained.
The requested branch is intentionally retained for the owner's review. No GitHub
Issue/Jira mapping was invented; that lifecycle mapping remains coordinator-owned.
