---
task_id: OTERYN-20261012-dc-p11-login-leftovers
governing_issue: 1474
required_reads: []
search_first:
  - scripts/validate_native_protocol_change_boundary.py
optional_reads: []
---

# OTERYN-20261012 login leftovers

## Goal

Governing GitHub Issue: #1474 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Remove the last v1 game-login leftovers after the legacy login controller and routes were deleted: the stale controller allowance in the native protocol change boundary script and the dead redeem-path entry in the response-caching middleware and its test.

## Acceptance criteria

- [x] `PreventSensitiveGameAuthResponseCaching` is kept: it still guards the live `POST /v1/game-auth/tickets` route, the device-session routes and `GuardCharacterCommandPeer`. Only the dead `internal/v1/game-auth/tickets/redeem` path entry and its test case are removed.
- [x] `GameLoginContextController.php` allowance removed from `scripts/validate_native_protocol_change_boundary.py`; no script test referenced it.
- [x] `RemovedLegacyLoginRoutesTest` is unchanged.
- [x] Legacy token baseline is unchanged: no token count dropped.
- [ ] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - tests/Unit/Http/Middleware/PreventSensitiveGameAuthResponseCachingTest.php
  - scripts/validate_native_protocol_change_boundary.py
  - docs/agents/tasks/archive/OTERYN-20261012-dc-p11-login-leftovers.md
modules:
  - game-auth
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
updated_at: 2026-10-12T09:00:00Z
head: UNKNOWN
branch: claude/dc-p11-login-leftovers-20261012
pr: UNKNOWN
status: validating
context_routes:
  - game-auth
owned_paths:
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - tests/Unit/Http/Middleware/PreventSensitiveGameAuthResponseCachingTest.php
  - scripts/validate_native_protocol_change_boundary.py
  - docs/agents/tasks/archive/OTERYN-20261012-dc-p11-login-leftovers.md
proven:
  - the middleware is applied in routes/api.php to the live ticket issue route and the device-session group
  - the redeem route no longer exists
derived:
  - none
unknown:
  - PHPUnit result until CI runs on this head
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - tests/Unit/Http/Middleware/PreventSensitiveGameAuthResponseCachingTest.php
  - scripts/validate_native_protocol_change_boundary.py
  - docs/agents/tasks/archive/OTERYN-20261012-dc-p11-login-leftovers.md
validation:
  - command: legacy_token_ratchet, checkpoint, php -l
    result: PASS
    evidence: local run
blockers:
  - none
next_action: none; archived with the terminal PR.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR
source_branch_evidence: pending
```

## Notes

The active record OTERYN-20261011-login-native-only claims a `game-auth.legacy_login.enabled` switch that is absent from `config/game-auth.php`; that record belongs to another packet and is left unchanged here.
