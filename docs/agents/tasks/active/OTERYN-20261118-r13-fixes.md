---
task_id: OTERYN-20261118-r13-fixes
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261118-r13-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Review round 13: eleven small-slice Codex reviews of portal-rs on the round 12
head. Confirmed P0-P2 findings were fixed by Codex Cloud packets with
red-then-green regression tests: Bazaar replay during recovery resync (gold
settlement Bind, earlier OUTBID release of a later winner, compensated release,
missing payout, Deliver/Return intent rebuilt with the new escrow digest),
the ITEMS journal 30 s producer clock allowance, and delayed CloseAccount and
FINALIZE expiry refusals after a recovery-generation increase. Also archives
the round 9 packet after PR #1945 merged.

## Acceptance criteria

- [x] Each confirmed finding has a fix with a regression test through the real caller
- [x] Each pushed fix was reviewed read-only before integration
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261118-r9-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r13-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r9-fixes.md
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_evidence.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
modules:
  - bazaar
  - identity
  - playerdata
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
updated_at: 2026-11-18T06:30:00Z
head: pending
branch: claude/happy-davinci-ijz3wy-r13-fixes
pr: 1946
status: implementing
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261118-r9-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r13-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r9-fixes.md
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_evidence.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
proven:
  - Each round-13 finding was confirmed by a read-only check with file:line evidence
  - Each pushed fix was reviewed read-only against its packet before integration
derived:
  - none
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261118-r9-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r13-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r9-fixes.md
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_evidence.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
validation:
  - command: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0 on the integrated tree
  - command: cargo test per crate with PostgreSQL 18
    result: PASS
    evidence: Codex Cloud fix runs with PostgreSQL - bazaar 185, playerdata 196, identity 197, 0 failed, 0 ignored
  - command: Portal RS CI on the PR head
    result: NOT_RUN
    evidence: full suite runs in CI shards
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: server-side fixes covered by tests through the real routers and workers
blockers:
  - none
next_action: Portal RS CI on the PR head, then squash auto-merge
```
