---
task_id: OTERYN-20261011-decanary-deploy-1a
governing_issue: 1476
required_reads: []
search_first:
  - docs/agents/programs/DECANARY_REMOVAL_PLAN.md
optional_reads: []
---

# OTERYN-20261011 DECANARY-DEPLOY-1a

## Goal

Governing GitHub Issue: #1476 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Phase D D-4 of `docs/agents/programs/DECANARY_REMOVAL_PLAN.md`. Make the deploy, rollback, health and preflight scripts (including `deploy-impact.sh`), the Synology workflows (image pin, staging, Bazaar staging, schema recovery) and their tests treat the legacy game-engine service, its variables and the `canary_image` workflow input as optional. Present in the compose file: current behaviour. Absent from the compose file: skipped, not validated. The topology is read from `deploy/synology/compose.yml` only. No service is removed and the compose topology is unchanged (that is D-5).

## Acceptance criteria

- [x] Scripts and workflows gate the engine-specific checks on the compose topology and keep the present-topology behaviour.
- [x] Tests cover both topologies (`deploy/synology/tests`, `tests/ci/test_synology_*`).
- [ ] Legacy-token ratchet does not rise (see Notes; owner decision requested).
- [ ] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/**
  - deploy/synology/tests/**
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - tests/ci/test_synology_*
  - docs/agents/tasks/active/OTERYN-20261011-decanary-deploy-1a.md
  - docs/agents/tasks/archive/OTERYN-20261011-decanary-deploy-1a.md
modules:
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
head: UNKNOWN
branch: claude/decanary-deploy-1a-20261011
pr: 1529
status: validating
context_routes:
  - deploy-synology
owned_paths:
  - deploy/synology/scripts/**
  - deploy/synology/tests/**
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - tests/ci/test_synology_*
  - docs/agents/tasks/active/OTERYN-20261011-decanary-deploy-1a.md
proven:
  - python -m pytest tests/ci deploy/synology/tests passes locally
derived:
  - Character Bazaar enable and verify need the legacy database, so they fail closed without the service; prepare-rollback still works
unknown:
  - behaviour on the live Synology host for either topology (not runnable locally)
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - deploy/synology/scripts
  - deploy/synology/tests
  - .github/workflows
  - tests/ci
validation:
  - command: python -m pytest tests/ci deploy/synology/tests
    result: PASS
    evidence: local run
  - command: python tools/validation/legacy_token_ratchet.py
    result: FAIL
    evidence: ci_deploy group rises because optional branches keep the present-topology lines and add gating lines; owner decision requested
blockers:
  - none
next_action: Archive this record once the PR is closed.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR
source_branch_evidence: pending
```

## Notes

Not validated locally: a live staging deploy, rollback and Bazaar run for either topology. The smaller service-list workflows (repair-synology-compose-orphans, synology-container-hygiene, synology-diagnostics, repair-synology-autostart, synology-production-target-preflight) are left for D-5 because they list services statically and are not required by D-4.
