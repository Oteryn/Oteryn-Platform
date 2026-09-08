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

Governing GitHub Issue: #1358 records the bounded repair of the Synology Platform fast/reconcile HTTP smoke namespace boundary.

The repair makes `platform-fast` and `platform-reconcile` probe the exact Platform container namespace instead of the containerized Actions runner loopback, while preserving fail-closed image/binding checks, full reconcile health verification, rollback authority and protected Merge Queue integration.

## Acceptance criteria

- [x] `platform-fast` and `platform-reconcile` never probe Platform through runner-local host loopback.
- [x] Both profiles share one bounded Platform-container HTTP status primitive.
- [x] `/health` and public `/login` smoke remain HTTP-status verified with production forwarding headers.
- [x] Recovery success markers require restored Platform HTTP health, not merely Docker `State.Running`.
- [x] Focused Synology contracts and repository-required exact-head checks pass.
- [x] Merge occurs through normal Merge Queue and resulting protected-main control-plane staging is healthy with rollback skipped.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/archive/OTERYN-20260908-synology-runner-namespace-health.md
modules:
  - Synology staging deployment control plane
dependencies:
  - Issue 1345 remains separate issue-level ownership for the first successful genuine live platform-reconcile timing proof
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T21:21:24Z
head: 3594f5ec9b3262326ba4875a68718ada3f19e666
branch: fix/1358-runner-namespace-health
pr: 1359
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - ci-repair
  - testing
  - execution-resources
owned_paths:
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/archive/OTERYN-20260908-synology-runner-namespace-health.md
proven:
  - The first genuine protected-main platform-reconcile attempt was Deploy Synology Staging run 34241277797 on main 7c8792d64e6baff599ab6a35595a521f212ee1e6.
  - Run 34241277797 selected profile platform-reconcile and then repeatedly failed to connect to 127.0.0.1:8000 from the Platform Actions runner before emitting Platform reconcile /health did not become ready.
  - The same failed run restored the previously proven Platform runtime; follow-up public probes returned HTTP 200 and read-only Synology Diagnostics run 34243765900 completed success.
  - Synology Diagnostics proved the Platform Actions runner is itself a Docker container with host Docker-socket access, while the Platform service publishes 127.0.0.1:8000 on the Synology host namespace.
  - deploy/synology/runner/compose.organization.example.yml does not place the Platform runner in host network mode.
  - The retained full health-check probes Platform inside the exact target container namespace rather than through runner-local loopback.
  - The repair adds one shared Platform-container HTTP status helper in deploy/synology/scripts/lib.sh and routes both fast and reconcile /health plus /login checks through that helper.
  - Recovery verification now requires restored Platform /health to return HTTP 200 before claiming the previous runtime was restored.
  - Local validation passed git diff check, the full Synology deployment Bash/Sh syntax set, checkpoint contract v1 and 127 focused Synology contracts: rollback 29, rollback recovery 13, release identity 37, auto staging 20, gateway inputs 7, deployment routing 5 and fresh baseline 16.
  - PR 1359 final exact head 078b8a2680f13331338501b0d9d99ae997ac83f3 completed all triggered required/supporting workflows successfully, including CI run 34246489692 with runtime-tests success and platform-gate job 102131008253 success.
  - The same exact head passed Phase 7 run 34246489689, Build Synology run 34246489666, Synology Rollback Contract 34246489724, Agent Governance 34246489674, CodeQL 34246489669, Edge Security 34246489703, Platform DB Outage 34246489675 and Game Auth Ticket Concurrency 34246489707.
  - Merge Queue candidate 3594f5ec9b3262326ba4875a68718ada3f19e666 passed CI run 34247066447 with platform-gate job 102132488394 success.
  - PR 1359 merged only through Merge Queue and protected main became exactly 3594f5ec9b3262326ba4875a68718ada3f19e666.
  - The original source branch fix/1358-runner-namespace-health was automatically deleted after merge; branch search returned no retained source ref.
  - Resulting-main CI run 34247355928 completed success on main 3594f5ec9b3262326ba4875a68718ada3f19e666.
  - Resulting-main Build Synology Staging Images run 34247355921 completed success; deployment-package validation passed and no new runtime image build was needed for this control-plane-only repair.
  - Resulting-main Deploy Synology Staging run 34247444487 job 102132962151 completed success on main 3594f5ec9b3262326ba4875a68718ada3f19e666.
  - Run 34247444487 selected the conservative full profile for the accumulated release range, verified Gateway to Platform internal TLS, brought World Registry route 1 online, and emitted Oteryn Synology staging deployment is healthy.
  - In run 34247444487 Deploy prebuilt images ran from 2026-09-08T15:52:00Z to 2026-09-08T16:00:17Z and the rollback step was skipped.
  - PR 1359 initially auto-closed Issue 1358 before runtime closeout; Issue 1358 was reopened so active ownership remained valid until this archive transition.
  - The initial resulting-main Agent Governance failure was lifecycle-only: the active packet still represented terminal PR 1359 without archive_pending state; no product or deployment implementation failure remained.
derived:
  - The failed 127.0.0.1 probe was a network-namespace bug in the fast-path smoke harness, not evidence that the Platform application or immutable image failed to boot.
  - The namespace-safe helper removes the false-negative runner-loopback dependency without weakening the host binding contract or full staging health contract.
  - Because the repair itself changed deployment-control paths, its resulting-main release correctly used the conservative full path; it is not a replacement for Issue 1345's future successful genuine platform-reconcile timing proof.
unknown:
  - Live successful platform-reconcile wall-time remains unknown until a later genuine qualifying product release exercises the repaired fast path.
conflicts: []
first_failure:
  marker: Deploy Synology Staging run 34241277797 Platform reconcile /health did not become ready
  evidence: profile platform-reconcile was selected, runner-local curl to 127.0.0.1:8000 failed repeatedly, then fail-closed recovery restored the prior Platform runtime
rejected_hypotheses:
  - The earlier WebKit portability failure caused staging failure; exact-head acceptance rerun passed before PR 1354 entered Merge Queue.
  - A changed PHP or Composer Docker base caused the Platform failure; the last healthy and failed Platform builds resolved the same base digests.
  - Double Platform recreation alone caused the failure; earlier healthy full deploys used the same Marketplace recreation pattern.
  - Extending the runner-local curl timeout would repair the issue; the address targeted the wrong network namespace regardless of timeout.
changed_paths:
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/tests/test_fresh_baseline_contract.py
  - docs/agents/tasks/archive/OTERYN-20260908-synology-runner-namespace-health.md
validation:
  - command: local focused Synology deployment validation
    result: PASS
    evidence: Bash/Sh syntax plus 127 focused Synology contracts and checkpoint v1 validation passed before publication
  - command: PR 1359 exact-head GitHub Actions on 078b8a2680f13331338501b0d9d99ae997ac83f3
    result: PASS
    evidence: CI run 34246489692 completed success with platform-gate job 102131008253; all triggered supporting workflows were green
  - command: Merge Queue CI on candidate 3594f5ec9b3262326ba4875a68718ada3f19e666
    result: PASS
    evidence: run 34247066447 completed success with platform-gate job 102132488394
  - command: resulting-main CI
    result: PASS
    evidence: run 34247355928 completed success on protected main 3594f5ec9b3262326ba4875a68718ada3f19e666
  - command: resulting-main Build Synology Staging Images
    result: PASS
    evidence: run 34247355921 completed success with deployment-package validation and safe provenance reuse
  - command: resulting-main Deploy Synology Staging
    result: PASS
    evidence: run 34247444487 job 102132962151 completed success, final healthy marker emitted and rollback skipped
blockers: []
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: implementation PR 1359 merged through protected Merge Queue and the source branch has no retention purpose
source_branch_evidence: branch search after merge returned no fix/1358-runner-namespace-health ref; protected main is 3594f5ec9b3262326ba4875a68718ada3f19e666
```

## Remaining issue-level observation

Issue #1345 intentionally remains open for a later genuine qualifying protected-main product release that successfully exercises the repaired `platform-reconcile` path and provides live timing evidence. Do not reactivate this completed repair task and do not manufacture a no-op product commit for that measurement.

## Notes

No production deployment, protected-main bypass, force-push, secret mutation, ruleset weakening or Merge Queue bypass occurred.