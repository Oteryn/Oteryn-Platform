---
task_id: OTERYN-20261011-login-native-only
governing_issue: 1474
required_reads: []
search_first:
  - app/GameAuth/Tickets/IssueGameLoginTicket.php
optional_reads: []
---

# OTERYN-20261011 login native-only

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Packet B7 (LOGIN-NATIVE-ONLY-1) of the removal plan under `docs/agents/programs/`.

The deployed gateway runs in native-only mode, the Platform v1 login-context route answers 410, and the Platform ticket issuer gains an explicit native-only switch. Binding-free native issuance was delivered earlier by #1484 and is re-proved here.

## Acceptance criteria

- [x] `deploy/synology/compose.yml` gateway service sets `GATEWAY_NATIVE_LOGIN_ENABLED=true` and passes no legacy session service variables (native-only mode of `services/game-gateway/internal/config`).
- [x] `GET /internal/v1/game-auth/accounts/{legacyAccountId}/login-context` answers 410 behind the gateway credential and reads no data.
- [x] `game-auth.legacy_login.enabled` (env `GAME_AUTH_LEGACY_LOGIN_ENABLED`, default on) set to false makes the legacy-client issuance branch deny without consuming the bearer; native issuance is unaffected and issues for an Identity without a legacy binding.
- [x] Tests: retired route, switch-off denial, native issuance in native-only mode.
- [x] The legacy token ratchet passes with no group rising.
- [ ] Exact-head CI passes (PHP tests were not runnable in the authoring sandbox: PHP 8.5 is required).

## Ownership

```yaml
owned_paths:
  - deploy/synology/compose.yml
  - routes/internal.php
  - config/game-auth.php
  - app/GameAuth/Tickets/IssueGameLoginTicket.php
  - app/GameAuth/OAuth/**
  - tests/Feature/GameAuth/**
  - docs/agents/tasks/active/OTERYN-20261011-login-native-only.md
  - docs/agents/tasks/archive/OTERYN-20261011-login-native-only.md
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
updated_at: 2026-10-11T12:00:00Z
head: UNKNOWN
branch: claude/decanary-b7-login-native-only-20261011
pr: 0
status: validating
context_routes:
  - game-auth
owned_paths:
  - deploy/synology/compose.yml
  - routes/internal.php
  - config/game-auth.php
  - app/GameAuth/Tickets/IssueGameLoginTicket.php
  - app/GameAuth/OAuth/**
  - tests/Feature/GameAuth/**
  - docs/agents/tasks/active/OTERYN-20261011-login-native-only.md
  - docs/agents/tasks/archive/OTERYN-20261011-login-native-only.md
proven:
  - Gateway config already implements native-only mode (native login on, both session variables unset); compose now selects it.
  - Native issuance is chosen by client type in `IssueGameLoginTicketFromOAuth` and writes no legacy binding reference (#1484).
derived:
  - The retired route uses a route closure; the controller is orphaned until D-8 deletes it. Route caching is not used by any deploy or CI path.
unknown:
  - PHP suite result; composer cannot install here (PHP 8.5 required), so CI is the first run.
  - Follow-up B7-DEPLOY: set `GAME_AUTH_LEGACY_LOGIN_ENABLED=false` in each Platform deployment (compose platform service and preproduction/production env are outside the owned paths); until then the switch defaults on.
  - Follow-up B7-SECRETS: `deploy/synology/scripts/deploy*.sh`, workflows and `.env.example` still require the legacy session service token; removal belongs to D-4/D-5.
  - Follow-up B7-E2E: `tests/e2e/native_auth_ephemeral_cutover` still starts the gateway in legacy mode; a native-only run belongs to D-1..D-3 or a gateway packet.
  - Follow-up D-8: delete `GameLoginContextController`, the retired route and the legacy issuance branch; `scripts/validate_native_protocol_change_boundary.py` lists the controller by name.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - deploy/synology/compose.yml
  - routes/internal.php
  - config/game-auth.php
  - app/GameAuth/Tickets/IssueGameLoginTicket.php
  - tests/Feature/GameAuth/GameLoginContextApiTest.php
  - tests/Feature/GameAuth/RetiredLoginContextTest.php
  - tests/Feature/GameAuth/GameLoginTicketApiTest.php
  - tests/Feature/GameAuth/OAuth/NativeOAuthNativeTicketTest.php
  - docs/agents/tasks/active/OTERYN-20261011-login-native-only.md
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261011-login-native-only.md --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: product runtime E2E
    result: NOT_RUN
    evidence: native E2E and gateway CI run on the exact head in CI
blockers:
  - none
next_action: Read exact-head CI; open the deployment follow-ups named above.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
