---
task_id: OTERYN-20261116-portal-rs-ci-shards
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261116-portal-rs-ci-shards

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Portal RS CI ran fmt, clippy, every crate test and the browser tests in one
job. On main run 43955821110 (85336fc6) the tests took 1428 s after a 3m45
build and the browser step was cancelled at the 30-minute job limit. Split the
job into `lint`, three package shards of `cargo test` and `browser`, each with
the same 30-minute limit, under an aggregate job that keeps the `check` name.
Owner consent for this workflow change was relayed by the coordinator.

## Acceptance criteria

- [x] Shard 3 is the workspace minus the shard 1 and 2 lists, so every package runs in exactly one shard
- [x] At the same SHA, the test names of shards 1+2+3 equal the full-workspace `cargo test -- --list` set
- [x] Timeout stays 30 minutes per job; no test, filter or expected browser count changes
- [x] `check` fails unless lint, every test shard and browser succeeded
- [x] No change to `.github/repository-policy.json`, rulesets or other workflows
- [x] Closeout of merged PR #1891: its task packet is archived as completed

## Ownership

```yaml
owned_paths:
  - .github/workflows/portal-rs-ci.yml
  - .claude/skills/oteryn-platform-dev/references/ci-merge.md
  - .claude/skills/oteryn-platform-dev/references/local-checks.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-ci-shards.md
  - docs/agents/tasks/active/OTERYN-20261116-gameauth-unavail-cause.md
  - docs/agents/tasks/archive/OTERYN-20261116-gameauth-unavail-cause.md
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
updated_at: 2026-11-16T23:20:00Z
head: e9b0bcaaacf61d29593e38454fd77977d86335d2
branch: claude/happy-davinci-ijz3wy-ci-split
pr: 1892
status: completed
context_routes: []
owned_paths:
  - .github/workflows/portal-rs-ci.yml
  - .claude/skills/oteryn-platform-dev/references/ci-merge.md
  - .claude/skills/oteryn-platform-dev/references/local-checks.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-ci-shards.md
  - docs/agents/tasks/active/OTERYN-20261116-gameauth-unavail-cause.md
  - docs/agents/tasks/archive/OTERYN-20261116-gameauth-unavail-cause.md
proven:
  - Base is origin/main a6b1b05e
  - Shard test-name sets at a6b1b05e sum to the full workspace set (2102)
  - Branch protection on main requires platform-gate and platform-postgres (16) and (17) only; no Portal RS CI context is required
derived: []
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - .github/workflows/portal-rs-ci.yml
  - .claude/skills/oteryn-platform-dev/references/ci-merge.md
  - .claude/skills/oteryn-platform-dev/references/local-checks.md
  - docs/agents/tasks/active/OTERYN-20261116-portal-rs-ci-shards.md
  - docs/agents/tasks/active/OTERYN-20261116-gameauth-unavail-cause.md
  - docs/agents/tasks/archive/OTERYN-20261116-gameauth-unavail-cause.md
validation:
  - command: cargo test --locked [selection] -- --list --format terse, for the full workspace and for each shard selection, at a6b1b05e
    result: PASS
    evidence: full 2102 test names; shard 1 565, shard 2 554, shard 3 983; the sorted multiset of shards 1+2+3 equals the full one
  - command: python3 tests/ci/test_workflow_trigger_economy.py, test_github_actions_storage_hygiene.py, test_stable_platform_gate_policy.py, test_required_test_gate.py, test_push_change_routing.py, test_classify_changes.py
    result: PASS
    evidence: every script exit 0
  - command: python3 tools/validation/workflow_inventory.py; python3 -m unittest test_workflow_inventory (in tools/validation)
    result: PASS
    evidence: lifecycle PASS actual=27 budget=55; 18 tests OK
  - command: python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint
    result: PASS
    evidence: 27 task checkpoints validated
blockers:
  - none
next_action: none; delivered by PR 1892 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1892 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1892 merged as e9b0bcaaacf61d29593e38454fd77977d86335d2
```
