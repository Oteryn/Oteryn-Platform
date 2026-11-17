---
task_id: OTERYN-20261117-portal-rs-l5-league-read
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-portal-rs-l5-league-read

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Main is red: Portal RS CI test shard 2 fails
`large_pages_tests::l5_league_fourteen_full_seasons_are_complete` with
`Error: Unavailable` (main e9b0bcaa, run 43975618006; also on #1894 and #1895).
Root cause: in a debug build the league read spends about 93 ms per
1000-standing season in `SeasonRow` validation (per-character NFC and general
category checks), so 14 seasons take about 2 s idle and over the 3 s read bound
on a loaded runner. Fix: dev-profile opt-level overrides for `oteryn-minigames`
and the two unicode crates. No product code, timeout or test changes.

## Acceptance criteria

- [x] Baseline reproduced red locally under one-core contention: read 3.15 s, `Error: Unavailable` (2 of 2 runs)
- [x] Same contention with the fix: read 1.64-1.75 s, test passes (3 of 3 runs)
- [x] No timeout, test, retry or product code change
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - portal-rs/Cargo.toml
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-l5-league-read.md
modules:
  - ci
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
updated_at: 2026-11-17T00:40:00Z
head: 0b7e89d0caf73dd5306aff5283b99eeeff154445
branch: claude/happy-davinci-ijz3wy-l5fix
pr: 1897
status: validating
context_routes: []
owned_paths:
  - portal-rs/Cargo.toml
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-l5-league-read.md
proven:
  - Base is origin/main 8e16fc6d; CI test shards run plain cargo test (dev profile)
  - Per-season decode 92-93 ms with no override or with any one override alone; 17 ms with all three
derived: []
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: l5_league_fourteen_full_seasons_are_complete Error Unavailable
  evidence: Portal RS CI run 43975618006 test shard 2 on main e9b0bcaa
rejected_hypotheses:
  - Canonical JSON re-serialization (7.5 ms per season)
changed_paths:
  - portal-rs/Cargo.toml
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-l5-league-read.md
validation:
  - command: cargo test --locked -p oteryn-minigames -p oteryn-portal (PostgreSQL 18)
    result: PASS
    evidence: portal bin 212 passed 1 ignored (pre-existing), minigames 74 passed
blockers:
  - none
next_action: Push, open the PR, watch Portal RS CI
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active; branch not yet pushed
source_branch_evidence: pending
```
