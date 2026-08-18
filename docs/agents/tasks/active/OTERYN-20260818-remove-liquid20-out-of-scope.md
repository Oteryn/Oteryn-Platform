---
task_id: OTERYN-20260818-remove-liquid20-out-of-scope
status: active
repository: blakinio/Oteryn-Platform
implementation_authorized: true
owner_direction: "Liquid20 and Freqtrade operational/control-plane material must not be part of Oteryn Platform."
---

# Remove Liquid20 from Oteryn Platform scope

## Goal

Correct an established scope violation: remove active Liquid20/Freqtrade operational code, workflow registration and migration-test coupling from `blakinio/Oteryn-Platform`.

The repository's canonical scope lock already excludes Freqtrade and other product programmes. This task enforces that rule in the current executable tree.

## Acceptance criteria

- [ ] remove `.github/workflows/liquid20-synology-control.yml`;
- [ ] remove `deploy/liquid20/**` operational/control-plane assets;
- [ ] remove the Liquid20 workflow from the CI lifecycle registry and reduce the workflow budget accordingly;
- [ ] remove Liquid20 from Platform transfer/GHCR regression tests;
- [ ] keep Platform GHCR/runner hardening for actual Platform components unchanged;
- [ ] do not mutate Freqtrade, Synology runtime, packages, runners, staging or production;
- [ ] historical Git commits remain provenance and are not rewritten;
- [ ] exact-head required CI/governance pass before merge.

## Ownership

```yaml
owned_paths:
  - .github/workflows/liquid20-synology-control.yml
  - deploy/liquid20/**
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/tasks/active/OTERYN-20260818-remove-liquid20-out-of-scope.md
modules:
  - repository-scope
  - ci-lifecycle
  - synology-transfer-contract-tests
external_write_required: false
live_operation_required: false
```

## Evidence

- canonical `docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md` explicitly excludes Freqtrade and other product programmes;
- current tree nevertheless contains an active Liquid20 workflow and `deploy/liquid20/**` control assets;
- PR #1153 incorrectly treated Liquid20 as Platform-owned during repository-transfer hardening;
- owner explicitly corrected the boundary on 2026-08-18.

## Stop condition

Do not move or recreate this material in another repository under this task. This is a Platform scope correction only.
