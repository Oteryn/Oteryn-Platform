---
task_id: OTERYN-20261105-portal-rs-pkg9
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261105-portal-rs-pkg9

## Goal

Assemble package 9 from the package 6 branch, following the owner’s ordered merge instructions. Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530 (open). Base PR: https://github.com/Oteryn/Oteryn-Platform/pull/1733 (open). Publish the package branch normally; do not create a PR.

## Acceptance criteria

- Ordered non-fast-forward merges of T30, T08b, T34, T34b and T50.
- Preserve module behavior and complete shared declarations and registrations.
- Unique migration versions; strict formatting, clippy, locked PostgreSQL 18.6 tests and token ratchet.
- No workflow changes, secrets or force push.

## Ownership

```yaml
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261105-portal-rs-pkg9.md
modules:
  - package-9-integration
dependencies:
  - package-6
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-05T18:04:47Z
head: 06103fdd
branch: codex/portal-rs-pkg9
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/**
proven:
  - All five requested branches retained in order with two-parent merge commits.
  - All 49 migration versions are unique; no renumbering required.
  - No SQLx offline cache exists.
derived: []
unknown: []
conflicts:
  - Shared manifests and registrations resolved additively as detailed below.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/**
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Final material candidate 06103fdd.
  - command: cargo clippy --locked --all-targets -- -D warnings
    result: PASS
    evidence: All targets passed with warnings denied on final material candidate 06103fdd.
  - command: cargo test --locked
    result: PASS
    evidence: PostgreSQL 18.6; 524 reported passing, 0 failed, 1 ignored; 318 tests actually used PostgreSQL.
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: Printed ratchet ok.
blockers:
  - none
next_action: Publish normally and retain the package branch for downstream integration.
```

## Conflict resolution ledger

Paths are relative to `portal-rs/`.

| Merge | Conflicted files | Resolution |
| --- | --- | --- |
| T30 | None | Shared store module declarations merged automatically. |
| T08b | Cargo.lock; bin/oteryn-portal/Cargo.toml; bin/oteryn-portal/src/main.rs; bin/oteryn-portal/src/worker.rs; crates/identity/Cargo.toml; crates/identity/src/lib.rs | Retained all existing dependencies, routes, worker cleanup and identity modules; added email-change router, worker, module and wallet dependency. Kept equivalent sha2, notify, Fluent and language dependencies once; kept the existing language macro feature. Mounted login once. Preserved full table bodies and call delimiters. |
| T34 | Cargo.lock; crates/support/Cargo.toml; crates/support/src/lib.rs | Kept tickets and sanctions modules; retained dependencies and added gameauth. Unioned SQLx json and Tokio time features with existing feature sets; kept compatible shared version ranges once. |
| T34b | Cargo.lock; crates/support/Cargo.toml; crates/support/src/lib.rs | Kept appeals, sanctions and tickets modules. Existing dependency ranges and features already cover the incoming requirements; retained each shared dependency once. |
| T50 | bin/oteryn-portal/src/main.rs | Retained all previous routes and added commerce router, with every call complete. Worker registration merged automatically. |

For each conflicted Cargo.lock, selected the current package base side and regenerated using `cargo metadata --format-version 1`; no lockfile hand edits. Ran metadata after the final clean lockfile merge as well. Formatting only reorders module declarations. No modules dropped and no behavior redesigned.

PostgreSQL exposed a semantic shared-schema conflict: T30 attempted to create and seed `character_restore_locks`, already supplied by the earlier T23 migration. Removed only that duplicate DDL from the new T30 migration, preserving the earlier migration untouched. T30 expects an integer result while the existing lock key is smallint, so its two lock queries explicitly cast the selected key to integer. Both modules retain the same row, lock modes and transaction order. This is shared-schema conflict resolution, without a new lock design. Initial acceptance failed during migration with PostgreSQL error 42P07 before this resolution.

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Codex lane integration branch published without a PR under the owner's instructions; its work reached main through later portal-rs PRs and the branch belongs to the Codex lane, so this session does not delete it
source_branch_evidence: live branch lookup on 2026-11-11 shows refs/heads/codex/portal-rs-pkg9 present; archived 2026-11-11 after the owner made portal-rs the main platform
```

## Notes

Jira mapping is unknown; synchronization is pending for the programme coordinator. No external messages sent. Product runtime E2E is not applicable to this assembly task; the requested workspace tests cover the retained module behavior.


## Final acceptance summary

Final material candidate: `06103fdd`. Executed the requested acceptance chain with the local PostgreSQL 18.6 URL set transiently; no credentials printed or committed:

```text
cd portal-rs && cargo fmt --all --check && cargo clippy --locked --all-targets -- -D warnings && PORTAL_TEST_DATABASE_URL=<local PG 18.6 URL> cargo test --locked
PASS: formatting
PASS: strict clippy, all targets
PASS: cargo test — 524 reported passing, 0 failed, 1 ignored
PostgreSQL-backed tests actually run: 318
python3 tools/validation/legacy_token_ratchet.py
ratchet ok
```

The database count matches successful test names to their actual PostgreSQL fixture paths, including ten macro-generated online tests and the live readiness query. It excludes the explicitly ignored browser-layout test. The optional pinned content import and asset smoke test both returned early: each was checked directly with the built executable and output capture disabled, finished in 0.00s and printed an explicit skip notice. Neither is counted as executed PostgreSQL coverage. All 318 database inventory entries matched successful test results with the URL supplied.

All 49 migration versions are unique. No migration renumbered. No SQLx offline cache. No workflow changes. No PR created. The final source comparison with each module branch found only the two documented T30 shared-lock integrations changed imported module files; other imported files are byte-identical. Shared declarations and registrations were composed additively. All five modules retained, none dropped. The final evidence-only commit preserves the tested material tree.
