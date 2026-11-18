---
task_id: OTERYN-20261118-r14-fixes
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261118-r14-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Review round 14: small-slice Codex reviews of portal-rs on the round 13
head. Confirmed P0-P2 findings are fixed by Codex Cloud packets with
red-then-green regression tests: Bazaar replay during recovery resync
(historical Character Lock, EXCESS release of the leader's hold, Character
Payout intent after restore), G-11 superseded-epoch retention, delayed records
of an earlier ownership period, login promotion feed-freshness ceiling,
zero-duration G-9 segments, and delayed CANCEL rejections after a
recovery-generation increase. Also archives the round 13 packet after PR #1946
merged.

## Acceptance criteria

- [ ] Each confirmed finding has a fix with a regression test through the real caller
- [ ] Each pushed fix was reviewed read-only before integration
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261118-r13-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r14-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r13-fixes.md
  - portal-rs/crates/bazaar/src/game/**
  - portal-rs/crates/identity/src/termination/**
  - portal-rs/crates/internal-api/src/playerdata/sessions.rs
  - portal-rs/crates/internal-api/tests/**
  - portal-rs/crates/playerdata/src/**
modules:
  - bazaar
  - identity
  - internal-api
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
updated_at: 2026-11-18T06:45:00Z
head: pending
branch: claude/happy-davinci-ijz3wy-r14-fixes
status: implementing
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261118-r13-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r14-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r13-fixes.md
  - portal-rs/crates/bazaar/src/game/**
  - portal-rs/crates/identity/src/termination/**
  - portal-rs/crates/internal-api/src/playerdata/sessions.rs
  - portal-rs/crates/internal-api/tests/**
  - portal-rs/crates/playerdata/src/**
proven:
  - Each round-14 finding was confirmed by a read-only check with file:line evidence
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
  - docs/agents/tasks/active/OTERYN-20261118-r13-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r14-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r13-fixes.md
  - portal-rs/crates/bazaar/src/game/**
  - portal-rs/crates/identity/src/termination/**
  - portal-rs/crates/internal-api/src/playerdata/sessions.rs
  - portal-rs/crates/internal-api/tests/**
  - portal-rs/crates/playerdata/src/**
validation:
  - command: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings
    result: NOT_RUN
    evidence: pending integration
  - command: cargo test per crate with PostgreSQL 18
    result: NOT_RUN
    evidence: pending integration
  - command: Portal RS CI on the PR head
    result: NOT_RUN
    evidence: full suite runs in CI shards
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: server-side fixes covered by tests through the real routers and workers
blockers:
  - none
next_action: integrate Codex fix branches, local checks, open PR
```
