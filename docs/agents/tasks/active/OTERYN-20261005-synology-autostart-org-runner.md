---
task_id: OTERYN-20261005-synology-autostart-org-runner
governing_issue: 1339
required_reads:
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/archive/OTERYN-20260822-retire-legacy-runner-selectors.md
  - deploy/synology/runner/compose.organization.example.yml
search_first:
  - repair-synology-autostart
  - oteryn-synology-platform
  - oteryn-synology-staging
optional_reads:
  - deploy/synology/README.md
---

# OTERYN-20261005 Synology autostart organization-runner reconciliation

## Goal

Repair Issue #1339's stale runner-container assumption without touching the protected Synology environment. Repository evidence proves the legacy Platform repository runner/container was retired after organization-runner replacement, while the repair workflow still requires the retired `oteryn-deploy-runner/runner` Compose service.

Repository terminal target: `REPO_READY_FOR_AUTHORIZED_HOST_VERIFICATION`.

## Acceptance criteria

- [x] Repair workflow resolves the current Platform runner as `oteryn-organization-runners/platform`.
- [x] Legacy `oteryn-deploy-runner/runner` is no longer a required container.
- [x] Workflow trigger follows the current organization Compose reference.
- [x] Existing persistent staging services remain fail-closed: exactly one container each, restart policy `always`, state running/restarting.
- [x] Atlas/Game organization runner services remain outside Platform autostart mutation.
- [x] Runbook and Synology README distinguish retired repository runner from current organization runner.
- [x] Focused static test binds workflow selectors to both Compose contracts.
- [ ] Exact-head repository CI passes once a PR is opened.
- [ ] Authorized real Synology host execution verifies the corrected workflow; this alias has no protected-environment authority and does not claim it.

## Ownership

```yaml
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - tests/ci/test_synology_autostart_contract.py
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - deploy/synology/README.md
  - docs/agents/tasks/active/OTERYN-20261005-synology-autostart-org-runner.md
modules:
  - Synology runner operations
  - CI repair capability
dependencies:
  - Issue #1339
blockers:
  - protected-environment authority is required only for final live-host verification
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T22:00:00+02:00
status: validating
phase: exact_head_ci
branch: fix/1339-synology-autostart-org-runner
head: 36e745deed88224992e9755dc627be52e2762c7b
pr: 1461
context_routes:
  - ci-repair
  - deployment-operations
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - tests/ci/test_synology_autostart_contract.py
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - deploy/synology/README.md
  - docs/agents/tasks/active/OTERYN-20261005-synology-autostart-org-runner.md
proven:
  - Terminal task OTERYN-20260822-retire-legacy-runner-selectors records that repository runner oteryn-synology-staging was deleted and its legacy Synology container removed after replacement PASS.
  - Current organization Compose defines project oteryn-organization-runners and service platform with runner group platform-runners, runner name oteryn-synology-platform, label oteryn-platform and restart always.
  - repair-synology-autostart.yml still requires the retired oteryn-deploy-runner/runner selector.
derived:
  - The repository repair selector is stale and can be corrected without recreating the retired runner.
unknown:
  - Current live Synology container/readback state after the historical terminal proof; protected-environment read/mutation is not authorized by this alias.
conflicts: []
first_failure:
  marker: stale_legacy_runner_selector
  evidence: Issue #1339 run 34202647284 failed before staging-service inspection because oteryn-deploy-runner/runner no longer exists.
rejected_hypotheses:
  - Recreate the retired repository runner merely to satisfy old autostart automation.
  - Modify Atlas or Game organization runner containers from Platform repair automation.
changed_paths:
  - .github/workflows/repair-synology-autostart.yml
  - tests/ci/test_synology_autostart_contract.py
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - deploy/synology/README.md
  - docs/agents/tasks/active/OTERYN-20261005-synology-autostart-org-runner.md
validation:
  - command: exact branch compare against main
    result: PASS
    evidence: commit 0dd62b08d0249e67321eaeaf91899411e02bc79e is one commit ahead of main and changes exactly the five owned paths
  - command: post-write GitHub readback
    result: PASS
    evidence: workflow resolves oteryn-organization-runners/platform, legacy selector is absent, focused contract test and ownership packet are present
  - command: focused Synology autostart contract assertions against exact branch head a670ed2c85385310ac8412cc4ce7c055c42cf205
    result: PASS
    evidence: 20/20 assertions passed for current org-runner selector/group/label, org-compose trigger, restart=always on platform runner and six persistent staging services, exclusion of tls-init/Atlas/Game, and exact-one fail-closed behavior
  - command: first PR #1461 exact-head repository validation on 36e745deed88224992e9755dc627be52e2762c7b
    result: BLOCKED
    evidence: all non-governance repository gates completed SUCCESS; Agent Governance was blocked only by stale LiveOps active ownership inherited from main, resolved separately by merged closeout PR #1464
blockers:
  - final live-host verification requires separately authorized protected-environment execution
next_action: Validate the main-synchronized #1461 exact head. If repository gates pass, classify REPO_READY_FOR_AUTHORIZED_HOST_VERIFICATION; live Synology execution/readback remains separately authorized and unclaimed.
```
