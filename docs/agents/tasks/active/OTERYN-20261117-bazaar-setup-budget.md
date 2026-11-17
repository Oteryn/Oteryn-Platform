---
task_id: OTERYN-20261117-bazaar-setup-budget
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-bazaar-setup-budget

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Fix the CI flake in
`game::authority_tests::postgres_cleanup_attempt_pointer_bounds_all_commands_and_issuer_wait`
(57014 at authority_tests.rs, seen on PR 1917 shard 1). The setup stores 512
receipts through `Authority::receive` in one transaction. `receive` calls
`locking::listing_gate`, which calls `oteryn_db::mutation_budget`. That budget's
deadline is transaction-local (`oteryn.mutation_deadline`) and is set once, so
all 512 receives shared one 3 s budget. Production receives one receipt per
request transaction (authority_http.rs). The setup now resets the deadline
before each receive. The 3 s budget, the production code and every assertion
are unchanged; `issue()` under test runs in its own transaction with no reset.
Coordinator approved this variant on 2026-11-17.

## Acceptance criteria

- [x] Under load (6 parallel copies + 8 CPU burners on 4 CPUs) the old layout fails 7/12 with 57014; the fixed test passes 12/12
- [x] Idle: fixed test passes 3/3; the longest single receipt under load is 95 ms against the 3 s budget
- [x] No production code, budget or assertion change
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-bazaar-setup-budget.md
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
modules:
  - bazaar
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
updated_at: 2026-11-17T09:05:00Z
head: 83c0255669c088c639b6e24234fadde5d65da337
branch: claude/happy-davinci-ijz3wy-bazaar-setup-tx
pr: none
status: ready
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-bazaar-setup-budget.md
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
proven:
  - Root cause is the shared transaction-wide mutation deadline in the setup, measured as setup transaction time up to 4.26 s under load
  - One transaction per command still reached 3.46-3.49 s under load and failed
derived:
  - Active packets audit-a2-l6, audit-a3-l6, audit-bza, a3-drb and a3-drc list this test file but have no PR, so no live writer overlaps
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: 57014 canceling statement due to statement timeout
  evidence: PR 1917 run 44045549584 shard 1
rejected_hypotheses:
  - One transaction per command (still exceeds 3 s under load)
  - One transaction per receipt (breaks the scan-bound assertion even idle, Return 269 tuples; tracked separately in issue 1919)
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-bazaar-setup-budget.md
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
validation:
  - command: 12 runs of the test under load, old and fixed tree (PostgreSQL 18)
    result: PASS
    evidence: old 7/12 FAIL 57014; fixed 12/12 PASS; idle 3/3 PASS
  - command: cargo fmt --check -p oteryn-bazaar; cargo clippy -p oteryn-bazaar --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
  - command: cargo test -p oteryn-bazaar --lib (PostgreSQL 18)
    result: PASS
    evidence: 117 passed, 0 failed
blockers:
  - none
next_action: open the PR and record CI on its head
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: not yet merged
source_branch_evidence: none
```
