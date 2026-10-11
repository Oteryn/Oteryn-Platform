---
task_id: OTERYN-20261008-native-login-default
governing_issue: 1476
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/agents/programs/OTERYN_PLATFORM_COMPLETION_LEAD_PROGRAMME.md
search_first:
  - config/game-auth.php
  - services/game-gateway/internal/config/config.go
optional_reads:
  - docs/agents/tasks/archive/OTERYN-20261006-lcfa-consumer.md
---

# OTERYN-20261008 native login default rollout

## Goal

Under Issue #1476 (PR #1479), default the native account characters (LCFA) read path to enabled in preproduction in Platform `config/game-auth.php`, so the Platform side no longer depends on a per-environment toggle.

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
  - docs/agents/tasks/archive/OTERYN-20261008-native-login-default.md
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
updated_at: 2026-10-11T08:00:00Z
status: completed
branch: claude/decanary-login-default-1-20261008
head: e3446d5e0d65921ba1636256b151eef54619c011
pr: 1479
context_routes:
  - game-auth
  - testing
owned_paths:
  - config/game-auth.php
  - services/game-gateway/internal/config/config.go
  - services/game-gateway/internal/config/config_test.go
  - docs/agents/tasks/archive/OTERYN-20261008-native-login-default.md
proven:
  - PR #1479 touches exactly four paths: config/game-auth.php, services/game-gateway/internal/config/config.go, services/game-gateway/internal/config/config_test.go and this task packet (docs/agents/tasks/archive/OTERYN-20261008-native-login-default.md).
  - Issue #1476 is open and governs this rollout; the LCFA packet governs #1419 only.
  - With GATEWAY_NATIVE_LOGIN_ENABLED unset the gateway keeps native login off (config.go and config_test.go).
  - On head e3446d5 the checkpoint-validation, test, runtime-tests, platform-gate and Build game-gateway component checks passed.
derived:
  - The Platform default change is limited to preproduction and leaves production behavior unchanged.
  - The only failing check on head e3446d5, acceptance, failed in a WebKit portal portability test (editorial-media-acceptance, WebKit internal error loading portal CSS) that this diff does not touch.
unknown:
  - Exact-head CI result for the head that carries this record (the earlier test, platform-gate and classify-changes failures were the legacy-token ratchet, fixed by token-free names); this record commit is a child of the head recorded above.
  - Whether the preproduction gateway deployment already sets GATEWAY_NATIVE_LOGIN_ENABLED.
  - Whether the acceptance WebKit failure is flaky; a rerun on the new head decides.
conflicts: []
first_failure:
  marker: acceptance
  evidence: head e3446d5 acceptance job failed with 1 failed portability-webkit editorial-media-acceptance test and unexpected WebKit console errors; unrelated to the owned paths
rejected_hypotheses:
  - Extend the LCFA packet ownership to services/game-gateway.
  - Treat the gateway as defaulting native login on in preproduction.
  - Attribute the acceptance failure to the config default change.
changed_paths:
  - config/game-auth.php
  - services/game-gateway/internal/config/config.go
  - services/game-gateway/internal/config/config_test.go
  - docs/agents/tasks/archive/OTERYN-20261008-native-login-default.md
validation:
  - command: go test ./internal/config/... (services/game-gateway)
    result: NOT_RUN
    evidence: validation is delegated to CI on the PR head
  - command: python3 tools/agents/checkpoint.py and task liveness validation
    result: PASS
    evidence: checkpoint-validation check succeeded on head e3446d5
  - command: acceptance (portal portability e2e)
    result: FAIL
    evidence: WebKit editorial-media-acceptance failure on head e3446d5, outside the owned paths; rerun required
blockers:
  - production release only: Decision P1/U12
  - gateway deployment opt-in via GATEWAY_NATIVE_LOGIN_ENABLED
next_action: none; PR #1479 merged and record archived.
```

## Source branch closeout

Every task branch must have an intentional terminal disposition. This section is closeout evidence and does not add fields to the shared Context checkpoint contract.

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: merged feature branch has no retention or recovery purpose after protected merge
source_branch_evidence: PR #1479 merged (REST pulls/1479 state closed, merged true) as 378e194ae9b2d682c4e006203b495ee68aaa3f6c at 2026-10-11T03:05:54Z from exact head edd3e01817143fe2a265485195d3510eb9ae22d4; source branch (claude/decanary-login-default-1-20261008) absent (REST branches API returns 404)
```
