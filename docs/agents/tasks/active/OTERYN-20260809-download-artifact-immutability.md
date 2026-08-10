---
task_id: OTERYN-20260809-download-artifact-immutability
mode: implementation
issue: 948
branch: repair/issue-948
status: validating
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
updated_at: 2026-08-10T21:46:10+02:00
head: 9fb3df66933b379bf8262ed055532fbd0ed8915d
branch: repair/issue-948
pr: 966
status: validating
context_routes:
  - agent-governance
  - security
  - public-web
  - testing
owned_paths:
  - app/Downloads/Security/ArtifactUrlPolicy.php
  - app/Downloads/Rules/ApprovedArtifactUrl.php
  - app/Downloads/Actions/PublishClientRelease.php
  - app/Http/Requests/Downloads/SaveClientReleaseRequest.php
  - config/downloads.php
  - tests/Unit/Downloads/ArtifactUrlPolicyTest.php
  - tests/Feature/Downloads/DownloadCenterTest.php
  - docs/agents/tasks/active/OTERYN-20260809-download-artifact-immutability.md
proven:
  - Issue #948 is open, implementation-authorized, priority P1 and risk high.
  - The deterministic branch repair/issue-948 was created from protected main 2e82d81bbace65a85b69fbbdadf85cdc44034b61 after absence of a competing branch or PR was verified.
  - Existing active tasks own public-edge verification and native-auth verification paths and do not overlap this Issue's exclusive repair paths.
  - ArtifactUrlPolicy now requires the approved host to have an explicit object_version_query contract and requires exactly one configured version query parameter with a bounded opaque identifier.
  - A version-looking path, latest or current path, missing contract, missing version reference, wrong query parameter, duplicate query parameter or extra query parameter fails closed.
  - Config invalid or missing immutable-reference metadata resolves to no contract and therefore fails closed.
  - PR #966 is the single authoritative delivery PR for this Issue.
derived:
  - Because SaveClientReleaseRequest already delegates artifact_url validation to ApprovedArtifactUrl and PublishClientRelease already revalidates ArtifactUrlPolicy while holding release and artifact locks, strengthening ArtifactUrlPolicy applies the immutable-reference invariant at both draft validation and publication without widening the repair scope.
unknown: []
conflicts: []
first_failure:
  marker: mutable-approved-host-artifact-reference
  evidence: Issue #948 proves the prior policy accepted an approved HTTPS host with any non-root pathname even when the external object reference remained overwriteable.
rejected_hypotheses:
  - A pathname that merely looks versioned is sufficient evidence of immutability; Issue #948 and the canonical public-site architecture explicitly reject that assumption.
  - Administrator-supplied SHA-256 can substitute for immutable-reference enforcement without fetching the bytes; the existing no-fetch boundary means Platform cannot independently bind that metadata to external object contents.
changed_paths:
  - app/Downloads/Security/ArtifactUrlPolicy.php
  - config/downloads.php
  - tests/Unit/Downloads/ArtifactUrlPolicyTest.php
  - tests/Feature/Downloads/DownloadCenterTest.php
  - docs/agents/tasks/active/OTERYN-20260809-download-artifact-immutability.md
validation:
  - command: exact branch source and PR diff inspection against Issue #948 acceptance
    result: PASS
    evidence: PR #966 changes only the bounded policy, configuration, focused tests and this task packet; no forbidden runtime or deployment path is modified.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: This is a backend-only invariant repair with no route or view change; the real save, publish and public-read integration path is exercised by the DownloadCenter feature tests.
  - command: CI and Agent Governance on 9fb3df66933b379bf8262ed055532fbd0ed8915d
    result: FAIL
    evidence: Both runs stopped on this task checkpoint schema because context_routes, derived, first_failure, head, owned_paths and rejected_hypotheses were absent and checkpoint status active was unsupported; this commit corrects exactly those reported fields.
blockers: []
next_action: Validate the corrected exact PR head, inspect any remaining CI, Downloads Acceptance, review or mergeability failures, repair them on PR #966 if needed, then merge and release Issue and task ownership.
```
