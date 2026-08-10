---
task_id: OTERYN-20260809-download-artifact-immutability
mode: implementation
issue: 948
branch: repair/issue-948
status: active
programme: OTERYN_PLATFORM_REMEDIATION
portal_programme: OTERYN_PORTAL_COMPLETION
---

# OTERYN-20260809-download-artifact-immutability

## Goal

Repair Issue #948 by enforcing a machine-testable immutable artifact-reference contract for Download Center releases while preserving the existing no-upload, no-proxy, no-fetch boundary and truthful administrator-supplied SHA-256 semantics.

## Feature scope

```yaml
feature_scope:
  type: backend_only
  user_facing: false
  backend_required: true
  frontend_required: false
  integration_required: true
  e2e_required: false
  completion_claim: internal_only
```

## Ownership

```yaml
project_lane: oteryn-platform-core
owned_paths:
  - app/Downloads/Security/ArtifactUrlPolicy.php
  - app/Downloads/Rules/ApprovedArtifactUrl.php
  - app/Downloads/Actions/PublishClientRelease.php
  - app/Http/Requests/Downloads/SaveClientReleaseRequest.php
  - config/downloads.php
  - tests/Unit/Downloads/ArtifactUrlPolicyTest.php
  - tests/Feature/Downloads/DownloadCenterTest.php
  - docs/agents/tasks/active/OTERYN-20260809-download-artifact-immutability.md
restricted_paths:
  - database/**
  - routes/**
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - .github/workflows/**
  - deploy/**
  - external repositories
  - production systems
coordination_key: module:downloads-artifact-immutability
```

## Acceptance inventory

- [x] Approved artifact references satisfy HTTPS, exact-host, no-userinfo, no-fragment, standard-port and concrete-reference protections.
- [x] An approved host alone plus a non-root pathname is insufficient for publication.
- [x] The configured host contract requires a machine-testable immutable reference identity.
- [x] Mutable aliases such as `latest`/`current` and ambiguous overwriteable references fail closed.
- [x] Publication revalidates the immutable-reference invariant while release/artifact rows are locked.
- [x] Administrator-supplied SHA-256 remains metadata and is not described as independently verified.
- [x] Focused unit tests cover accepted immutable references and rejected mutable/ambiguous references.
- [x] Feature tests cover draft validation and publication-time revalidation.
- [x] No migration, route, view, workflow, deployment, credential, production or external-repository mutation occurs.
- [ ] Exact-head self-review, applicable repository CI and closeout pass with no unresolved material finding.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: chatgpt-portal-closeout-20260810-2125
  classified_at: 2026-08-10T21:25:00+02:00
  risk: high
  triggers:
    - executable client distribution
    - software supply-chain integrity
    - administrator-controlled external artifact reference
    - mutable object replacement after publication
  unknown_or_conflict: []
  rationale: Download Center publishes executable client references; the repair changes the trust invariant used at save and publish time.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T21:43:00+02:00
base_sha: 2e82d81bbace65a85b69fbbdadf85cdc44034b61
implementation_head: a1b9352703f4eac5b75b7a776135ebaab5f429a4
branch: repair/issue-948
pr: 966
status: active
phase: validation
owner: chatgpt-portal-closeout-20260810-2125
proven:
  - Issue #948 is open, implementation-authorized, priority P1 and risk high.
  - The deterministic branch repair/issue-948 was created from protected main 2e82d81bbace65a85b69fbbdadf85cdc44034b61 after absence of a competing branch/PR was verified.
  - Existing active tasks own public-edge verification and native-auth verification paths and do not overlap this Issue's exclusive repair paths.
  - ArtifactUrlPolicy now requires the approved host to have an explicit object_version_query contract and requires exactly one configured version query parameter with a bounded opaque identifier.
  - A version-looking path, latest/current path, missing contract, missing version reference, wrong query parameter, duplicate query parameter or extra query parameter fails closed.
  - SaveClientReleaseRequest already applies ApprovedArtifactUrl and PublishClientRelease already revalidates ArtifactUrlPolicy while holding locked release/artifact rows; the stronger invariant therefore applies at draft validation and publication without widening scope.
  - Config invalid/missing immutable-reference metadata resolves to no contract and therefore fails closed.
  - PR #966 is the single authoritative delivery PR for this Issue.
unknown: []
conflicts: []
changed_paths:
  - app/Downloads/Security/ArtifactUrlPolicy.php
  - config/downloads.php
  - tests/Unit/Downloads/ArtifactUrlPolicyTest.php
  - tests/Feature/Downloads/DownloadCenterTest.php
  - docs/agents/tasks/active/OTERYN-20260809-download-artifact-immutability.md
validation:
  - command: exact branch/source inspection against Issue #948 acceptance
    result: PASS
    evidence: implementation head a1b9352703f4eac5b75b7a776135ebaab5f429a4 changes only the bounded policy/config/tests plus this task packet.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: backend-only invariant repair with no route/view/browser change; real save/publish/public-read integration paths are covered by DownloadCenter feature tests.
blockers: []
next_action: Run repository-required validation on the exact PR head, inspect the whole diff and any CI/review findings, repair on PR #966 if needed, then merge and complete Issue/task ownership release.
```
