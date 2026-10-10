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

# OTERYN-20261005 Synology autostart organization-runner reconciliation — terminal archive

## Terminal outcome

PR #1461 (`fix/1339-synology-autostart-org-runner`) merged into `main` at 2026-10-10T20:50:49Z. The repository repair is delivered. Authorized real Synology host execution/readback remains separately authorized and is not claimed by this record.

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
- [x] Exact-head repository CI passes (PR #1461 merged through protected checks).
- [ ] Authorized real Synology host execution verifies the corrected workflow; this record has no protected-environment authority and does not claim it. Tracked by Issue #1339.

## Ownership

```yaml
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - tests/ci/test_synology_autostart_contract.py
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - deploy/synology/README.md
  - docs/agents/tasks/archive/OTERYN-20261005-synology-autostart-org-runner.md
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
updated_at: 2026-10-10T21:00:00Z
status: completed
terminal_pr_policy: archive_pending
phase: terminal_archive
branch: fix/1339-synology-autostart-org-runner
head: 0e2cc3a4e1053f10e264eaaf46f5e3052a43773d
pr: 1461
context_routes:
  - ci-repair
  - deployment-operations
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - tests/ci/test_synology_autostart_contract.py
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - deploy/synology/README.md
  - docs/agents/tasks/archive/OTERYN-20261005-synology-autostart-org-runner.md
proven:
  - Terminal task OTERYN-20260822-retire-legacy-runner-selectors records that repository runner oteryn-synology-staging was deleted and its legacy Synology container removed after replacement PASS.
  - Current organization Compose defines project oteryn-organization-runners and service platform with runner group platform-runners, runner name oteryn-synology-platform, label oteryn-platform and restart always.
  - PR #1461 merged into main at 2026-10-10T20:50:49Z from head 0e2cc3a4e1053f10e264eaaf46f5e3052a43773d.
derived:
  - The repository repair selector was stale and is corrected without recreating the retired runner.
unknown:
  - Current live Synology container/readback state; protected-environment read/mutation is not authorized by this record.
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
  - docs/agents/tasks/archive/OTERYN-20261005-synology-autostart-org-runner.md
validation:
  - command: focused Synology autostart contract assertions against exact branch head a670ed2c85385310ac8412cc4ce7c055c42cf205
    result: PASS
    evidence: 20/20 assertions passed for current org-runner selector/group/label, org-compose trigger, restart=always on platform runner and six persistent staging services, exclusion of tls-init/Atlas/Game, and exact-one fail-closed behavior
  - command: first PR #1461 exact-head repository validation on 36e745deed88224992e9755dc627be52e2762c7b
    result: BLOCKED
    evidence: all non-governance repository gates completed SUCCESS; Agent Governance was blocked only by stale LiveOps active ownership inherited from main, resolved separately by merged closeout PR #1464
  - command: PR #1461 protected merge
    result: PASS
    evidence: PR #1461 state closed, merged true, merged_at 2026-10-10T20:50:49Z
blockers:
  - final live-host verification requires separately authorized protected-environment execution (tracked by Issue #1339)
next_action: Archive this terminal task record; PR #1461 is already merged. Live Synology execution/readback remains separately authorized under Issue #1339 and is unclaimed.
```
