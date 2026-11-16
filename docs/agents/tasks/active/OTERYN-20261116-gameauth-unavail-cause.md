---
task_id: OTERYN-20261116-gameauth-unavail-cause
governing_issue: 1889
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261116-gameauth-unavail-cause

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1889
(open at task admission), under programme Issue #536.

A bare `KeyError::Unavailable` during the wave2_01 gameauth setup at load 30
could not be traced to its source. Every signing-key and witness `Unavailable`
now carries a `Cause` (the constructing source line and a closed `CauseKind`)
without strings, paths, payloads or secret values, and without any change to
retries, timeouts, HTTP status, wire bodies, audit or journal.

## Acceptance criteria

- [x] `KeyError::Unavailable` holds a `Cause`; `keys/trust.rs` and `keys/storage.rs` build it at every site with its exact kind
- [x] Trust snapshot issuer, profile or purpose mismatch reports `CauseKind::Mismatch`; `Stale` means only an aged or future snapshot
- [x] Public evidence lock, floor and advance still answer the byte-exact 503 body
- [x] PostgreSQL tests pin `PoolTimeout`, `Sqlstate("57014")` and `LockBusy` at their `keys/trust.rs` sites

## Ownership

```yaml
owned_paths:
  - portal-rs/crates/gameauth/
  - docs/agents/tasks/active/OTERYN-20261116-gameauth-unavail-cause.md
modules:
  - gameauth
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
updated_at: 2026-11-16T22:55:00Z
head: 3fdc71224fab83fa22f4b8a00a0c64e764bb7468
branch: claude/happy-davinci-ijz3wy-unavail-cause
pr: 1891
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/gameauth/
  - docs/agents/tasks/active/OTERYN-20261116-gameauth-unavail-cause.md
proven:
  - Base is origin/main 85336fc6be4145028864693a8e5dadd78c87c3f2
  - Code changes stay in five files under portal-rs/crates/gameauth/src
  - No migration, contract, fixture, flag or wire byte change
  - No Game repository, production service or production data accessed
  - Checks ran on packet commit 2f35cbed whose portal-rs tree equals 3fdc7122 and this head
  - Full gameauth lib suite 165 passed 0 failed 0 ignored with PostgreSQL and no SKIPPED line
  - Advisory statement-timeout test 20 of 20 passed while the full suite ran concurrently
derived: []
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/gameauth/src/evidence.rs
  - portal-rs/crates/gameauth/src/keys/mod.rs
  - portal-rs/crates/gameauth/src/keys/storage.rs
  - portal-rs/crates/gameauth/src/keys/tests.rs
  - portal-rs/crates/gameauth/src/keys/trust.rs
  - docs/agents/tasks/active/OTERYN-20261116-gameauth-unavail-cause.md
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: exit 0, repository-pinned Rust 1.97
  - command: cargo clippy --locked --all-targets -- -D warnings
    result: PASS
    evidence: exit 0 on the whole workspace after a full rebuild
  - command: cargo test --locked -p oteryn-gameauth --lib seed
    result: PASS
    evidence: 2 passed, 0 failed, 163 filtered out
  - command: cargo test --locked -p oteryn-gameauth --lib --no-fail-fast
    result: PASS
    evidence: 165 passed, 0 failed, 0 ignored, 94.54 s, PostgreSQL enabled, no SKIPPED line
  - command: 20 runs of keys::tests::unavailable_cause_advisory_key_held_is_statement_timeout concurrent with the full suite
    result: PASS
    evidence: 20 of 20 exit 0, 1 passed each, no SKIPPED line
  - command: python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint
    result: PASS
    evidence: 26 task checkpoints validated
  - command: python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: 24 prompts and 3 handovers validated
  - command: python3 tools/agents/policy_consistency.py
    result: BLOCKED
    evidence: no local META checkout; same refusal on main b4ebbbf1; CI agent-governance runs it
blockers:
  - none
next_action: Coordinator pushes the branch and opens the PR for #1889
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active; branch not yet pushed
source_branch_evidence: pending
```

## Notes

The coordinator owns push, PR and Jira synchronization.
