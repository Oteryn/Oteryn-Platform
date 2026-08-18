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

- [x] remove `.github/workflows/liquid20-synology-control.yml`;
- [x] remove `deploy/liquid20/**` operational/control-plane assets;
- [x] remove the Liquid20 workflow from the CI lifecycle registry and reduce the workflow budget accordingly;
- [x] remove Liquid20 from Platform transfer/GHCR regression tests;
- [x] keep Platform GHCR/runner hardening for actual Platform components unchanged;
- [x] do not mutate Freqtrade, Synology runtime, packages, runners, staging or production;
- [x] historical Git commits remain provenance and are not rewritten;
- [ ] exact-head required CI/governance pass before merge.

## Ownership

```yaml
owned_paths:
  - .github/workflows/liquid20-synology-control.yml
  - deploy/liquid20/**
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-platform-transfer-inventory.json
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
- the pre-correction tree nevertheless contained an active Liquid20 workflow and `deploy/liquid20/**` control assets;
- PR #1153 incorrectly treated Liquid20 as Platform-owned during repository-transfer hardening;
- owner explicitly corrected the boundary on 2026-08-18.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-18T15:06:00Z
head: d482564631c2085648a11cb7cfdbed5c2f2ef947
branch: fix/remove-liquid20-from-platform-scope
pr: 1156
status: validating
context_routes:
  - agent-governance
  - testing
  - ci-repair
  - architecture
owned_paths:
  - .github/workflows/liquid20-synology-control.yml
  - deploy/liquid20/**
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-platform-transfer-inventory.json
  - docs/agents/tasks/active/OTERYN-20260818-remove-liquid20-out-of-scope.md
proven:
  - The canonical Platform programme scope excludes Freqtrade and other non-WWW product programmes.
  - PR 1156 removes the active Liquid20 workflow and deploy/liquid20 operational control assets from the Platform tree.
  - Workflow lifecycle validation on head d482564631c2085648a11cb7cfdbed5c2f2ef947 passed with exactly 52 registered workflows and budget 52.
  - Focused Synology transfer contract tests on head d482564631c2085648a11cb7cfdbed5c2f2ef947 passed 15 tests including the out-of-scope operational-asset absence assertion.
  - Platform transfer readiness and machine-readable inventory no longer classify Liquid20 as a current Platform-owned component.
  - No Freqtrade repository write package mutation runner registration secret mutation Synology runtime operation staging operation or production operation occurred.
derived:
  - The implementation failure is governance metadata only and does not invalidate the scope-removal code or lifecycle result.
  - Historical Git and archived task references may retain Liquid20 as truthful provenance without restoring active Platform ownership.
unknown: []
conflicts: []
first_failure:
  marker: missing-context-checkpoint-and-live-identity
  evidence: CI run 32151999974 classify-changes job 95759905729 and Agent Governance run 32151999892 job 95759904462
rejected_hypotheses:
  - Liquid20 must remain registered to satisfy workflow inventory; lifecycle validation passed after unregistering it.
  - The focused Platform transfer tests require Liquid20; all focused tests passed after removing the coupling.
  - The correction requires writing to Freqtrade or mutating Synology; this task is repository-scope-only.
changed_paths:
  - .github/workflows/liquid20-synology-control.yml
  - deploy/liquid20/README.md
  - deploy/liquid20/publish-status.sh
  - deploy/liquid20/synology-control.sh
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/tasks/active/OTERYN-20260818-remove-liquid20-out-of-scope.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-platform-transfer-inventory.json
  - tests/ci/test_synology_deploy_release_identity.py
validation:
  - command: workflow lifecycle registry validation on d482564631c2085648a11cb7cfdbed5c2f2ef947
    result: PASS
    evidence: CI run 32151999974 reported actual 52 budget 52
  - command: focused Synology transfer contract tests on d482564631c2085648a11cb7cfdbed5c2f2ef947
    result: PASS
    evidence: CI run 32151999974 reported 15 tests OK including out-of-scope asset absence
  - command: active task checkpoint and live identity on d482564631c2085648a11cb7cfdbed5c2f2ef947
    result: FAIL
    evidence: task record lacked Context checkpoint and therefore exposed neither PR nor branch to liveness validation
  - command: runtime browser or live Synology E2E
    result: NOT_APPLICABLE
    evidence: scope correction removes inactive repository control material and performs no user-facing runtime or live environment operation
blockers: []
next_action: Validate the repaired checkpoint on the new exact PR head then self-review PR 1156 and squash-merge only if required CI governance and review hygiene are clean.
```

## Stop condition

Do not move or recreate this material in another repository under this task. This is a Platform scope correction only.

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: PR 1156 is validating after the governance checkpoint repair
source_branch_evidence: pending
```
