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
- [x] Execute exact-ID bounded deletion of closed-pull-request merge-ref caches while preserving open-PR and branch/default-branch caches; install immediate per-PR cleanup so new closed-PR cache backlog does not accumulate.
- [x] Execute exact-ID bounded deletion of workflow artifacts older than 14 days while preserving recent evidence; retain scheduled/manual bounded convergence for the historical backlog.
- [x] Delete only completed workflow runs older than 30 days; no run was eligible because the repository was younger than the 30-day threshold at live execution.
- [x] Do not delete releases, packages, GHCR images, repository files, environments, secrets or unrelated GitHub resources.
- [x] Add permanent exact-scope cleanup for closed-PR caches and scheduled/manual storage hygiene.
- [x] Produce sanitized post-cleanup counts/bytes and verify the cleanup predicates after live execution.
- [ ] Merge the closeout that removes one-time push authority, then archive this task after resulting-main verification.

## Ownership

```yaml
owned_paths:
  - .github/workflows/github-actions-storage-hygiene.yml
  - scripts/ci/github_actions_storage_hygiene.py
  - tests/ci/test_github_actions_storage_hygiene.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/evidence/OTERYN-20260811-actions-storage-hygiene-live.md
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
updated_at: 2026-08-11T10:07:00Z
head: aa76773873f063aeefc84626d5d073705cbc7296
branch: chore/github-actions-storage-hygiene-closeout
pr: 993
status: validating
terminal_pr_policy: archive_pending
context_routes:
  - ci-operations
  - execution-resource-hygiene
owned_paths:
  - .github/workflows/github-actions-storage-hygiene.yml
  - scripts/ci/github_actions_storage_hygiene.py
  - tests/ci/test_github_actions_storage_hygiene.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/evidence/OTERYN-20260811-actions-storage-hygiene-live.md
  - docs/agents/tasks/active/OTERYN-20260811-github-actions-storage-hygiene.md
  - docs/agents/tasks/archive/OTERYN-20260811-github-actions-storage-hygiene.md
proven:
  - Task-start preflight reported 15327 Actions artifacts and 1056 active caches using 3281257982 bytes.
  - PR #980 final head 8f6efc6b08f9c7b31160db5ec1ffdb032c83e9c2 merged as 11e223f5f0883f0f3096769fbc2291de7edae62e.
  - Both Codex review findings on capped workflow-run search and parent-run artifact accounting were fixed, regression-tested and resolved before merge.
  - Live storage-hygiene run 31476011425 executed cleanup against trusted main with Actions write scope and no package/release/content deletion endpoints.
  - Attempt 1 maintenance job 93729781096 deleted 494 old artifacts (4244404516 bytes) and 206 closed-PR caches (2273009806 bytes), with zero workflow-run deletions.
  - Attempt 2 maintenance job 93733099349 deleted 531 old artifacts (322357146 bytes) and 169 closed-PR caches (22419524 bytes), with zero workflow-run deletions.
  - Across the two successful bounded live attempts, 1400 exact resources were deleted: 1025 artifacts plus 375 closed-PR caches, reclaiming 6862190992 bytes (~6.39 GiB) from explicit deletions.
  - Attempt 2 post-state reported 14523 artifacts and 906 active caches using 3224537526 bytes; 4344 qualifying historical candidates remained only because of the 700-delete per-run safety budget.
  - Attempt 1 inventory observed 15525 artifacts total and 4819 older than 14 days, leaving 10706 then-recent artifacts inside the 14-day retention window; that observed volume is about 765 retained artifacts per day across the window.
  - Normal CI continued creating recent artifacts/caches between snapshots; recent evidence and open/default-branch cache scopes were not cleanup candidates.
  - Closeout audit corrected supersedable PR validation concurrency while preserving non-cancelled cleanup executions.
  - Permanent pull_request_target closed-event cleanup and bounded twice-daily/manual maintenance are retained to prevent recurrence and drain historical residual without broad deletion.
derived:
  - The dominant storage pressure was stale artifacts plus large closed-PR caches; two bounded passes reclaimed most byte-heavy candidates while preserving the API safety reserve.
  - A weekly 700-delete schedule did not have demonstrated count headroom over the observed recent artifact volume; twice-daily bounded execution provides up to 1400 delete slots per day without increasing the 700-delete authority of any single run.
  - Remaining eligible count is dominated by small historical resources and can converge under the retained bounded maintenance without increasing per-run destructive authority.
unknown:
  - Exact future residual count after subsequent scheduled/manual bounded maintenance, because normal CI concurrently creates and expires resources.
conflicts: []
first_failure:
  marker: CI_TRIGGER_AMPLIFICATION
  evidence: an intermediate build-workflow retention edit caused 57 PR workflows; the edit was fully reverted before PR #980 merge and the build workflow was absent from the final diff.
rejected_hypotheses:
  - Blanket cache/artifact deletion is required; exact-ID retention predicates reclaimed the byte-heavy backlog safely.
  - Repository-wide cache-retention setting mutation is required; the endpoint returned HTTP 402 and the solution does not depend on it.
  - Packages, GHCR images, releases or repository content need cleanup for this task; they were explicitly outside scope and untouched.
changed_paths:
  - .github/workflows/github-actions-storage-hygiene.yml
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/evidence/OTERYN-20260811-actions-storage-hygiene-live.md
  - docs/agents/tasks/active/OTERYN-20260811-github-actions-storage-hygiene.md
validation:
  - command: PR #980 exact-head repository validation
    result: PASS
    evidence: implementation merged after current-head governance, CI, storage-hygiene, deep, Phase 7, DB outage, game-auth and edge validation gates.
  - command: live Actions storage hygiene run 31476011425 attempt 1 job 93729781096
    result: PASS
    evidence: 700 exact resources deleted; 6517414322 bytes reclaimed; bounded residual reported.
  - command: live Actions storage hygiene run 31476011425 attempt 2 job 93733099349
    result: PASS
    evidence: another 700 exact resources deleted; 344776670 bytes reclaimed; bounded residual reported.
  - command: closeout capacity and concurrency audit
    result: PASS
    evidence: superseded PR validation cancellation is scoped to pull_request only; two daily schedule slots provide bounded headroom while closed-PR and maintenance executions remain non-cancelled.
  - command: closeout PR #993 exact-head validation
    result: NOT_RUN
    evidence: exact-head checks pending after final closeout audit corrections.
blockers:
  - none
next_action: After PR #993 reaches terminal state, verify resulting main has no push cleanup authority, then move this task to archive.
```

## Notes

Sanitized live evidence is recorded in `docs/agents/evidence/OTERYN-20260811-actions-storage-hygiene-live.md`. The retained maintenance is deliberately bounded at 700 exact deletions per execution and never broadens into package/GHCR/release/repository-content deletion.
