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
updated_at: 2026-08-11T07:27:00Z
head: ce7c0fe438c2115ab35c4b2e3803e7206e7bbda1
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
  - Draft PR 980 owns the storage-hygiene implementation.
  - An attempted build-workflow retention edit caused 57 PR workflows to be created, so that edit was reverted exactly to main before merge.
  - Existing CI now executes the storage-hygiene pure contract test through test_workflow_trigger_economy.py before merge.
derived:
  - Closed-PR merge-ref caches are safely disposable because they cannot benefit main or sibling pull requests and can be regenerated if a historical run is intentionally repeated.
  - Keeping the existing build workflow unchanged avoids making storage hygiene itself a trigger for the repository's heavyweight product validation suite.
unknown:
  - Exact artifact bytes and exact number of cleanup-eligible runs until the paginated live audit executes after merge.
conflicts: []
first_failure:
  marker: CI_TRIGGER_AMPLIFICATION
  evidence: modifying build-synology-staging-images.yml caused 57 PR workflow runs on head 6f31832e046ad974f26721f69aa397bf6162f487; change reverted to main content
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
  - command: current-head pull-request CI
    result: NOT_RUN
    evidence: awaiting checks after CI-contract wiring
blockers:
  - none
next_action: inspect PR 980 current-head checks and fix any failing validation before merge
```

## Notes

Retention policy for this task is intentionally conservative: artifacts 14 days, completed runs 30 days, closed-PR merge-ref caches immediately after PR closure. Default/main branch caches are not age-pruned by this task. Cleanup is batched to preserve a GitHub API request reserve; any provider-limited residual must be reported exactly and left under the permanent scheduled cleanup rather than bypassing API safety.