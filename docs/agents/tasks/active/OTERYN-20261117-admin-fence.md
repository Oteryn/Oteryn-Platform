---
task_id: OTERYN-20261117-admin-fence
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-admin-fence

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, class B, admin crate. Every `begin()` in non-test admin sources now
arms `mutation_budget` (3 s lock and statement budget) on the line after
`begin`, or is listed in a crate guard with its reason. The admin-control
agent routes and store (`control/http.rs`, `control/store.rs`), the R4
control integration (`control/integration.rs`) and players attest took an
advisory or row lock before any budget, so a held lock stalled the request
and its pooled connection without bound; they now answer unavailable. The
RBAC-helper sites (bazaar, commerce cancel, console stored query, role
change and revoke, player mutations) arm the budget at `begin` before the
helper. No lock order, wire body, migration, contract, fixture or flag
changes.

## Acceptance criteria

- [x] A held agent row makes `claim_pending` answer unavailable within the budget (red without the change: 10 s bound elapsed)
- [x] Crate guard: every begin() is budgeted or listed with a reason (red before the change: 34 sites in 15 files)
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-admin-fence.md
  - portal-rs/crates/admin/src/begin_guard_tests.rs
  - portal-rs/crates/admin/src/lib.rs
  - portal-rs/crates/admin/src/bazaar.rs
  - portal-rs/crates/admin/src/commerce/http.rs
  - portal-rs/crates/admin/src/console/stored.rs
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/integration.rs
  - portal-rs/crates/admin/src/control/store.rs
  - portal-rs/crates/admin/src/control/tests.rs
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/web.rs
modules:
  - admin
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
updated_at: 2026-11-17T08:10:00Z
head: 6fb8ad647cd773eda152052d2a77585f7cfe3035
branch: claude/happy-davinci-ijz3wy-admin-fence
pr: 1913
status: validating
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-admin-fence.md
  - portal-rs/crates/admin/src/begin_guard_tests.rs
  - portal-rs/crates/admin/src/lib.rs
  - portal-rs/crates/admin/src/bazaar.rs
  - portal-rs/crates/admin/src/commerce/http.rs
  - portal-rs/crates/admin/src/console/stored.rs
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/integration.rs
  - portal-rs/crates/admin/src/control/store.rs
  - portal-rs/crates/admin/src/control/tests.rs
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/web.rs
proven:
  - Base is origin/main 6fb8ad64; changes are uncommitted in the worktree
  - src/test_staff.rs is declared under cfg(test) in lib.rs and is not compiled into non-test builds
  - sqlx 55P03 and 57014 reach each module's unavailable path (agent routes keep the existing 503 ADM_INTERNAL body through store::Error::Storage)
  - Listed sites are audit_log INSERTs (no FK, unique index or insert trigger), budgeted reads (statement_timeout 3s first), a READ ONLY snapshot, the admin.control.expire worker (SKIP LOCKED) and the commerce.refund outbox job that spans the provider call
derived:
  - mutation_budget is armed once per transaction, so each lock wait is bounded at about 3 s, not the whole multi-lock handler
unknown:
  - Coordinator Jira synchronization
conflicts:
  - Stale active packet OTERYN-20261115-a2-01b-admin lists admin paths (status ready, pr none)
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-admin-fence.md
  - portal-rs/crates/admin/src/begin_guard_tests.rs
  - portal-rs/crates/admin/src/lib.rs
  - portal-rs/crates/admin/src/bazaar.rs
  - portal-rs/crates/admin/src/commerce/http.rs
  - portal-rs/crates/admin/src/console/stored.rs
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/integration.rs
  - portal-rs/crates/admin/src/control/store.rs
  - portal-rs/crates/admin/src/control/tests.rs
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/web.rs
validation:
  - command: cargo test -p oteryn-admin --lib pg_claim_answers (PostgreSQL 18), without and with the change
    result: PASS
    evidence: without the change the claim exceeds the 10 s bound; with it the claim returns Storage within 5 s
  - command: cargo test -p oteryn-admin --lib begin_guard, before and after
    result: PASS
    evidence: before, 34 unbudgeted begin() sites in 15 files; after, the guard passes
  - command: cargo test --locked -p oteryn-admin (PostgreSQL 18); cargo fmt --all -- --check; cargo clippy --locked -p oteryn-admin --all-targets -- -D warnings
    result: PASS
    evidence: lib 182 passed 0 failed; fmt and clippy exit 0
blockers:
  - none
next_action: "commit, open the PR, set pr, wait for Portal RS CI"
```
