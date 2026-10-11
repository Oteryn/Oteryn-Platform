---
task_id: OTERYN-20261011-staging-native-defaults-1
governing_issue: 1474
required_reads: []
search_first:
  - deploy/synology/scripts/lib.sh
  - .github/workflows/deploy-synology-staging.yml
optional_reads: []
---

# OTERYN-20261011 STAGING-NATIVE-DEFAULTS-1

## Goal

Governing GitHub Issue: #1474 (native-only migration). The Synology staging deploy must not depend on GitHub repository variables set by hand. Committed non-secret defaults live in `deploy/synology/native-defaults.env`; a repository variable of the same name still overrides. Values that cannot be known before the first route publish stay blank and the native admission gate fails closed with a message naming the setting. Native admission stays default-off.

## Acceptance criteria

- [x] `OTERYN_STAGING_NATIVE_ADMISSION_ENABLED` has a committed default of `false`; the other three native settings are blank by design.
- [x] Deploy and Bazaar control workflows resolve all four through `oteryn_native_default` (variable > file > empty).
- [x] No secret, key or model identifier added; `docs/agents/LEGACY_TOKEN_BASELINE.json` untouched.
- [x] `tests/ci/test_synology_native_admission_gate.py` passes.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - deploy/synology/native-defaults.env
  - deploy/synology/scripts/lib.sh
  - deploy/synology/README.md
  - tests/ci/test_synology_native_admission_gate.py
  - .github/workflows/build-synology-staging-images.yml
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deployment_package_routing.py
  - docs/agents/tasks/active/OTERYN-20261011-staging-native-defaults-1.md
  - docs/agents/tasks/archive/OTERYN-20261011-staging-native-defaults-1.md
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
updated_at: 2026-10-11T18:00:00Z
head: UNKNOWN
branch: claude/staging-native-defaults-1-20261011
pr: 1552
status: validating
terminal_pr_policy: archive_pending
context_routes:
  - deploy/synology
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - deploy/synology/**
  - tests/ci/test_synology_native_admission_gate.py
  - .github/workflows/build-synology-staging-images.yml
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deployment_package_routing.py
proven:
  - Native admission gate tests pass locally (39 tests).
derived: []
unknown:
  - Exact-head CI result.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - deploy/synology/native-defaults.env
  - deploy/synology/scripts/lib.sh
  - deploy/synology/README.md
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/build-synology-staging-images.yml
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deployment_package_routing.py
  - tests/ci/test_synology_native_admission_gate.py
validation:
  - command: python3 -m pytest -q tests/ci/test_synology_native_admission_gate.py
    result: PASS
    evidence: all tests passed locally
blockers:
  - none
next_action: Archive this task record; PR #1552 is terminal.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: pending
```

## Notes

Scope note: native-defaults.env is a runtime deployment input, so the image build workflow, its build classifier and the deployment package routing test are owned here to route it as a deployment-package change rather than a component image rebuild.

Still owner or after first deploy: native World id, runtime-status and scope-assignment identity maps (need WorldId/ChannelId from the first route publish), then flipping admission to true. Set them by editing the defaults file in a PR, or by repository variable override.
