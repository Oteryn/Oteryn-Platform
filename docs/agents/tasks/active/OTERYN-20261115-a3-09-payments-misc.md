---
task_id: OTERYN-20261115-a3-09-payments-misc
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261115-a3-09-payments-misc

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/536 (open, verified).
Owner task: fix W3-01 publication authorization revocation race and W3-02 unauthenticated character budget allocation.
Base: 90234d8d2d0a51a654b8ff6947d38372a740acba.
Branch: claude/happy-davinci-ijz3wy-fix-a3-09-payments-misc.
No PR requested; coordinator handles integration. Single writer, no subagents.

## Ownership

Owned: portal-rs/crates/downloads and portal-rs/crates/player-api.
The parallel task's admin/RBAC files remain read-only.
No migration or contract changes planned.

## Acceptance

- Real PostgreSQL regressions fail on base and pass with fixes.
- Publication holds account and shared RBAC fence locks through commit.
- Invalid character-list bearers return 401 without budget allocation.
- Formatting, crate Clippy and crate tests pass.
- Commit with owner-specified author/trailers and push only the requested branch.

## Evidence

Validation complete: 28 passed, 0 failed, 0 skipped/ignored on Rust 1.97 with real PostgreSQL. Jira mapping/synchronization unknown; worker makes no Jira writes.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T21:22:07Z
head: 90234d8d2d0a51a654b8ff6947d38372a740acba
branch: claude/happy-davinci-ijz3wy-fix-a3-09-payments-misc
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/downloads
  - portal-rs/crates/player-api
proven:
  - W3-01 regression failed on base when revoke committed during paused publication (2.27s).
  - W3-02 regression failed on base with 128 unauthorized requests allocating 128 budgets (3.03s).
  - Rust 1.97 formatting and all-target Clippy pass for both touched crates.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: expected baseline regressions reproduced; final validation passes
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/downloads/src/command.rs
  - portal-rs/crates/downloads/tests/command.rs
  - portal-rs/crates/player-api/src/lib.rs
  - portal-rs/crates/player-api/tests/character_budget.rs
  - docs/agents/tasks/active/OTERYN-20261115-a3-09-payments-misc.md
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Rust 1.97, exit 0
  - command: cargo clippy --locked --all-targets -p oteryn-downloads -p oteryn-player-api -- -D warnings
    result: PASS
    evidence: Rust 1.97, exit 0
  - command: cargo test --locked -p oteryn-downloads -p oteryn-player-api -- --nocapture --test-threads=1
    result: PASS
    evidence: Rust 1.97 real PostgreSQL; 28 passed, 0 failed, 0 skipped/ignored
blockers: []
next_action: Coordinator review and integration after authorized branch publication; no PR requested.
```

## Validation notes

The prepared env.sh forces Rust 1.100; unset RUSTUP_TOOLCHAIN to honor the installed repository pin 1.97.
Initial Rust 1.100 Clippy stopped on an existing identity lint; pinned Clippy passes without unrelated changes.
Initial parallel tests hit PostgreSQL shared lock table limits in two existing schema cleanup paths; all 28 tests pass on the serial rerun.
No migration, flags, contract bytes, or parallel-owned admin/RBAC files changed.

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requested this branch for coordinator audit integration, with no PR; blakinio owns review at the integration handoff.
source_branch_evidence: Owner task explicitly requests commit and non-force push to this branch only.
```

PostgreSQL regression durations: db_publication_holds_authorization_until_commit 7.340679912s; postgres_invalid_character_bearers_allocate_no_budgets 2.913840824s. Both fail on the base and pass with the fixes.
