---
task_id: OTERYN-20261117-stats-fence
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-stats-fence

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, class B, stats crate. The public world total and the owner online
ranking took the projection fence (`FOR SHARE` on `public_projection_state`
in `online::storage::fresh`) on a plain transaction with no lock budget.
Ingest holds that row `FOR UPDATE`, so a request waited for the ingest without
bound. Both now begin through `online::storage::read_fence`, which sets
`lock_timeout` 1 s and `statement_timeout` 2.5 s, and answer unavailable. The
watch subscribe and list paths (mounted in production) took a per-owner
advisory lock or ran policy reads before any budget; they now arm
`mutation_budget` right after `begin`. A crate guard requires every `begin()`
to arm a budget or be listed with its reason.

## Acceptance criteria

- [x] A held projection lock makes the world total answer unavailable within the budget (red without the change)
- [x] Crate guard: every begin() is budgeted or listed with a reason (red without the change)
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-stats-fence.md
  - portal-rs/crates/stats/src/begin_guard_tests.rs
  - portal-rs/crates/stats/src/lib.rs
  - portal-rs/crates/stats/src/online.rs
  - portal-rs/crates/stats/src/online/storage.rs
  - portal-rs/crates/stats/src/rankings_online.rs
  - portal-rs/crates/stats/src/watch.rs
  - portal-rs/crates/stats/tests/online.rs
modules:
  - stats
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
updated_at: 2026-11-17T07:20:00Z
head: 8633994271bcca625c2c7a9b1186918553e2770a
branch: claude/happy-davinci-ijz3wy-stats-fence
pr: 1911
status: validating
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-stats-fence.md
  - portal-rs/crates/stats/src/begin_guard_tests.rs
  - portal-rs/crates/stats/src/lib.rs
  - portal-rs/crates/stats/src/online.rs
  - portal-rs/crates/stats/src/online/storage.rs
  - portal-rs/crates/stats/src/rankings_online.rs
  - portal-rs/crates/stats/src/watch.rs
  - portal-rs/crates/stats/tests/online.rs
proven:
  - No migration, contract, fixture, flag or wire byte change
  - Game ingest apply, the daily purge, the background workers (advisory lock 65001) and watch dispatch (no production caller) are listed in the guard with their reasons
  - Every FOR SHARE, FOR UPDATE and advisory lock in non-test stats sources sits inside a budgeted or listed begin site
derived:
  - The owner ranking path is covered by the crate guard only; the behavioural red/green test drives world_total
  - The owner ranking query is now capped at 2.5 s per statement, a new unavailable path independent of locks
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-stats-fence.md
  - portal-rs/crates/stats/src/begin_guard_tests.rs
  - portal-rs/crates/stats/src/lib.rs
  - portal-rs/crates/stats/src/online.rs
  - portal-rs/crates/stats/src/online/storage.rs
  - portal-rs/crates/stats/src/rankings_online.rs
  - portal-rs/crates/stats/src/watch.rs
  - portal-rs/crates/stats/tests/online.rs
validation:
  - command: cargo test --locked -p oteryn-stats --test online postgres_public_world_totals (PostgreSQL 18), without and with the change
    result: PASS
    evidence: without the change the world total stalls past the 10 s bound; with it the request answers unavailable within 3 s
  - command: cargo test --locked -p oteryn-stats --lib begin_guard, without and with the rankings_online change
    result: PASS
    evidence: without it the guard lists rankings_online.rs (and, on main, watch.rs subscribe and list) as unbudgeted; with it the guard passes
  - command: cargo test --locked -p oteryn-stats (PostgreSQL 18); cargo fmt -p oteryn-stats --check; cargo clippy --locked -p oteryn-stats --all-targets -- -D warnings
    result: PASS
    evidence: lib 41, online 14 and the other test targets pass; fmt and clippy exit 0
blockers:
  - none
next_action: "open the PR, set pr, wait for Portal RS CI, then squash auto-merge"
```
