---
task_id: OTERYN-20260811-github-actions-storage-hygiene
status: completed
project_lane: oteryn-platform-core
---

# OTERYN-20260811 GitHub Actions storage hygiene

## Goal

Audit and safely reduce GitHub Actions storage for `blakinio/Oteryn-Platform`, then leave deterministic bounded maintenance that prevents closed-PR caches and stale Actions evidence from accumulating again.

## Acceptance criteria

- [x] Pre-cleanup Actions artifacts/cache inventory recorded without secrets.
- [x] Closed-PR merge-ref caches removed only by exact ID; open-PR and branch/default-branch cache scopes preserved.
- [x] Artifacts older than 14 days removed only by exact ID; recent evidence preserved.
- [x] Completed workflow runs use a 30-day threshold; zero runs were eligible during live execution because the repository was younger than that threshold.
- [x] Releases, packages, GHCR images, repository content, environments, secrets and unrelated resources remained outside deletion authority.
- [x] GitHub filtered workflow-run search ceiling is handled by bounded recursive time-window partitioning.
- [x] Parent-run/artifact deletion budgeting avoids hiding artifact candidates unless the exact parent run is selected.
- [x] Two successful live bounded cleanup attempts completed and sanitized evidence is retained.
- [x] One-time implementation push cleanup authority was removed after live execution.
- [x] Permanent closed-PR cleanup, twice-daily bounded maintenance and confirmation-gated manual cleanup remain.
- [x] Superseded PR-validation runs may cancel, while closed-PR and maintenance cleanup executions remain non-cancelled.
- [x] Implementation and closeout PRs passed exact-head repository validation and all review threads are terminal.
- [x] Task reached terminal closeout and is moved to archive.

## Ownership

```yaml
owned_paths:
  - .github/workflows/github-actions-storage-hygiene.yml
  - scripts/ci/github_actions_storage_hygiene.py
  - tests/ci/test_github_actions_storage_hygiene.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/evidence/OTERYN-20260811-actions-storage-hygiene-live.md
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

## Final outcome

Implementation PR #980 merged as `11e223f5f0883f0f3096769fbc2291de7edae62e`. Live workflow run `31476011425` completed two successful bounded maintenance attempts:

- attempt 1 job `93729781096`: 494 old artifacts / 4,244,404,516 bytes and 206 closed-PR caches / 2,273,009,806 bytes deleted;
- attempt 2 job `93733099349`: 531 old artifacts / 322,357,146 bytes and 169 closed-PR caches / 22,419,524 bytes deleted;
- combined: **1,400 exact resources** — 1,025 artifacts plus 375 closed-PR caches — reclaiming **6,862,190,992 bytes (~6.39 GiB)**;
- workflow runs deleted: 0, because none satisfied the 30-day retention threshold.

Attempt 2 post-state reported 14,523 artifacts and 906 active caches using 3,224,537,526 bytes. 4,344 qualifying historical candidates remained because each execution is intentionally capped at 700 exact deletions. Normal CI concurrently creates recent artifacts/caches, so count snapshots are not expected to monotonically decrease one-for-one.

Closeout PR #993 merged as `7edd8c9480f3faa8dbea49cc63542861a346295a` after eight of eight exact-head workflows passed on `44f24e05617e9e1e480fef130f370e3a0b2a5466` and all Codex review findings were resolved.

Permanent `.github/workflows/github-actions-storage-hygiene.yml` on resulting `main` has:

- no `push` trigger and no #980 bootstrap condition;
- exact closed-PR cache cleanup on `pull_request_target: closed`;
- bounded scheduled cleanup at `23 3,15 * * *` (03:23 and 15:23 UTC);
- manual audit/cleanup with exact confirmation `CLEAN_ACTIONS_STORAGE`;
- 700-delete cap per maintenance execution;
- PR-only `cancel-in-progress`, leaving closed-PR and maintenance cleanup non-cancelled;
- no package/GHCR/release/repository-content/environment/secret deletion path.

Full sanitized live evidence is retained at `docs/agents/evidence/OTERYN-20260811-actions-storage-hygiene-live.md`.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T10:31:00Z
head: 7edd8c9480f3faa8dbea49cc63542861a346295a
branch: main
pr: 993
status: completed
context_routes:
  - ci-operations
  - execution-resource-hygiene
owned_paths:
  - .github/workflows/github-actions-storage-hygiene.yml
  - scripts/ci/github_actions_storage_hygiene.py
  - tests/ci/test_github_actions_storage_hygiene.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/evidence/OTERYN-20260811-actions-storage-hygiene-live.md
  - docs/agents/tasks/archive/OTERYN-20260811-github-actions-storage-hygiene.md
proven:
  - PR #980 final head 8f6efc6b08f9c7b31160db5ec1ffdb032c83e9c2 merged as 11e223f5f0883f0f3096769fbc2291de7edae62e.
  - Live run 31476011425 attempts 1 and 2 removed 1400 exact resources and reclaimed 6862190992 bytes from stale Actions artifacts/caches.
  - No packages, GHCR images, releases, repository content, environments, secrets, recent artifacts, open-PR caches or branch/default-branch caches were cleanup targets.
  - PR #993 final head 44f24e05617e9e1e480fef130f370e3a0b2a5466 passed Agent Governance, CI, GitHub Actions Storage Hygiene, Deep System Validation, Phase 7 Production-Like Validation, Platform DB Outage Validation, Game Auth Ticket Concurrency and Edge Security Emulation.
  - PR #993 merged as 7edd8c9480f3faa8dbea49cc63542861a346295a.
  - Resulting main has no push cleanup authority and retains bounded closed-PR, twice-daily and manual cleanup controls.
derived:
  - The dominant byte pressure was removed in the first two bounded passes; historical count residual can converge through retained bounded maintenance without broad deletion authority.
unknown:
  - Exact future residual count because normal CI continuously creates and expires resources.
conflicts: []
first_failure:
  marker: CI_TRIGGER_AMPLIFICATION
  evidence: an intermediate build-workflow retention edit caused excessive PR workflow fan-out and was fully reverted before implementation merge.
rejected_hypotheses:
  - Blanket deletion was required.
  - Package/GHCR/release cleanup was required.
  - Destructive maintenance needed a higher per-run authority than 700 exact IDs.
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20260811-github-actions-storage-hygiene.md
validation:
  - command: PR #980 implementation validation
    result: PASS
    evidence: exact-head repository gates and review closure before merge.
  - command: live cleanup run 31476011425 attempts 1 and 2
    result: PASS
    evidence: 1400 exact deletions; 6862190992 bytes reclaimed; bounded residual reported.
  - command: PR #993 closeout validation
    result: PASS
    evidence: eight of eight exact-head workflows passed on 44f24e05617e9e1e480fef130f370e3a0b2a5466; review threads terminal.
  - command: resulting-main workflow inspection
    result: PASS
    evidence: main@7edd8c9480f3faa8dbea49cc63542861a346295a has no push trigger, has twice-daily bounded schedule, PR-only cancellation and confirmation-gated manual cleanup.
blockers:
  - none
next_action: No further action; terminal closeout is complete.
```

## Self-review

```yaml
result: PASS
exact_head: 7edd8c9480f3faa8dbea49cc63542861a346295a
acceptance_checked: true
full_diff_checked: true
negative_paths_checked: true
rollback_checked: true
compatibility_checked: true
related_prs_checked: true
findings: []
evidence:
  - Live cleanup numbers were verified from successful maintenance job logs.
  - Implementation and closeout exact-head validation were terminal-success.
  - Permanent workflow scope was verified directly on resulting main.
  - All closeout review threads were resolved before merge.
```

## Notes

Terminal state: `DONE`. Historical qualifying Actions resources intentionally remaining after the bounded live cleanup are owned by the retained twice-daily/manual maintenance policy and do not represent an unfinished one-time task.
