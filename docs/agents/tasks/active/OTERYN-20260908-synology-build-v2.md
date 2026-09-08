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
updated_at: 2026-09-08T08:12:00Z
head: 45c251d93937d6d6e0335f22fc9a4c3838dd1b8a
branch: ci/1328-synology-gateway-proof
pr: none
status: implementing
context_routes:
  - testing
  - execution-resources
owned_paths:
  - deploy/synology/docker/gateway.Dockerfile
  - tests/ci/test_synology_gateway_build_inputs.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
proven:
  - PR 1331 integrated through Merge Queue as protected main 48565e8c3a31c1d0349a89e7c7b38118fd4cebba with component-aware build/provenance architecture
  - PR 1335 repaired protected-main provenance fan-in and bounded Gateway inputs; protected-main Build Synology run 34197410049 and Deploy Synology run 34197488519 both succeeded at cbe2d4ee618019f1884c3f076133469386198f95
  - Gateway BuildKit context was reduced from 136.13 kB before truthful bounding to 12.53 kB after PR 1335
  - PR 1336 removed broad deploy/synology/** routing, separated runtime/operator/validation-only inputs, passed exact-head and Merge Queue qualification, and integrated as protected main 1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19
  - protected-main Build Synology Staging Images run 34201292818 at 1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19 passed classifier, deployment validation, Platform/Gateway publication and exact-provenance staging dispatch; ordinary main did not publish deploy-runner
  - automatic Deploy Synology Staging run 34201364919 succeeded for protected main 1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19 with rollback skipped
  - PR 1337 passed exact-head and Merge Queue qualification and integrated as protected main 0cd7e76c45736e81cc50db24c6af79ba4a819631
  - protected-main Build Synology Staging Images run 34202647423 at 0cd7e76c45736e81cc50db24c6af79ba4a819631 passed deployment validation and classifier, allocated zero image build jobs, and dispatched staging with BUILD_PLATFORM=false and BUILD_GATEWAY=false
  - protected-main Case A dispatch explicitly reused exact proven Platform and Gateway components from Synology current-release state; no current-run component provenance artifact was downloaded because neither component rebuilt
  - Case A changed only a real deployment-package input by pinning tls-init to alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce plus its contract and task ownership
  - protected-main Gateway build run 34201292818 resolved golang:1.24-alpine to sha256:8bee1901f1e530bfb4a7850aa7a479d17ae3a18beb6e09064ed54cfd245b7191 and alpine:3.22 to sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce; BuildKit provenance recorded both materials
  - Case B branch pins those already observed Gateway base-image digests and adds an exact deterministic contract; no Platform or deploy-runner image input changes
  - Repair Synology Autostart run 34202647284 failed before staging service checks because no oteryn-deploy-runner/runner container existed; no causal link to the Case A tls-init digest is proven
derived:
  - Case A demonstrates image-job allocation reduction from the former fixed three-entry matrix to zero for a truthful deployment-package-only protected-main release
  - Case B is a truthful Gateway-only image change and should allocate exactly one Gateway image build while Platform is resolved from persisted immutable provenance
  - the autostart failure is a separate infrastructure observation unless older workflow evidence proves otherwise; it must not be attributed to Case A without causal evidence
unknown:
  - terminal result and exact reused component identities from Deploy Synology Staging run 34202722679 for protected main 0cd7e76c45736e81cc50db24c6af79ba4a819631
  - exact-head Case B PR routing and CI result
  - protected-main Case B one-image-build and Platform-reuse result
  - whether the missing oteryn-deploy-runner/runner container predates Case A; recent run history inspected so far does not establish an earlier comparable autostart result
conflicts: []
first_failure:
  marker: repair-synology-autostart-missing-runner-container
  evidence: run 34202647284 failed with Expected exactly one oteryn-deploy-runner/runner container, found 0 before iterating the oteryn-staging services
rejected_hypotheses:
  - Case A rebuilt Platform, Gateway or deploy-runner; protected-main build job was skipped with BUILD_PLATFORM=false and BUILD_GATEWAY=false
  - Case A staging dispatch invented new runtime component identities; dispatch sent empty current-run component inputs and selected persisted-provenance reuse
  - Case B base-image digests were guessed from mutable Docker tags; both exact digests are recorded in successful protected-main BuildKit logs and provenance
  - Repair Synology Autostart failure proves the tls-init digest is invalid; the failure occurred at missing deploy-runner container discovery before staging services were inspected
changed_paths:
  - deploy/synology/docker/gateway.Dockerfile
  - tests/ci/test_synology_gateway_build_inputs.py
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
validation:
  - command: Build Synology Staging Images run 34202647423 at protected main 0cd7e76c45736e81cc50db24c6af79ba4a819631
    result: PASS
    evidence: deployment validation and classifier succeeded, runtime image build matrix was skipped, and exact-provenance staging dispatch succeeded with both changed flags false
  - command: Deploy Synology Staging run 34202722679
    result: NOT_RUN
    evidence: live run is still in progress after immutable provenance resolution and is executing Deploy prebuilt images
  - command: Repair Synology Autostart run 34202647284
    result: FAIL
    evidence: workflow found zero oteryn-deploy-runner/runner containers before checking staging services; relationship to Issue 1328 is not proven
blockers: []
next_action: Open the Case B Gateway-only PR from ci/1328-synology-gateway-proof, prove exactly one Gateway image build with no Platform or deploy-runner build, then integrate through protected policy and verify immutable Platform reuse on Synology staging.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task ownership is bound to the live Case B branch and remains active until one-component protected-main behavior and terminal closeout are proven
source_branch_evidence: ci/1328-synology-gateway-proof continues Issue #1328 after merged PR #1337
```

## Notes

Production deployment remains excluded. PRs never deploy Synology staging. Ordinary protected-main pushes never publish the privileged deploy-runner. Component reuse remains immutable and provenance-validated.
