---
task_id: OTERYN-20261011-native-only-cut-infra
governing_issue: 1474
required_reads: []
search_first:
  - deploy/synology/compose.yml
optional_reads: []
---

# OTERYN-20261011 native-only infra cut

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Packets D-3 to D-6 of the removal plan under `docs/agents/programs/`, cut directly because the environment is not production.

The Canary container, database, environment, TLS alias, `canary_image` input and `CANARY_*` variables are removed from the Synology deploy tooling, tests and workflows. The acceptance bootstrap and acceptance workflows no longer provision a Canary database, principals or runtime Redis ACL. The MQ doc renames and the lifecycle registry change are left to PR #1527.

## Acceptance criteria

- [x] Compose, nginx, TLS init, env example, release-state, deploy, deploy-impact, rollback, health and preflight scripts have no Canary service or variable.
- [x] Synology workflows drop `canary_image` and the Canary secrets; the build workflow dispatches `game_world_host`.
- [x] Acceptance bootstrap, docker harness and acceptance workflows drop the Canary database and runtime Redis principal.
- [x] CI and Synology contract tests updated; legacy token ratchet passes.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - deploy/synology/**
  - .github/workflows/**
  - scripts/acceptance/**
  - tests/ci/**
  - docs/agents/tasks/archive/OTERYN-20261011-native-only-cut-infra.md
modules:
  - ci
  - deploy
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
head: 2a9f9ddc2d1441cc3df65e05763ff97199bf95f5
branch: claude/decanary-cut-infra-20261011
pr: 1531
status: completed
context_routes:
  - ci
  - deploy
owned_paths:
  - deploy/synology/**
  - .github/workflows/**
  - scripts/acceptance/**
  - tests/ci/**
  - docs/agents/tasks/archive/OTERYN-20261011-native-only-cut-infra.md
proven:
  - pytest tests/ci deploy/synology/tests tools/validation passes locally (349 passed).
  - docker compose config renders for compose.yml and the marketplace overlay.
  - Legacy token ratchet passes locally.
derived:
  - Release-state slots after the removed image shift down by one in the fast-deploy and resume paths.
  - Release state written before the cut (retired game-server image line, or legacy-format state) is rollback-ineligible; rollback and schema recovery refuse it.
  - Character Bazaar enable and verify are refused until a native transfer and escrow contract exists; prepare-rollback stays available.
unknown:
  - Acceptance specs and coverage manifests that assert Canary-sourced data (public game data, community data, resilience, character-owner checks) are unchanged and need native fixtures or an unavailable state from the C-phase; they are not run locally.
  - Gateway startup without the game-session issuer configuration depends on the gateway packet (D-7).
  - game-catalog-contract and native-protocol-contract-audits keep their Canary cross-repository proofs (catalog and protocol packets).
  - The legacy token baseline file is outside the owned paths and can be lowered separately.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - deploy/synology/**
  - .github/workflows/**
  - scripts/acceptance/bootstrap-production-like.sh
  - scripts/acceptance/docker/compose.yml
  - scripts/acceptance/docker/run.sh
  - tests/ci/**
  - docs/agents/tasks/archive/OTERYN-20261011-native-only-cut-infra.md
validation:
  - command: python -m pytest tests/ci deploy/synology/tests tools/validation
    result: PASS
    evidence: 359 passed
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline
  - command: docker compose config for compose.yml and compose.marketplace.yml
    result: PASS
    evidence: both render
  - command: acceptance browser suites and staging deploy
    result: NOT_RUN
    evidence: need CI runners and the Synology staging host
blockers:
  - none
next_action: Archive this record once the PR is closed.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: merged feature branch has no retention or recovery purpose after protected merge
source_branch_evidence: PR #1531 merge pending; branch claude/decanary-cut-infra-20261011 is deleted automatically after the protected merge
```
