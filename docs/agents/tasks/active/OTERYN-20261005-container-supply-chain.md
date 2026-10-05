---
task_id: OTERYN-20261005-container-supply-chain
governing_issue: 1011
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/operations/SYNOLOGY_STAGING.md
search_first:
  - Synology container supply chain
optional_reads: []
---

# OTERYN-20261005 container supply-chain hardening

## Goal

Implement Issue #1011 for the existing Synology/registry build path without production deployment, registry credential changes, protected-environment mutation or cross-repository writes.

## Acceptance criteria

- [ ] Pin all shipped/staging Dockerfile external base references to reviewed immutable digests while retaining readable tags.
- [ ] Add automated Docker pin update coverage.
- [ ] Generate SPDX JSON SBOM evidence for every built component image.
- [ ] Scan built images for HIGH/CRITICAL patchable vulnerabilities and fail closed under an explicit bounded exception policy.
- [ ] Generate deterministic provenance binding source SHA, Dockerfile/build-policy inputs, SBOM/scan digests and final image digest.
- [ ] Prevent new mutable external base-image references with deterministic repository tests.
- [ ] Preserve current non-root runtime identities.
- [ ] Exact-head CI/build validation and Merge Queue integration pass.

## Ownership

```yaml
owned_paths:
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/gateway.Dockerfile
  - deploy/synology/runner/Dockerfile
  - .github/dependabot.yml
  - .github/workflows/build-synology-staging-images.yml
  - scripts/ci/classify_synology_builds.py
  - scripts/ci/container_supply_chain.py
  - tests/ci/test_container_supply_chain.py
  - docs/security/CONTAINER_SUPPLY_CHAIN_POLICY.json
  - docs/security/CONTAINER_VULNERABILITY_EXCEPTIONS.json
  - deploy/synology/README.md
  - docs/agents/tasks/active/OTERYN-20261005-container-supply-chain.md
modules:
  - Synology staging container build
  - CI supply-chain evidence
dependencies: []
blockers: []
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T19:40:00+02:00
head: 91d70e7075697d11cf3f3e53e19dd2dc2c012438
branch: security/issue-1011-container-supply-chain
pr: none
status: implementing
terminal_pr_policy: active
context_routes:
  - ci
  - security
owned_paths:
  - deploy/synology/docker/platform.Dockerfile
  - .github/dependabot.yml
  - .github/workflows/build-synology-staging-images.yml
  - scripts/ci/classify_synology_builds.py
  - scripts/ci/container_supply_chain.py
  - tests/ci/test_container_supply_chain.py
  - docs/security/CONTAINER_SUPPLY_CHAIN_POLICY.json
  - docs/security/CONTAINER_VULNERABILITY_EXCEPTIONS.json
proven:
  - gateway and deploy-runner external bases are already digest pinned
  - Platform php and composer external references are mutable on protected main
  - current build job publishes immutable image digests and exact source SHA labels
  - current Dependabot has composer npm and github-actions ecosystems but no docker ecosystem
  - no tracked SBOM or Trivy policy exists on protected main
derived:
  - existing build job is the correct enforcement point because staging dispatch already depends on successful image build jobs
unknown:
  - exact live vulnerability result of the newly pinned Platform and Gateway images on this branch
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - a second standalone release workflow is required
  - signing is required to satisfy provenance when deterministic source-to-image evidence is already available
changed_paths:
  - .github/dependabot.yml
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/README.md
  - deploy/synology/docker/platform.Dockerfile
  - docs/agents/tasks/active/OTERYN-20261005-container-supply-chain.md
  - docs/security/CONTAINER_SUPPLY_CHAIN_POLICY.json
  - docs/security/CONTAINER_VULNERABILITY_EXCEPTIONS.json
  - scripts/ci/classify_synology_builds.py
  - scripts/ci/container_supply_chain.py
  - tests/ci/test_container_supply_chain.py
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation not yet committed
blockers: []
next_action: implement immutable base policy, SBOM/scan/provenance evidence and deterministic tests, then open exact-head PR
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository security hardening branch
source_branch_evidence: pending
```
