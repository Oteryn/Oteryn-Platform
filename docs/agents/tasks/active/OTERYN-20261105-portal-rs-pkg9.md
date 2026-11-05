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
updated_at: 2026-11-05T00:00:00Z
head: 836a6352
branch: codex/portal-rs-pkg9
pr: none
status: validating
context_routes: []
owned_paths:
  - portal-rs/**
proven:
  - All five requested branches retained in order with two-parent merge commits.
  - All 49 migration versions are unique; no renumbering required.
  - No SQLx offline cache exists.
derived: []
unknown:
  - Final acceptance results.
conflicts:
  - Shared manifests and registrations resolved additively as detailed below.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/**
validation:
  - command: requested acceptance chain
    result: NOT_RUN
    evidence: Formatting passed; clippy in progress.
blockers:
  - none
next_action: Complete strict clippy and PostgreSQL-backed acceptance tests.
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

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requests the package branch for subsequent integration without a PR.
source_branch_evidence: Explicit owner instruction in this integration task.
```

## Notes

Jira mapping is unknown; synchronization is pending for the programme coordinator. No external messages sent. Product runtime E2E is not applicable to this assembly task; the requested workspace tests cover the retained module behavior.
