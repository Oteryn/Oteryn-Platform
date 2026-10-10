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

Under Issue #1476 (PR #1479), default the native account characters (LCFA) read path to enabled in preproduction in Platform `config/game-auth.php`, so the Platform side no longer depends on a per-environment canary toggle.

The Game gateway is deliberately different: with `GATEWAY_NATIVE_LOGIN_ENABLED` unset the gateway keeps native login OFF (`services/game-gateway/internal/config/config.go`, verified by `config_test.go`). Enabling native login on a gateway deployment therefore requires an explicit per-deployment opt-in through `GATEWAY_NATIVE_LOGIN_ENABLED`. That deployment setting is not part of this diff and remains an open item below.

This slice changes the Platform preproduction default only. It does not activate native production login, does not change the LCFA consumer implemented under Issue #1419 (PR #1465), and does not resolve Decision P1/U12 production freshness.

## Acceptance criteria

- [x] Platform `config/game-auth.php` defaults native account characters on in preproduction and stays off elsewhere.
- [x] Gateway config keeps native login off when `GATEWAY_NATIVE_LOGIN_ENABLED` is unset, and `config_test.go` covers that fail-closed default.
- [ ] The actual preproduction gateway deployment sets `GATEWAY_NATIVE_LOGIN_ENABLED` explicitly (deployment setting, outside this diff; owner/operator evidence required).
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
  - gateway native login needs an explicit per-deployment GATEWAY_NATIVE_LOGIN_ENABLED opt-in
cross_repository_tasks:
  - none; no Game repository write is authorized
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-10T14:30:00Z
status: validating
terminal_pr_policy: archive_pending
phase: pre_merge_validation
branch: claude/decanary-login-default-1-20261008
head: d398c28ac23732142788694c6f34750536c12f7c
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
  - With GATEWAY_NATIVE_LOGIN_ENABLED unset the gateway keeps native login off (config.go and config_test.go).
derived:
  - The Platform default change is limited to preproduction and leaves production behavior unchanged.
unknown:
  - Exact-head CI result for the head that carries this record.
  - Whether the preproduction gateway deployment already sets GATEWAY_NATIVE_LOGIN_ENABLED.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Extend the LCFA packet ownership to services/game-gateway.
  - Treat the gateway as defaulting native login on in preproduction.
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
  - gateway deployment opt-in via GATEWAY_NATIVE_LOGIN_ENABLED
next_action: Run exact-head CI for PR #1479, then hand the review packet to the lead; archive this record when the PR ends the task.
```

## Source branch closeout

Every task branch must have an intentional terminal disposition. This section is closeout evidence and does not add fields to the shared Context checkpoint contract.

```yaml
source_branch_disposition: pending # pending|auto_delete_after_merge|delete_on_close|retain
source_branch_reason: task is still active
source_branch_evidence: pending
```
