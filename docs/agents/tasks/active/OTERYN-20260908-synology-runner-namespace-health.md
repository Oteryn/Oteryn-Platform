---
task_id: OTERYN-20260908-synology-runner-namespace-health
governing_issue: 1358
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - GitHub Issue #1345 live platform-reconcile benchmark evidence
  - Deploy Synology Staging run 34241277797
  - deploy/synology/scripts/{deploy.sh,deploy-impact.sh,lib.sh,health-check.sh}
optional_reads: []
---

# OTERYN-20260908-synology-runner-namespace-health

## Goal

Governing GitHub Issue: #1358 - canonical lifecycle authority for this repair.

Make the Platform fast/reconcile HTTP smoke namespace-safe on the containerized Synology Actions runner, while preserving fail-closed image/binding checks, retained full reconcile health verification, rollback authority and Merge Queue policy.

## Acceptance criteria

- [x] `platform-fast` and `platform-reconcile` never probe Platform through runner-local host loopback.
- [x] Both profiles share one bounded Platform-container HTTP status primitive.
- [x] `/health` and public `/login` smoke remain HTTP-status verified with the production forwarding headers.
- [x] Recovery success markers require restored Platform HTTP health, not merely `State.Running`.
- [ ] Focused Synology contracts and repository-required exact-head checks pass.
- [ ] Merge occurs through normal Merge Queue and resulting protected-main control-plane staging is healthy with rollback skipped.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-runner-namespace-health.md
modules:
  - Synology staging deployment control plane
dependencies:
  - GitHub Issue #1345 live benchmark remains open until a later genuine qualifying product release succeeds through platform-reconcile
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T15:42:12Z
head: 12e7b98aaf27877a3cbc97d9926bfaa154813f17
branch: fix/1358-runner-namespace-health
pr: 1359
status: validating
context_routes:
  - ci-repair
  - testing
  - execution-resources
owned_paths:
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-runner-namespace-health.md
proven:
  - Deploy Synology Staging run 34241277797 selected platform-reconcile and failed because runner-local curl to 127.0.0.1:8000 could not connect; rollback restored the previous Platform.
  - Synology Diagnostics run 34243765900 proves the Platform Actions runner executes inside a Docker container with host Docker-socket access, while Platform is published on the Synology host loopback.
  - deploy/synology/runner/compose.organization.example.yml gives the Platform runner normal container networking rather than host networking.
  - The retained full health-check probes Platform through the target container namespace instead of runner-local loopback.
  - PR 1359 is the canonical validation and integration PR for Issue 1358; its implementation commit is 12e7b98aaf27877a3cbc97d9926bfaa154813f17.
derived:
  - Fast/reconcile HTTP smoke must enter the exact Platform container namespace rather than address runner-local 127.0.0.1.
unknown:
  - Final exact-head CI and required-check result for PR 1359.
  - Resulting protected-main conservative/full staging result after repair integration.
  - Whether the repaired probe passes the next genuine live fast/reconcile product release on protected main.
conflicts: []
first_failure:
  marker: Deploy Synology Staging run 34241277797 Platform reconcile /health did not become ready
  evidence: repeated curl (7) connection failures to runner-local 127.0.0.1:8000 followed by successful fail-closed rollback
rejected_hypotheses:
  - The WebKit portability flake caused staging failure; exact-head rerun passed before Merge Queue integration.
  - A changed Docker base caused the Platform failure; the last healthy and failed Platform builds resolved identical PHP and Composer base digests.
  - Double Platform recreation alone caused the failure; prior healthy full staging deploys performed the same marketplace recreation pattern.
changed_paths:
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-runner-namespace-health.md
validation:
  - command: git diff --check plus repository Synology bash/sh syntax validation
    result: PASS
    evidence: diff check and the full build-synology deployment-package bash -n/sh -n script set passed locally in WSL
  - command: repository Synology contract suite used by build-synology-staging-images.yml
    result: PASS
    evidence: 127 contracts passed - rollback 29, rollback recovery 13, release identity 37, auto staging 20, gateway inputs 7, deployment routing 5, fresh baseline 16
  - command: python3 tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20260908-synology-runner-namespace-health.md --require-checkpoint
    result: PASS
    evidence: checkpoint contract v1 validated locally
blockers:
  - none
next_action: validate the final exact head of PR 1359 across all required checks before any Merge Queue enqueue
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository repair PR path
source_branch_evidence: pending Merge Queue integration and automatic source-ref deletion
```

## Notes

The resulting-main deployment for this repair must remain conservative/full because deployment-control paths changed. That deployment proves the repair did not regress the retained full path; #1345 still requires a subsequent genuine product change to exercise `platform-reconcile` live.
