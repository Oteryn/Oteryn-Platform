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
  - docs/agents/tasks/archive/OTERYN-20261005-container-supply-chain.md
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
updated_at: 2026-10-05T19:45:00+02:00
head: 206f67d034d44e2c2ead62de08aca5aa9e541fb0
branch: security/issue-1011-container-supply-chain
pr: 1456
status: completed
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
  - docs/agents/tasks/archive/OTERYN-20261005-container-supply-chain.md
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
next_action: Follow-up SUPPLY-CHAIN-POLICY-2 stays open (see Follow-up); record archived.
```

## Follow-up

- SUPPLY-CHAIN-POLICY-2 (carry-forward, not merge-blocking): (a) persist the blocked policy decision (`--json-output`) before `evaluate-trivy` exits nonzero so rejected builds retain a structured decision record; (b) make the evaluator the sole vulnerability filter by scanning the full relevant result set, or deriving the Trivy action severity/unfixed inputs from the validated policy, so tightening the JSON policy cannot be bypassed by the scan step. Review threads 4239863117 and 4239863119.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: merged feature branch has no retention or recovery purpose after protected merge
source_branch_evidence: PR #1456 merged (REST pulls/1456 state closed, merged true) as 8d80c94fc2640560d0c68aae70a75f7a5a2ddd0c at 2026-10-11T01:41:50Z from exact head 206f67d034d44e2c2ead62de08aca5aa9e541fb0; source branch (security/issue-1011-container-supply-chain) absent (REST branches API returns 404)
```
