---
task_id: OTERYN-20261117-world-fence
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-world-fence

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, class B, world crate. The staff world panel records a refused
request in the audit log (`admin::authorize`, denial branch) on a plain
transaction with no lock budget, so a held lock on `audit_log` (a migration
or index build in production) made the request wait without bound. It now
arms `mutation_budget` right after `begin` and answers unavailable (503).
The panel mutation (`admin::mutate`) already armed the budget inside
`authorize_mutation_with_resources`; it now also arms it at `begin`, with no
change to lock order. A crate guard requires every `begin()` to arm a budget
or be listed with its reason. The native catalog intake and publication
(serve background refresh) and the world import publish and rollback (no
production caller) are operator publish/import, out of class B scope by
coordinator decision; the catalog cache reads are lock-free.

## Acceptance criteria

- [x] A held audit table lock makes the denied staff request answer unavailable within the budget (red without the change)
- [x] Crate guard: every begin() is budgeted or listed with a reason (red without the change)
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-world-fence.md
  - portal-rs/crates/world/Cargo.toml
  - portal-rs/crates/world/src/admin.rs
  - portal-rs/crates/world/src/begin_guard_tests.rs
  - portal-rs/crates/world/src/last_wait_tests.rs
  - portal-rs/crates/world/src/lib.rs
modules:
  - world
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
updated_at: 2026-11-17T12:00:00Z
head: a20b2e81bd6163ca14d751613c329c12ad2ca268
branch: claude/happy-davinci-ijz3wy-world-fence
pr: 1914
status: completed
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-world-fence.md
  - portal-rs/crates/world/Cargo.toml
  - portal-rs/crates/world/src/admin.rs
  - portal-rs/crates/world/src/begin_guard_tests.rs
  - portal-rs/crates/world/src/last_wait_tests.rs
  - portal-rs/crates/world/src/lib.rs
proven:
  - No migration, contract, fixture, flag or wire byte change; Cargo.lock unchanged (oteryn-db was already a dev-dependency)
  - The native catalog refresh is spawned only by the serve role at startup (bin/oteryn-portal/src/main.rs refresh_in_background), never by a request
  - WorldStore import_directory, import_configured_directory and rollback have no production caller
  - A budget timeout (55P03, 57014) in admin::authorize and admin::mutate maps to 503 through the existing unavailable mapping
derived:
  - In admin::mutate a statement timeout during the final check_in recheck maps to 403 (existing mapping, budget was already armed there before this change)
  - get_entity_locked (FOR SHARE) runs in transactions begun by the minigames crate, outside this crate's guard
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-world-fence.md
  - portal-rs/crates/world/Cargo.toml
  - portal-rs/crates/world/src/admin.rs
  - portal-rs/crates/world/src/begin_guard_tests.rs
  - portal-rs/crates/world/src/last_wait_tests.rs
  - portal-rs/crates/world/src/lib.rs
validation:
  - command: cargo test --locked -p oteryn-world --lib postgres_denied_audit (PostgreSQL 18), without and with the change
    result: PASS
    evidence: without the change the denied request stalls past the 10 s bound; with it the request answers 503 within 4 s and writes no audit row
  - command: cargo test --locked -p oteryn-world --lib begin_guard, with an empty exempt list
    result: PASS
    evidence: red inventory admin.rs 2, catalog/cache.rs 2, catalog/mod.rs 1, catalog/publication.rs 2, store.rs 2; green after arming admin.rs and listing the rest
  - command: cargo test --locked -p oteryn-world (PostgreSQL 18); cargo fmt --all -- --check; cargo clippy --locked -p oteryn-world --all-targets -- -D warnings
    result: PASS
    evidence: lib 45, catalog 1, catalog_refresh 11, import_validation 1, postgres 10 pass, no SKIPPED; fmt and clippy exit 0
blockers:
  - none
next_action: none; delivered by PR 1914 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1914 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1914 merged as a20b2e81bd6163ca14d751613c329c12ad2ca268
```
