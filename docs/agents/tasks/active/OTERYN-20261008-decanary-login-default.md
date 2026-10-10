---
task_id: OTERYN-20261008-decanary-login-default
governing_issue: 1476
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/agents/programs/OTERYN_PLATFORM_COMPLETION_LEAD_PROGRAMME.md
search_first:
  - config/game-auth.php
  - services/game-gateway/internal/config/config.go
optional_reads:
  - docs/agents/tasks/active/OTERYN-20261006-lcfa-consumer.md
---

# OTERYN-20261008 DECANARY-LOGIN-DEFAULT-1 rollout

## Goal

Under Issue #1476 (PR #1479), default the native account characters (LCFA) read path to enabled in preproduction in both Platform `config/game-auth.php` and the Game gateway configuration in `services/game-gateway/internal/config`, so preproduction no longer depends on a per-environment canary toggle.

This slice changes preproduction defaults only. It does not activate native production login, does not change the LCFA consumer implemented under Issue #1419 (PR #1465), and does not resolve Decision P1/U12 production freshness.

## Acceptance criteria

- [x] Platform `config/game-auth.php` defaults native account characters on in preproduction and stays off elsewhere.
- [x] Game gateway config defaults match the Platform default and keep fail-closed behavior outside preproduction.
- [x] Gateway config tests cover the preproduction default and the non-preproduction off default.
- [ ] Focused Go config tests and PHP config checks pass.
- [ ] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - config/game-auth.php
  - services/game-gateway/internal/config/config.go
  - services/game-gateway/internal/config/config_test.go
  - docs/agents/tasks/active/OTERYN-20261008-decanary-login-default.md
modules:
  - GameAuth
  - GameGateway
dependencies:
  - Issue #1476
  - Issue #1419 LCFA consumer (PR #1465)
blockers:
  - production activation remains separately blocked by Decision P1/U12 measured freshness acceptance
cross_repository_tasks:
  - none; no Game repository write is authorized
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-10T14:00:00Z
status: validating
terminal_pr_policy: archive_pending
phase: pre_merge_validation
branch: claude/decanary-login-default-1-20261008
head: 96696ffbd6ca299322e1c08cf726339eaa4f545e
pr: 1479
context_routes:
  - game-auth
  - testing
owned_paths:
  - config/game-auth.php
  - services/game-gateway/internal/config/config.go
  - services/game-gateway/internal/config/config_test.go
  - docs/agents/tasks/active/OTERYN-20261008-decanary-login-default.md
proven:
  - PR #1479 touches only config/game-auth.php and the game-gateway config package and its test.
  - Issue #1476 is open and governs this rollout; the LCFA packet governs #1419 only.
derived:
  - The default change is limited to preproduction and leaves production behavior unchanged.
unknown:
  - Exact-head CI result for the head that carries this record.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Extend the LCFA packet ownership to services/game-gateway.
changed_paths:
  - config/game-auth.php
  - services/game-gateway/internal/config/config.go
  - services/game-gateway/internal/config/config_test.go
  - docs/agents/tasks/active/OTERYN-20261008-decanary-login-default.md
validation:
  - command: go test ./internal/config/... (services/game-gateway)
    result: NOT_RUN
    evidence: validation is delegated to CI on the PR head
  - command: python3 tools/agents/checkpoint.py and task liveness validation
    result: NOT_RUN
    evidence: validation is delegated to CI on the PR head
blockers:
  - production release only: Decision P1/U12
next_action: Run exact-head CI for PR #1479, then hand the review packet to the lead; archive this record when the PR ends the task.
```
