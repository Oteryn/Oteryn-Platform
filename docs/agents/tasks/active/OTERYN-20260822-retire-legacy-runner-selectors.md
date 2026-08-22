---
task_id: OTERYN-20260822-retire-legacy-runner-selectors
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
search_first:
  - runs-on: oteryn-staging
optional_reads: []
---

# OTERYN-20260822-retire-legacy-runner-selectors

## Goal

Move every retained Platform workflow from the legacy `oteryn-staging` runner selector to the proven repository-scoped organization route before final legacy retirement.

## Acceptance criteria

- [ ] No retained Platform workflow uses `runs-on: oteryn-staging`.
- [ ] All migrated jobs use `platform-runners` plus `oteryn-platform`.
- [ ] Compose project/state/runtime identifiers remain unchanged.
- [ ] Exact-head CI passes and the PR merges normally.
- [ ] Legacy runner is retired only after Platform trusted-main acceptance PASS.

## Ownership

```yaml
owned_paths:
  - .github/workflows/*.yml
  - docs/agents/tasks/active/OTERYN-20260822-retire-legacy-runner-selectors.md
modules:
  - ci-runner-routing
dependencies:
  - Oteryn/Oteryn-Platform#1217
  - Oteryn/Oteryn#34
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn#34
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T10:09:30Z
head: df8078a10a757d03e64aa4f0c26767bc5ec496cc
branch: ci/issue-1220-retire-legacy-selectors
pr: 1221
status: validating
context_routes:
  - ci-repair
  - testing
owned_paths:
  - .github/workflows/*.yml
  - docs/agents/tasks/active/OTERYN-20260822-retire-legacy-runner-selectors.md
proven:
  - Atlas trusted-main replacement route is PASS.
  - Game trusted-main replacement run 32566399984 is PASS.
  - Direct organization ACL seal proved exact selected repository per runner group.
  - Eight retained Platform workflows still targeted legacy runs-on oteryn-staging on main before this task.
derived:
  - Legacy retirement is unsafe until these selectors move to the Platform organization runner route.
unknown:
  - Final Platform trusted-main diagnostics result after bounded docker-system-df repair.
conflicts: []
first_failure:
  marker: retained-legacy-selectors
  evidence: eight active Platform workflow files matched exact legacy runs-on selector
rejected_hypotheses:
  - Compose project name oteryn-staging is not a runner selector and must remain unchanged.
changed_paths:
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/playwright-runtime-validation.yml
  - .github/workflows/recover-synology-staging-schema.yml
  - .github/workflows/repair-synology-autostart.yml
  - .github/workflows/repair-synology-compose-orphans.yml
  - .github/workflows/synology-container-hygiene.yml
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/synology-diagnostics.yml
validation:
  - command: python tools/validation/workflow_inventory.py
    result: PASS
    evidence: 53 registered workflows, budget 53, lifecycle PASS
  - command: exact legacy runs-on selector scan
    result: PASS
    evidence: NO_LEGACY_RUNS_ON=PASS on task branch
blockers:
  - none
next_action: Validate PR #1221 exact head with bounded diagnostics, merge normally, then rerun trusted-main proof and retire legacy if green.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary bounded runner routing migration
source_branch_evidence: pending
```
