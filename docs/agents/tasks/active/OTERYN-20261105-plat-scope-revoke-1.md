---
task_id: OTERYN-20261105-plat-scope-revoke-1
governing_issue: 1419
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261105-plat-scope-revoke-1

## Goal

Governing GitHub Issue: Oteryn/Oteryn-Platform#1419 — canonical lifecycle authority for this task.

Ingest Game `ReportScopeRevocationV1` from `oteryn-game-native-runtime-status-v1` §16.1 so a committed revocation supersedes older scope assignments, invalidates stored runtime evidence, and leaves the scope unroutable until a higher assignment and matching runtime report arrive.

## Acceptance criteria

- [x] Add the authenticated, bounded, no-store internal revocation route and exact seven-member decoder.
- [x] Persist revocations in the assignment sequence with transactional ordering, replay, conflict, and report-invalidation semantics.
- [x] Make assignment, runtime ingestion, and reads fail safely for a revoked row's null node identity.
- [x] Cover malformed input, unroute/reroute behavior, ordering, authorization, default-off switch, and rollback refusal.
- [x] Keep the capability default-off; this task performs no enablement or deployment.

## Ownership

```yaml
owned_paths:
  - routes/internal.php
  - app/Http/Controllers/GameAuth/NativeScopeRevocationController.php
  - app/GameAuth/NativeRuntimeStatus/NativeScopeRevocationReport.php
  - app/GameAuth/NativeRuntimeStatus/NativeScopeAssignmentIngestion.php
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusIngestion.php
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusReadModel.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - database/migrations/2026_11_05_090000_add_native_scope_revocations.php
  - tests/Feature/GameAuth/NativeRuntimeStatus/NativeScopeRevocationIngestionTest.php
  - docs/agents/tasks/active/OTERYN-20261105-plat-scope-revoke-1.md
modules:
  - GameAuth native runtime status
dependencies:
  - Game oteryn-game-native-runtime-status-v1 §16.1
  - Oteryn/Oteryn-Platform#1419
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-04T00:00:00Z
head: UNKNOWN
branch: claude/happy-davinci-ijz3wy-plat-scope-revoke-1-v2
pr: 1676
status: ready
context_routes:
  - n/a
owned_paths:
  - routes/internal.php
  - app/Http/Controllers/GameAuth/NativeScopeRevocationController.php
  - app/GameAuth/NativeRuntimeStatus/NativeScopeRevocationReport.php
  - app/GameAuth/NativeRuntimeStatus/NativeScopeAssignmentIngestion.php
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusIngestion.php
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusReadModel.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - database/migrations/2026_11_05_090000_add_native_scope_revocations.php
  - tests/Feature/GameAuth/NativeRuntimeStatus/NativeScopeRevocationIngestionTest.php
  - docs/agents/tasks/active/OTERYN-20261105-plat-scope-revoke-1.md
proven:
  - Game contract §16.1 and Platform #1419 define the ingestion boundary.
derived:
  - Revocations share the existing assignment ordering and epoch lock.
unknown:
  - PHPUnit validation result (left for CI by task constraint)
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - owned paths only
validation:
  - command: git diff --check
    result: PASS
    evidence: no whitespace errors
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261105-plat-scope-revoke-1.md --require-checkpoint
    result: PASS
    evidence: validated against checkpoint contract v1
blockers:
  - none
next_action: run the focused PHPUnit feature test in CI and review the pull request
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository task branch
source_branch_evidence: pending PR merge
```

## Notes

The route remains default-off through the existing `game-auth.native_scope_assignment.enabled` switch. No environment enablement or deployment is part of this task. Jira synchronization is pending/unknown; ordinary repository workers do not write Jira.
