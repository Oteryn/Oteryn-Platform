---
task_id: OTERYN-20260811-github-actions-storage-hygiene
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - .github/workflows
  - GitHub Actions artifacts caches workflow runs
optional_reads:
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
---

# OTERYN-20260811-github-actions-storage-hygiene

## Goal

Audit and safely reduce GitHub Actions storage for `blakinio/Oteryn-Platform`, then leave deterministic maintenance that prevents closed-PR caches and stale Actions evidence from accumulating again.

## Acceptance criteria

- [x] Record a sanitized pre-cleanup inventory for Actions artifacts, caches and workflow runs.
- [ ] Delete caches scoped to closed pull-request merge refs while preserving open-PR and branch/default-branch caches.
- [ ] Delete workflow artifacts older than 14 days while preserving recent evidence.
- [x] Delete only completed workflow runs older than 30 days; preserve all newer or non-completed runs. No eligible run exists because the repository is younger than 30 days.
- [x] Do not delete releases, packages, GHCR images, repository files, environments, secrets or unrelated GitHub resources.
- [x] Add permanent exact-scope cleanup for closed-PR caches and scheduled/manual storage hygiene.
- [ ] Produce terminal sanitized post-cleanup counts/bytes and verify the cleanup predicates after all bounded live cleanup passes.
- [x] Implementation PR #980 merged after exact-head CI/governance/heightened validation.
- [ ] Remove the one-time #980 push bootstrap after terminal live cleanup evidence and archive this task.

## Ownership

```yaml
owned_paths:
  - .github/workflows/github-actions-storage-hygiene.yml
  - scripts/ci/github_actions_storage_hygiene.py
  - tests/ci/test_github_actions_storage_hygiene.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20260811-github-actions-storage-hygiene.md
  - docs/agents/tasks/archive/OTERYN-20260811-github-actions-storage-hygiene.md
modules:
  - github-actions-operations
dependencies:
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T11:46:00+02:00
head: 08ccdadca4c55fb76ca8e3526260acf6a7af0e3b
branch: chore/github-actions-storage-hygiene-archive-pending
pr: 990
status: validating
context_routes:
  - ci-operations
  - execution-resource-hygiene
owned_paths:
  - .github/workflows/github-actions-storage-hygiene.yml
  - scripts/ci/github_actions_storage_hygiene.py
  - tests/ci/test_github_actions_storage_hygiene.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20260811-github-actions-storage-hygiene.md
  - docs/agents/tasks/archive/OTERYN-20260811-github-actions-storage-hygiene.md
proven:
  - Repository pre-cleanup inventory reported 15327 Actions artifacts and 1056 active caches using 3281257982 bytes at task discovery.
  - Pull-request caches are scoped to refs/pull/<number>/merge and can only be restored by reruns of that pull request.
  - Implementation PR #980 merged to protected main as 11e223f5f0883f0f3096769fbc2291de7edae62e from exact head 8f6efc6b08f9c7b31160db5ec1ffdb032c83e9c2.
  - Exact #980 implementation head passed Agent Governance 31474126909, CI 31474126854, GitHub Actions Storage Hygiene 31474126884, Phase 7 31474126847, Platform DB Outage 31474126899, Game Auth Ticket Concurrency 31474126846, Edge Security 31474126877 and Deep System Validation 31474126860 before merge.
  - Post-merge push workflow run 31476011425 is the intended one-time cleanup bootstrap on exact merge 11e223f5f0883f0f3096769fbc2291de7edae62e.
  - Post-merge cleanup attempt 1 maintenance job 93729781096 completed successfully with exact bounded cleanup mode and delete budget 700.
  - Attempt 1 started with 15525 artifacts / 12224098506 bytes and 1127 caches / 5498901543 bytes, selected 700 exact candidates, deleted 494 artifacts / 4244404516 bytes and 206 caches / 2273009806 bytes, leaving 15042 artifacts and 929 caches / 3248921261 bytes.
  - Attempt 1 reported 5009 eligible candidates remaining only because of the explicit bounded deletion budget.
  - Post-merge cleanup attempt 2 of run 31476011425 completed successfully; maintenance job 93733099349 ran from 2026-08-11T09:18:45Z through 2026-08-11T09:28:06Z and reached the cleanup completion boundary.
  - PR #990 is the live follow-up delivery path while post-merge cleanup remains incomplete, so ownership of the workflow/script/test paths remains active until terminal cleanup, bootstrap removal and archival.
  - The one-time push bootstrap is fail-closed to a main push whose head commit message begins with chore(ci): clean GitHub Actions storage safely (#980); ordinary later pushes do not execute destructive maintenance.
derived:
  - The stale pre-merge next_action was incorrect after PR #980 merged and caused unrelated Agent Governance failures.
  - `terminal_pr_policy: archive_pending` is not valid while live cleanup and bootstrap-removal edits remain because task liveness would release ownership prematurely.
  - Terminal closeout requires bounded cleanup passes until no cleanup-eligible candidate remains due to the per-pass deletion budget, then removal of the one-time #980 push bootstrap and task archival.
unknown:
  - Exact terminal artifact/cache counts and bytes until bounded live cleanup reaches zero remaining eligible candidates.
  - Exact remaining eligible candidate count after successful attempt 2 because the connector exposes terminal job status but not the step log payload containing the cleanup summary.
conflicts: []
first_failure:
  marker: LIVE_CLEANUP_REMAINS_AFTER_BOUNDED_PASS
  evidence: post-merge run 31476011425 attempt 1 succeeded but reported remaining_eligible_candidates_due_to_budget=5009 after 700 exact deletions; attempt 2 is now terminal success and a further bounded pass is required unless terminal inventory proves otherwise.
rejected_hypotheses:
  - PR #980 is still merge-pending; GitHub proves it merged as 11e223f5f0883f0f3096769fbc2291de7edae62e.
  - The active task may be archived immediately after implementation merge; live cleanup acceptance is still incomplete.
  - `terminal_pr_policy: archive_pending` may be used while cleanup remains active; review and task-liveness behavior prove that would release ownership too early.
  - Blanket cache or artifact deletion without retention/ownership predicates.
  - Repository cache-retention setting mutation because the live API returned HTTP 402 for that settings endpoint.
  - Editing unrelated heavyweight workflows merely to shorten storage retention.
changed_paths:
  - .github/workflows/github-actions-storage-hygiene.yml
  - scripts/ci/github_actions_storage_hygiene.py
  - tests/ci/test_github_actions_storage_hygiene.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20260811-github-actions-storage-hygiene.md
validation:
  - command: implementation PR #980 exact-head validation and merge verification
    result: PASS
    evidence: PR #980 merged as 11e223f5f0883f0f3096769fbc2291de7edae62e after all declared exact-head validation gates passed.
  - command: post-merge push cleanup attempt 1
    result: PASS
    evidence: run 31476011425 attempt 1 maintenance job 93729781096 completed bounded exact-ID cleanup and terminal reporting; remaining_eligible_candidates_due_to_budget=5009 proves more bounded passes were required.
  - command: post-merge push cleanup attempt 2
    result: PASS
    evidence: run 31476011425 attempt 2 maintenance job 93733099349 completed successfully and reached the cleanup completion boundary.
blockers: []
next_action: execute one additional bounded maintenance rerun from trusted main, inspect its terminal result, and continue bounded passes only while cleanup-eligible candidates remain before removing the one-time bootstrap and archiving the task
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: PORTAL-CLOSEOUT-20260811-1144
  session_started_at: 2026-08-11T11:44:00+02:00
  checkpointed_at: 2026-08-11T11:46:00+02:00
  last_progress_at: 2026-08-11T11:46:00+02:00
  phase: live_actions_storage_cleanup
  exact_head: 08ccdadca4c55fb76ca8e3526260acf6a7af0e3b
  pull_request: 990
  active_operation: none
  external_run_ids:
    - 31476011425
  operation_started_at: null
  wait_deadline_at: null
  check_generation: cleanup-attempt-3
  checks_used: 0
  status: ready
  safe_to_resume: true
  resume_condition: attempt 2 is terminal success and the next operation is one bounded trusted-main maintenance rerun
  next_action: rerun maintenance job 93733099349 once, then inspect the resulting run attempt before deciding whether another bounded cleanup pass is required
```

## Notes

Retention policy is conservative: artifacts 14 days, completed runs 30 days, closed-PR merge-ref caches immediately after PR closure. Default/main branch caches are not age-pruned. Cleanup is API-budgeted with a safety reserve; high-volume run searches are partitioned recursively instead of truncating or blocking the other cleanup lanes.
