---
task_id: OTERYN-20261118-r15-fixes
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261118-r15-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Review round 15: small-slice Codex reviews of portal-rs on the round 14
head. Confirmed P0-P2 findings are fixed by Codex Cloud packets with
red-then-green regression tests: Bazaar character custody verified against the
Lock's issuance projection epoch after recovery, settled sales finishing when the
seller is later restricted (and refund when the buyer is), Bazaar publication
lock taken before wallet locks on character handoff, character erasure bounded by
the job's closed ownership revision, and expired G-11 progress records stored and
retained only in content-free tombstone form. Also archives the round 14 packet
after PR #1947 merged.

## Acceptance criteria

- [x] Each confirmed finding has a fix with a regression test through the real caller
- [x] Each pushed fix was reviewed read-only before integration
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261118-r14-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r15-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r14-fixes.md
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/game_store.rs
  - portal-rs/crates/bazaar/src/game/game_wire.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/playerdata/src/journal_tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
modules:
  - bazaar
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
updated_at: 2026-11-18T08:45:00Z
head: e367f15f
branch: claude/happy-davinci-ijz3wy-r15-fixes
pr: 1948
status: completed
terminal_pr_policy: archived
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261118-r14-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r15-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r14-fixes.md
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/game_store.rs
  - portal-rs/crates/bazaar/src/game/game_wire.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/playerdata/src/journal_tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
proven:
  - Each round-15 finding was confirmed by a read-only check with file:line evidence
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
  - docs/agents/tasks/active/OTERYN-20261118-r14-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r15-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r14-fixes.md
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/game_store.rs
  - portal-rs/crates/bazaar/src/game/game_wire.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/playerdata/src/journal_tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
validation:
  - command: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings
    result: PASS
    evidence: integrated tree
  - command: cargo test per crate with PostgreSQL 18
    result: PASS
    evidence: local PostgreSQL 18 on the integrated tree - playerdata 202, bazaar 193; 0 failed, 0 ignored
  - command: Portal RS CI on the PR head
    result: PASS
    evidence: PR #1948 merged as e367f15f with required checks green
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: server-side fixes covered by tests through the real routers and workers
blockers:
  - none
next_action: none; delivered by PR #1948 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR; head branch deleted after merge
source_branch_evidence: PR 1948 merged as e367f15f1e0a171f46edebb743c12588397b9e2e
```
