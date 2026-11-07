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
  - portal-rs/migrations/20261107140001_admin_analytics.up.sql
  - portal-rs/migrations/20261107140001_admin_analytics.down.sql
  - portal-rs/crates/content/src/announcements_tests.rs
  - portal-rs/crates/content/src/events/tests.rs
  - portal-rs/crates/content/src/media_tests.rs
  - portal-rs/crates/content/src/news_tests.rs
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
updated_at: 2026-11-07T16:13:30Z
head: 1f9326444049c66840e3e295d4b9ce9091279403
branch: claude/portal-rs-admin-analytics
pr: none
status: ready
context_routes:
  - admin-rbac
  - database
  - testing
owned_paths:
  - portal-rs/crates/admin/**
  - portal-rs/templates/admin/analytics.html
  - portal-rs/migrations/20261107140001_admin_analytics.up.sql
  - portal-rs/migrations/20261107140001_admin_analytics.down.sql
  - portal-rs/crates/content/src/announcements_tests.rs
  - portal-rs/crates/content/src/events/tests.rs
  - portal-rs/crates/content/src/media_tests.rs
  - portal-rs/crates/content/src/news_tests.rs
  - portal-rs/e2e/specs/admin-analytics.spec.mjs
  - portal-rs/bin/oteryn-portal/src/main.rs
proven:
  - contract accepted revision 2 including section 10
  - Update-2 material diff SHA-256 f5045b5d08fdfbb6c1fceeb3439a43763cc85c759c6f9402bb8c674fbde890fa
  - final fmt and workspace Clippy pass with no warnings
  - final full workspace tests pass with 1110 passed and 3 ignored browser tests
  - four Playwright panel cases pass for pl/en at 1440px and 390px
  - explicit .down.sql rollback and .up.sql reapply both pass
  - wrong-host, duplicate-Host and disabled-panel/presentation requests return 404 without audit writes
  - rate, privacy-policy, dependency and capacity refusals preserve epoch and active generation
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
    evidence: rerun exit 0 with RUST_TEST_THREADS=4; 1110 passed, 0 failed, 3 ignored, 83 targets; PORTAL_TEST_DATABASE_URL on PostgreSQL 18.6
  - command: cargo test --locked -p oteryn-admin analytics::tests -- --nocapture
    result: PASS
    evidence: nine tests; PostgreSQL host/denial audits, refused-message fence preservation and explicit down/up rollback/reapply all executed
  - command: node run.mjs <private-task-env.json> test specs/admin-analytics.spec.mjs
    result: PASS
    evidence: four cases, pl/en and desktop/mobile; real registration/MFA; private cache, HSTS, nosniff and referrer headers; ten audited reads per actor; no export route
  - command: postgres_reversible_analytics_migration_preserves_role_catalog
    result: PASS
    evidence: PASS in focused PostgreSQL tests and full suite; actual .down.sql and .up.sql files executed on a disposable database

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


## Update 2 — bounded review fixes

Owner review of `1f932644` requests P1–P3 only, on the same branch without a PR,
workflow changes or deployment. P1 reuses the existing panel host/security-header
boundary outside session/audit, including presentation-off 404 and staff-only
denial audit writes. P2 rolls back fence/active-generation changes for rate,
privacy-policy, dependency and capacity refusals. P3 uses the established
`.up.sql`/`.down.sql` pair and skips down scripts in direct-SQL integration
fixtures, as the T45 migration-version test already does. The explicit
rollback/reapply regression executes the actual down and up files.

Activation remains blocked on the contract resync envelope/baseline (§§5.3,
10.6). No fence row is created until then. Production startup and all three
operations continue to fail closed when that evidence is missing; test-only
fixture rows do not establish activation authority.

Update-2 validation is complete:

- `cargo fmt --all --check`: PASS, exit 0.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: PASS, exit 0.
- `cargo test --locked --no-fail-fast`: final PASS, exit 0, 1110 passed and
  3 existing optional Chromium tests ignored, 83 targets, PostgreSQL 18.6 with
  `max_locks_per_transaction=1024`, `max_connections=300`, `RUST_TEST_THREADS=4`.
- The initial default-parallelism full run exited 101 on the existing
  `credentials_http::tests::postgres_sudo_rotation_csrf_refusal_and_expiry`
  assertion (302 versus expected 429). That test passed in isolation without
  code changes, then the complete suite passed with four harness threads.
  Credentials/Identity source was not changed for this review.
- Panel Playwright spec: PASS, 4/4 (pl/en, 1440px/390px), with presentation ON,
  ingestion OFF, synthetic authority/privacy and no fence or baseline.
- Actual `.down.sql` rollback and `.up.sql` reapply: PASS in focused and full
  PostgreSQL tests; both SQL files preserve their original byte checksums.
- Checkpoint validation, legacy-token ratchet and full diff checks: PASS.

The only extra files outside Admin are the four Content migration fixture tests,
the existing E2E spec, migration renames and this packet; fixture edits only skip
down migrations. Excluded concurrent paths and workflows remain untouched.
Task-owned test databases/private E2E scratch are removed after qualification;
reusable pre-existing tool/dependency caches are retained.
