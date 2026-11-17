---
task_id: OTERYN-20261117-portal-rs-league-fence-hold
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-portal-rs-league-fence-hold

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Main went red on Portal RS CI test shard 2:
`league::tests::database_replay_correction_freeze_and_retention` fails with
55P03 lock timeout (main 08b3dace, run 43995651178; also the first attempt on
PR 1896). The test runs two duplicate ingests concurrently; the second waits on
the league write fence (advisory lock 94000003) under `lock_timeout = 100ms`
while the first holds it. Measured inside the fence (temporary local timing,
not committed): in the debug build the Ed25519 `verify_strict` takes 8-31 ms of
an 8-42 ms hold, commit up to 35 ms under CPU load. Fix: dev-profile opt-level
overrides for `curve25519-dalek`, `ed25519-dalek` and `sha2`, following the
existing unicode overrides. No product code, timeout or test changes.

## Acceptance criteria

- [x] Fence hold measured before the fix: max 42 ms locally with all cores busy, verify up to 31 ms of it
- [x] Same load with the fix: max hold 11.5 ms, verify at most 0.12 ms
- [x] No timeout, test, retry or product code change
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - portal-rs/Cargo.toml
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-league-fence-hold.md
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
updated_at: 2026-11-17T03:05:00Z
head: bb2f6c07f2aa9d2fa820750753aa7daba376b9e4
branch: claude/happy-davinci-ijz3wy-leaguefence
pr: 1899
status: validating
context_routes: []
owned_paths:
  - portal-rs/Cargo.toml
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-league-fence-hold.md
proven:
  - Base is origin/main bb2f6c07; CI test shards run plain cargo test (dev profile)
  - The test takes the global test lock and its own schema, so the fence is contended only by its joined duplicate ingest
  - Debug verify_strict 7.6 ms idle, 43 us with the three overrides (standalone bench)
derived:
  - A loaded 4-vCPU runner with PostgreSQL on the same host can stretch the debug hold past 100 ms; not reproduced locally
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: database_replay_correction_freeze_and_retention 55P03 lock timeout
  evidence: Portal RS CI run 43995651178 test shard 2 on main 08b3dace
rejected_hypotheses:
  - Cross-test contention on the fence (each test has its own schema and holds the global test lock)
changed_paths:
  - portal-rs/Cargo.toml
  - docs/agents/tasks/active/OTERYN-20261117-portal-rs-league-fence-hold.md
validation:
  - command: cargo test --locked -p oteryn-minigames --lib (PostgreSQL 18), three runs
    result: PASS
    evidence: 74 passed each run
blockers:
  - none
next_action: wait for Portal RS CI on PR 1899
```
