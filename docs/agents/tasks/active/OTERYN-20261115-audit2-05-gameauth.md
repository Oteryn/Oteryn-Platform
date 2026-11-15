---
task_id: OTERYN-20261115-audit2-05-gameauth
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit wave 2 gameauth repairs

Governing workstream: https://github.com/Oteryn/Oteryn-Platform/issues/536,
verified open at task admission. Owner specifies base, branch, five findings,
PostgreSQL regressions, exact checks and push; no PR or merge. Single writer.
Existing active tasks cover Bazaar and public-domain work, without gameauth overlap.
Jira synchronization belongs to the coordinator.

Base: c6fdc5438a4afa91f32d547b7cfc61f22bf21139.
Owned implementation: portal-rs/crates/gameauth. No migration or contract changes.
All five findings confirmed against the base. Recovery must serialize only its
account, authenticated operations must honor their absolute deadline, runtime
and recovery expiry clocks must follow waits, all certificate purposes must be
validated together, and durable filesystem work must use bounded blocking workers.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T17:23:19Z
head: c6fdc5438a4afa91f32d547b7cfc61f22bf21139
branch: claude/happy-davinci-ijz3wy-fix-a2-05-gameauth
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/gameauth/
proven:
  - All five findings confirmed against the exact owner-selected base
  - Eight regression counterchecks fail with the original defect behavior restored
  - Regression tests run against isolated real PostgreSQL schemas with no skips
  - Seven new PostgreSQL durations in seconds 1.325 16.595 3.649 2.518 2.827 7.603 2.693
  - Single Tokio worker remains responsive during a six-second custody save stall
  - Revocation returns all unused pool slots while the handle blocker remains held
  - Complete gameauth suite passes 153 tests with zero failures or ignored tests
  - Existing default-off flags and contract bytes retained
  - No Game repository, production service or production data accessed
  - Repository identity is Oteryn/Oteryn-Platform
  - Startup environment override is Rust 1.100; repository pin is installed Rust 1.97
  - Final required checks use the installed repository pin without installing tools
derived: []
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: Expected pre-fix counterchecks failed; final candidate checks pass
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/gameauth/
validation:
  - command: cargo test --locked -p oteryn-gameauth
    result: PASS
    evidence: 153 passed, zero failed or ignored; PostgreSQL enabled; RUST_TEST_THREADS=1; 127.32 seconds
  - command: cargo fmt --all --check
    result: PASS
    evidence: Installed repository-pinned Rust 1.97
  - command: cargo clippy --locked --all-targets -p oteryn-gameauth -- -D warnings
    result: PASS
    evidence: Installed repository-pinned Rust 1.97
blockers:
  - none
next_action: Publish the owner-requested fix branch for coordinator review; no PR or merge
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requested this fix branch for coordinator review; no PR or merge
source_branch_evidence: Exact branch named in owner task
```
