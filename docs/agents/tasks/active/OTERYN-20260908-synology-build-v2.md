---
task_id: OTERYN-20260908-synology-build-v2
governing_issue: 1328
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - Platform Issue #1328
  - Platform Issue #1313
  - Platform PR #1331
  - Platform PR #1335
  - Platform PR #1336
  - Platform PR #1337
optional_reads: []
---

# OTERYN-20260908-synology-build-v2

## Goal

Governing GitHub Issue: #1328 — make Synology staging image builds component-aware and proportional while preserving protected-main identity, immutable per-component provenance, rollback/recovery safety and guarded automatic staging deployment.

## Acceptance criteria

- [ ] Pull requests build only images whose real build inputs changed; control-plane changes fail closed.
- [ ] Protected-main changes rebuild only changed runtime components and reuse unchanged components only by proven source SHA + immutable digest.
- [ ] Overall release identity remains separate from Platform/Gateway component identity.
- [ ] Candidate/current/last-good/rollback/recovery state validates explicit per-component provenance.
- [ ] Platform, Gateway and deploy-runner Docker contexts are bounded to truthful inputs.
- [ ] Deployment-package inputs are separated from runtime-image inputs and docs-only changes do not start the Synology image/deploy workflow.
- [ ] Existing migration/schema/rollback safety remains fail closed.
- [ ] Exact-head CI including `platform-gate` and Merge Queue passes.
- [ ] Deployment-package-only and one-runtime-component protected-main behavior is proven live on Synology staging.
- [ ] Performance evidence demonstrates proportional image-job reduction.

## Ownership

```yaml
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/compose.yml
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/platform.Dockerfile.dockerignore
  - deploy/synology/docker/gateway.Dockerfile
  - deploy/synology/docker/gateway.Dockerfile.dockerignore
  - deploy/synology/runner/Dockerfile.dockerignore
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - deploy/synology/scripts/upgrade-legacy-release-state.sh
  - scripts/ci/classify_synology_builds.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tests/ci/test_synology_auto_staging_deploy.py
  - tests/ci/test_synology_gateway_build_inputs.py
  - tests/ci/test_synology_deployment_package_routing.py
  - tests/ci/test_synology_rollback_contract.py
  - tests/ci/test_synology_rollback_recovery_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
modules:
  - Synology staging image selection, deployment package, component provenance, release state and rollback/recovery validation
dependencies:
  - Platform Issue #1313 completed; Deploy Synology Staging run 34167821638 succeeded
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T07:59:00Z
head: 6f244bec5bf26c020b08d7975a3f3393795f582a
branch: ci/1328-synology-deploy-package-proof
pr: 1337
status: validating
context_routes:
  - testing
  - execution-resources
owned_paths:
  - deploy/synology/compose.yml
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
proven:
  - PR 1331 integrated through Merge Queue as protected main 48565e8c3a31c1d0349a89e7c7b38118fd4cebba with component-aware build/provenance architecture
  - PR 1335 repaired protected-main provenance fan-in and bounded Gateway inputs; protected-main Build Synology run 34197410049 and Deploy Synology run 34197488519 both succeeded at cbe2d4ee618019f1884c3f076133469386198f95
  - Gateway BuildKit context was reduced from 136.13 kB before truthful bounding to 12.53 kB after PR 1335
  - PR 1336 removed broad deploy/synology/** routing, separated runtime/operator/validation-only inputs, passed exact-head and Merge Queue qualification, and integrated as protected main 1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19
  - protected-main Build Synology Staging Images run 34201292818 at 1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19 passed classifier, deployment validation, Platform/Gateway publication and exact-provenance staging dispatch; ordinary main did not publish deploy-runner
  - Agent Governance run 34201292867 failed only because this task still referenced terminal PR 1336 with a stale merge next action; checkpoint schema and governing Issue liveness passed
  - deploy/synology/compose.yml used mutable alpine:3.22 specifically for tls-init even though the exact Alpine digest sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce was already independently resolved in prior successful Gateway BuildKit provenance
  - PR 1337 pins tls-init to alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce and adds a deterministic contract requiring that immutable identity
  - PR 1337 changes only the staging deployment package, its contract test, and active task ownership; no Platform, Gateway or deploy-runner image input is modified
derived:
  - PR 1337 is a truthful runtime deployment-package-only change: deploy/synology/compose.yml is release-relevant, while no Platform, Gateway or deploy-runner image input changes
  - after protected integration, PR 1337 should allocate zero runtime image build jobs and resolve both runtime components from accepted persisted immutable current-release provenance before staging reconciliation
  - active ownership now references open PR 1337 rather than terminal PR 1336, removing the prior liveness conflict subject to fresh exact-head governance validation
unknown:
  - terminal result of Deploy Synology Staging run 34201364919 for protected main 1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19
  - exact-head CI and routing result for PR 1337
  - live protected-main Case A zero-image-build and component-reuse result
  - representative one-runtime-component one-image-build live proof with immutable reuse
conflicts: []
first_failure:
  marker: post-1336-terminal-task-liveness
  evidence: Agent Governance run 34201292867 reported terminal_pr_stale_next_action and terminal_pr_active_task for merged PR 1336; all checkpoint and governing-Issue validators passed
rejected_hypotheses:
  - PR 1336 technical routing failed on protected main
  - protected-main deploy-runner publication occurred after PR 1336
  - health-check alpine references require a new pin; lib.sh already substitutes their historical tags with immutable probe digests
changed_paths:
  - deploy/synology/compose.yml
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
validation:
  - command: Build Synology Staging Images run 34201292818 at protected main 1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19
    result: PASS
    evidence: deployment validation, classifier, Platform/Gateway publish and exact-provenance staging dispatch succeeded; no deploy-runner main publication job existed
  - command: Agent Governance run 34201292867
    result: FAIL
    evidence: only live task-liveness failed because merged PR 1336 remained recorded as active ownership; schema and Issue liveness passed
blockers: []
next_action: Qualify PR 1337 exact-head as a deployment-package-only change with zero image build jobs, integrate through protected policy, and verify automatic Synology staging reconciliation with both runtime components reused by immutable provenance.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task ownership is bound to open PR 1337 and remains active until both live proportionality cases and terminal closeout are proven
source_branch_evidence: PR #1337 continues Issue #1328 after merged PR #1336
```

## Notes

Production deployment remains excluded. PRs never deploy Synology staging. Ordinary protected-main pushes never publish the privileged deploy-runner. Component reuse remains immutable and provenance-validated.
