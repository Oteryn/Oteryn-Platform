---
task_id: OTERYN-20261117-audit02-window
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-audit02-window

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

`audit02_varied_email_recovery_exhausts_aggregate_source_budget` failed on main
(Portal RS CI job 133826616942, 05:00:02.93Z): the sixth request got 302 instead
of 429. The source budget `identity-recovery-key-use` (5 per 3600 s) is a fixed
window aligned to the hour on the database clock, and the seven requests
straddled 05:00:00Z, so the count restarted. The limiter is correct. A test
helper `fresh_window` now waits for a fresh window when less than a minute of the
current one is left; audit02 and
`postgres_browser_csrf_authorization_locales_and_rate_limit` (the same exposure on
`identity-recovery-key-manage`, 5 per 3600 s) call it. Their assertions are unchanged.

## Acceptance criteria

- [x] Shifting `web_rate_limits.window_start` back an hour mid-loop reproduces the CI failure (302 vs 429 at the same assertion)
- [x] The wait path runs and the test passes after it (scratch run with a 120 s window forced to wait for the next window)
- [x] Both recovery-key budget tests start inside one window; three consecutive green runs of audit02
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-audit02-window.md
  - docs/agents/tasks/archive/OTERYN-20261117-classb-identity.md
  - portal-rs/crates/identity/src/recovery_key_tests.rs
modules:
  - identity
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
updated_at: 2026-11-17T05:55:00Z
head: 32890246deff80a1f6bec79e55ff08ce397c5ba2
branch: claude/happy-davinci-ijz3wy-audit02-window
pr: none
status: validating
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-audit02-window.md
  - docs/agents/tasks/archive/OTERYN-20261117-classb-identity.md
  - portal-rs/crates/identity/src/recovery_key_tests.rs
proven:
  - The CI failure line is at 05:00:02.93Z, seconds after the hourly window rolled over
  - No product code, migration, contract, fixture, flag or wire byte change
derived:
  - Tests outside this packet that fill a fixed-window budget (crates/web/tests/ratelimit.rs anonymous_source_budget_survives_rotating_identifiers, termination game_closure throttling) may carry the same rollover exposure; left for a separate task
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: recovery_key_tests.rs:542 left 302 right 429
  evidence: Portal RS CI job 133826616942 on main
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-audit02-window.md
  - docs/agents/tasks/archive/OTERYN-20261117-classb-identity.md
  - portal-rs/crates/identity/src/recovery_key_tests.rs
validation:
  - command: cargo test --locked -p oteryn-identity --lib audit02_varied_email (PostgreSQL 18), without the wait and with window_start shifted back an hour mid-loop, then with the change
    result: PASS
    evidence: the shifted run fails with left 302 right 429 (the CI signature); with the change three consecutive runs pass
  - command: cargo test --locked -p oteryn-identity --lib audit02_varied_email -- --nocapture (scratch 120 s window, forced wait)
    result: PASS
    evidence: waited from 104 s left to a fresh window, then 1 passed
  - command: cargo test --locked -p oteryn-identity --lib recovery_key (PostgreSQL 18)
    result: PASS
    evidence: 16 passed, 0 failed
  - command: cargo fmt -p oteryn-identity; cargo clippy --locked -p oteryn-identity --all-targets -- -D warnings
    result: PASS
    evidence: exit 0
blockers:
  - none
next_action: "wait for Portal RS CI on the PR, then squash auto-merge"
```
