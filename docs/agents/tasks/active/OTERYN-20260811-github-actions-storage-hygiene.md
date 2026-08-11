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

- [ ] Record a sanitized pre-cleanup inventory for Actions artifacts, caches and workflow runs.
- [ ] Delete caches scoped to closed pull-request merge refs while preserving open-PR and branch/default-branch caches.
- [ ] Delete workflow artifacts older than 14 days while preserving recent evidence.
- [ ] Delete only completed workflow runs older than 30 days; preserve all newer or non-completed runs.
- [ ] Do not delete releases, packages, GHCR images, repository files, environments, secrets or unrelated GitHub resources.
- [ ] Add permanent exact-scope cleanup for closed-PR caches and scheduled/manual storage hygiene.
- [ ] Produce sanitized post-cleanup counts/bytes and verify the cleanup predicates after live execution.
- [ ] Merge only after current-head CI/governance checks pass and archive this task after verified completion.

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
updated_at: 2026-08-11T07:55:00Z
head: 6521d43b9c648f4f794ef15bf85cbd96f0538b91
branch: chore/github-actions-storage-hygiene
pr: 980
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
  - Repository main was b54de1859cfdb3ca12ff8904e0b0ead82449f613 at task start.
  - GitHub API reported 15327 Actions artifacts before cleanup.
  - GitHub API reported 1056 active caches using 3281257982 bytes before cleanup.
  - Pull-request caches are scoped to refs/pull/<number>/merge and GitHub documents that they can only be restored by reruns of that pull request.
  - An attempted build-workflow retention edit caused 57 PR workflows to be created, so that edit was reverted exactly to main.
  - Head a6fd050eecf93ba6b6d7a924365572c5c170af9d passed all eight relevant PR workflows including Deep System Validation and the new storage-hygiene validation.
  - Two subsequent review findings identified a 1000-result filtered-run ceiling failure and redundant artifact/run candidate accounting; both root causes are patched and covered by focused tests.
  - GitHub documents a 1000-result ceiling for filtered workflow-run searches and supports a created date-time range, which the revised inventory now partitions recursively.
  - Main advanced by one unrelated wiki/portal-exhaustive commit with no changed-path overlap with this task.
derived:
  - Closed-PR merge-ref caches are safely disposable because they cannot benefit main or sibling pull requests and can be regenerated if a historical run is intentionally repeated.
  - Recursive time-window partitioning prevents high-volume old-run inventory from permanently blocking cache/artifact cleanup at the provider search ceiling.
  - Assigning aggregate artifact bytes to an old run lets one run deletion cover child artifacts without wasting separate deletion budget slots.
unknown:
  - Exact artifact bytes and exact number of cleanup-eligible resources until the live bounded cleanup executes after merge.
conflicts: []
first_failure:
  marker: CI_TRIGGER_AMPLIFICATION
  evidence: modifying build-synology-staging-images.yml caused 57 PR workflow runs on intermediate head 6f31832e046ad974f26721f69aa397bf6162f487; change reverted to main content
rejected_hypotheses:
  - Blanket cache or artifact deletion without retention/ownership predicates.
  - Repository cache-retention setting mutation because the live API returned HTTP 402 for that settings endpoint.
  - Editing the heavyweight build workflow solely to shorten build-record retention because the PR-trigger fan-out outweighed the benefit in this bounded task.
changed_paths:
  - .github/workflows/github-actions-storage-hygiene.yml
  - scripts/ci/github_actions_storage_hygiene.py
  - tests/ci/test_github_actions_storage_hygiene.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20260811-github-actions-storage-hygiene.md
validation:
  - command: GitHub REST preflight inventory
    result: PASS
    evidence: artifacts total_count=15327; active_caches_count=1056; active_caches_size_in_bytes=3281257982
  - command: build workflow scope rollback
    result: PASS
    evidence: .github/workflows/build-synology-staging-images.yml content SHA restored to main blob e67f8ba64883bcfe009df5e87a4fcc6ca43b5746
  - command: PR current-head validation before review fixes
    result: PASS
    evidence: head a6fd050eecf93ba6b6d7a924365572c5c170af9d passed Agent Governance, CI, GitHub Actions Storage Hygiene, Deep System Validation, Phase 7, DB Outage, Game Auth Concurrency and Edge Security Emulation
  - command: focused review-fix tests
    result: NOT_RUN
    evidence: awaiting current-head GitHub checks after commits 09b684944e55a236433b8c63303b07ab660e7bb6 and 6521d43b9c648f4f794ef15bf85cbd96f0538b91
blockers:
  - none
next_action: require all current-head checks to pass, resolve the two review threads, then merge PR 980 with exact head and execute live cleanup
```

## Notes

Retention policy is conservative: artifacts 14 days, completed runs 30 days, closed-PR merge-ref caches immediately after PR closure. Default/main branch caches are not age-pruned. Cleanup is API-budgeted with a safety reserve; high-volume run searches are partitioned recursively instead of truncating or blocking the other cleanup lanes.