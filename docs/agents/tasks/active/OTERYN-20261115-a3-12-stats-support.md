---
task_id: OTERYN-20261115-a3-12-stats-support
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit wave 3: statistics and support

Governing workstream: https://github.com/Oteryn/Oteryn-Platform/issues/536 (OPEN, verified live 2026-11-15). Owner authorizes this exact task branch and push, without a PR. Single agent. Parallel RBAC ownership exclusions preserved. Jira synchronization belongs to the coordinator.

Base: 90234d8d2d0a51a654b8ff6947d38372a740acba.
Branch: claude/happy-davinci-ijz3wy-fix-a3-12-stats-support.

## Goal and acceptance

Fix W3-01 persistent death quarantine in derivation and both derived readers; W3-02 edited ticket draft recovery; W3-03 terminal receipt ordering. Confirm each against the base, reproduce with regression tests, pass required formatting, Clippy and crate tests with real PostgreSQL, commit and push the authorized branch. No migration or contract changes required.

## Ownership

- portal-rs/crates/stats
- portal-rs/crates/support
- portal-rs/Cargo.lock (existing community dependency for publisher regression)
- this task packet

## Evidence

All three findings reproduced before their fixes. W3-01 failed because derived history exposed quarantined deaths; W3-02 failed because the displayed draft retained the bound request key; W3-03 failed because stale terminal revision 1 returned 200 after pending revision 2.

Final Rust 1.97 checks: formatting PASS; warnings-denied Clippy for both touched crates PASS; all 152 tests PASS, zero failed/ignored. Real PostgreSQL was configured without printing credentials. Regression durations: W3-01 0.126949 s; W3-02 0.220391 s; W3-03 0.235175 s. No PostgreSQL skips. Full suites use two test threads to stay within the prepared PostgreSQL lock table; concurrency inside tests remains enabled. The initial unbounded run exhausted PostgreSQL shared lock memory in three existing support tests, resolved by this bounded rerun. The setup's Rust 1.100 environment override exposed an unrelated Clippy lint; the final required checks use the repository's already-installed Rust 1.97 pin. No tools reinstalled.

Reviewed the complete diff and excluded-path check. No extra product paths outside stats/support; the lockfile adds only the existing community crate as a development dependency to exercise the real V2 publisher. This packet is the only governance path added. No PR requested or created.

## Source branch closeout

source_branch_disposition: retain
source_branch_reason: Owner requests branch publication only; coordinator owns integration and later deletion.
source_branch_evidence: current owner task instruction

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T21:10:27Z
head: 90234d8d2d0a51a654b8ff6947d38372a740acba
branch: claude/happy-davinci-ijz3wy-fix-a3-12-stats-support
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/stats
  - portal-rs/crates/support
  - portal-rs/Cargo.lock
proven:
  - All three findings confirmed against the base and reproduced by PostgreSQL regressions
  - Focused PostgreSQL regressions pass after fixes and each takes more than 0.05 seconds
  - No merged migration, wire contract or feature flag changed
  - Forbidden parallel RBAC paths untouched
  - GitHub repository identity and governing Issue 536 OPEN verified live
  - Player HTTP ticket recovery and internal receipt HTTP reconciliation exercised
  - Actual V2 death publication, conflict, worker and both derived readers exercised
unknown:
  - Coordinator integration and Jira synchronization
derived: []
conflicts: []
first_failure:
  marker: none
  evidence: Expected pre-fix regression failures recorded in local validation logs
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/stats
  - portal-rs/crates/support
  - portal-rs/Cargo.lock
validation:
  - command: cargo fmt --all --check; cargo clippy --locked --all-targets -p oteryn-stats -p oteryn-support -- -D warnings; cargo test --locked -p oteryn-stats -p oteryn-support
    result: PASS
    evidence: Rust 1.97; 152 tests passed, zero failed or ignored; real PostgreSQL regressions took 0.126949 s, 0.220391 s and 0.235175 s; test threads limited to two
blockers: []
next_action: Commit and publish the owner-authorized task branch for coordinator integration
```
