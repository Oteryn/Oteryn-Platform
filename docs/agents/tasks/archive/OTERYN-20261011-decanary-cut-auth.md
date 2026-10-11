---
task_id: OTERYN-20261011-decanary-cut-auth
governing_issue: 1474
required_reads: []
search_first:
  - app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php
optional_reads: []
---

# OTERYN-20261011 DECANARY-CUT-AUTH

## Goal

Governing GitHub Issue: #1474 — canonical lifecycle authority for this task.

Remove the Canary game login path directly (the environment is not production: no dual-compat and no nullable stage). Native ticket issuance and the ticket model stay. The Game Gateway becomes native-only.

## Acceptance criteria

- [x] GameAuth legacy v1 path, redeem and login-context routes, the Canary branch of ticket issuance and the Canary-keyed world registry are removed.
- [x] Account overview shows native account characters and no binding state; registration creates no Canary binding; the provisioning command is unscheduled.
- [x] Gateway serves native admission only; the session client and protocol v1 branch are removed.
- [x] Local validation passes (phpunit, pint, phpstan, go vet, go test, legacy token ratchet).
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/
  - app/Accounts/
  - app/Identity/
  - app/Http/Controllers/GameAuth/
  - app/Http/Controllers/Accounts/
  - app/Providers/AppServiceProvider.php
  - config/game-auth.php
  - routes/
  - resources/views/identity/account/
  - lang/
  - services/game-gateway/
  - tests/Feature/Accounts/
  - tests/Feature/GameAuth/
  - tests/Feature/Identity/
  - docs/agents/tasks/archive/OTERYN-20261011-decanary-cut-auth.md
modules:
  - game auth
  - game gateway
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
updated_at: 2026-10-11T10:00:00Z
head: 9cf89cd0e9ab375f1c373903c464cb8ba43ade91
branch: claude/decanary-cut-auth-20261011
pr: 1537
status: completed
phase: terminal_closeout
terminal_pr_policy: archive_pending
context_routes:
  - game-auth
owned_paths:
  - app/GameAuth/
  - app/Accounts/
  - app/Identity/
  - services/game-gateway/
  - docs/agents/tasks/archive/OTERYN-20261011-decanary-cut-auth.md
proven:
  - phpunit Accounts, Identity, GameAuth and Unit suites pass; pint and phpstan pass; go vet and go test pass.
derived: []
unknown:
  - Exact-head CI result.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - app/GameAuth/
  - app/Accounts/
  - app/Identity/
  - services/game-gateway/
  - docs/agents/tasks/archive/OTERYN-20261011-decanary-cut-auth.md
validation:
  - command: python tools/agents/checkpoint.py docs/agents/tasks/archive/OTERYN-20261011-decanary-cut-auth.md --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
blockers:
  - none
next_action: archive this record after PR #1537 reaches its terminal state; no further implementation.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: merged feature branch has no retention or recovery purpose after protected merge
source_branch_evidence: pending protected merge of PR #1537
```

## Notes

Shims left for other workers: CanaryAccountProvisioningGateway contract and exceptions, IdentityCanaryAccount model, provisioning security-event constants and the AppServiceProvider binding are still used by app/CanaryIntegration, Marketplace and CharacterProfiles.
