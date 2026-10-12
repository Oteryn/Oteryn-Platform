---
task_id: OTERYN-20261012-decanary-acceptance
governing_issue: 1474
required_reads: []
search_first:
  - scripts/acceptance/assert-platform-state.php
optional_reads: []
---

# OTERYN-20261012 decanary acceptance

## Goal

Governing GitHub Issue: #1474 (Platform Canary retirement umbrella; coordination Oteryn/Oteryn-Game#1622, DC-P9 / W-INFRA acceptance part) - canonical lifecycle authority for this task. Remove live Canary dependencies from acceptance tooling.

Finding: PR #1531 already removed the `canary_acceptance` service, Canary database, principals and runtime Redis ACL from the acceptance bootstrap, docker harness and every acceptance workflow. The only remaining live Canary database use was the unused `character-owner` command in `assert-platform-state.php`, removed here. The remaining `IdentityCanaryAccount` seeds and `canary_player_id` assertions exercise live `app/**` paths (account overview, marketplace, character profile preferences) and stay until their code packets land.

## Acceptance criteria

- [x] No acceptance script opens the `canary` database connection.
- [x] Legacy token baseline lowered (ci_deploy 91 to 86), never raised.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - scripts/acceptance/**
  - .github/workflows/** (except ci.yml)
  - tests/Unit/**
  - tests/ci/**
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - docs/agents/tasks/archive/OTERYN-20261012-decanary-acceptance.md
modules:
  - ci
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Game#1622 (coordination only; Game repository read-only here)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T12:00:00Z
head: UNKNOWN
branch: claude/decanary-acceptance-20261012
pr: none
status: completed
context_routes:
  - ci
owned_paths:
  - scripts/acceptance/**
  - .github/workflows/**
  - tests/ci/**
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - docs/agents/tasks/archive/OTERYN-20261012-decanary-acceptance.md
proven:
  - Workflows contain no canary_acceptance service or CANARY_* variable on current main.
  - character-owner had no caller.
derived:
  - Remaining acceptance Canary tokens (IdentityCanaryAccount seeds, canary_player_id, guard and denylist text) depend on live app code and are kept.
unknown:
  - none
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - scripts/acceptance/assert-platform-state.php
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - docs/agents/tasks/archive/OTERYN-20261012-decanary-acceptance.md
validation:
  - command: php -l scripts/acceptance/assert-platform-state.php
    result: PASS
    evidence: no syntax errors
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline
blockers:
  - none
next_action: Await exact-head CI.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: merged feature branch has no retention or recovery purpose after protected merge
source_branch_evidence: branch claude/decanary-acceptance-20261012 is deleted automatically after the protected merge
```
