---
task_id: OTERYN-20260809-download-artifact-immutability
mode: implementation
issue: 948
branch: repair/issue-948
status: ready
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
  - repository environments
  - secrets and variables
  - production systems
  - external repositories
coordination_key: module:downloads-artifact-immutability
```

## Acceptance inventory

- [x] Approved artifact references satisfy HTTPS, exact-host, no-userinfo, no-fragment, standard-port and concrete-reference protections.
- [x] An approved host alone plus a non-root pathname is insufficient for publication.
- [x] The configured host contract requires a machine-testable immutable reference identity.
- [x] Mutable aliases such as `latest`/`current` and ambiguous overwriteable references fail closed unless an explicit trusted host contract proves exact paths immutable.
- [x] Publication revalidates the immutable-reference invariant while release/artifact rows are locked.
- [x] Administrator-supplied SHA-256 remains metadata and is not described as independently verified.
- [x] Focused unit tests cover accepted immutable references and rejected mutable/ambiguous references.
- [x] Feature tests cover draft validation and publication-time revalidation.
- [x] No migration, route, view, workflow, deployment, credential, production or external-repository mutation occurs.
- [x] Exact implementation-head self-review and repository-required CI/heightened validation pass with no unresolved implementation finding.
- [ ] PR merge, task archival and Issue/claim ownership release complete.

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
    result: PASS
    exact_head: 4cd73c5abf69d278b94aab89c03c80a83bf93592
    evidence:
      - PR #966 changed exactly five paths: ArtifactUrlPolicy, downloads config, focused unit/feature tests and this task record; no Issue #948 forbidden path changed.
      - Whole-diff review confirmed non-test runtime remains fail closed without a supported per-host immutable-reference contract.
      - object_version_query accepts exactly one configured query key and rejects missing, wrong, duplicate, extra, empty, malformed, latest and current references.
      - host_path_immutable is an explicit trusted storage contract and rejects all query strings so the exact host plus path is the reference identity.
      - testing compatibility is limited to configured-state construction in APP_ENV=testing; explicit empty contracts passed to the policy still fail closed.
      - APP_ENV=acceptance receives only the synthetic downloads.example.test host_path_immutable contract required by the existing repository acceptance harness.
      - SaveClientReleaseRequest and PublishClientRelease continue to use the strengthened policy, with publication-time revalidation already occurring while release/artifact rows are locked.
      - No upload, proxy, fetch, checksum-verification, signing, deployment or production behavior was introduced.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T22:20:00+02:00
head: 4cd73c5abf69d278b94aab89c03c80a83bf93592
branch: repair/issue-948
pr: 966
status: ready
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
  - Issue #948 is implementation-authorized, priority P1, risk high, rollout_order independent and explicitly forbids deployment, repository-environment, credential, production and external-repository mutation.
  - PR #966 is open, non-draft and mergeable with base main and implementation head 4cd73c5abf69d278b94aab89c03c80a83bf93592.
  - PR #966 changed exactly app/Downloads/Security/ArtifactUrlPolicy.php, config/downloads.php, tests/Unit/Downloads/ArtifactUrlPolicyTest.php, tests/Feature/Downloads/DownloadCenterTest.php and this task record.
  - ArtifactUrlPolicy supports explicit object_version_query and host_path_immutable contracts and fails closed when an allowed runtime host has no supported contract.
  - object_version_query requires exactly one configured parameter and a bounded opaque identifier and rejects latest/current aliases.
  - host_path_immutable rejects every query string, binding the trusted immutable reference to exact host plus concrete path.
  - Existing acceptance fixtures are preserved through an acceptance-only explicit host_path_immutable contract; regular testing compatibility applies only to configured-state construction with an empty contract map.
  - Repository CI run 31427544699 passed on 4cd73c5abf69d278b94aab89c03c80a83bf93592 including Pint, PHPStan and the complete PHPUnit test step.
  - Agent Governance run 31427544905 passed on the same implementation head.
  - Downloads Acceptance run 31427544620 passed on the same implementation head.
  - Content Scale Acceptance run 31427544582 passed on the same implementation head.
  - Acceptance E2E and Visual UX run 31427544670 passed on the same implementation head.
  - Portal Acceptance Contract run 31427545204 passed on the same implementation head.
  - Portal Exhaustive Audit run 31427544502 passed on the same implementation head.
  - Edge Security Emulation run 31427544580 passed on the same implementation head.
  - Phase 7 Production-Like Validation run 31427544804 passed on the same implementation head.
  - Platform DB Outage Validation run 31427544697 passed on the same implementation head.
  - Build Synology Staging Images validated the deployment package and successfully built/validated Platform and game-gateway images; its deploy-runner image job failed before build because docker/setup-buildx received external HTTP 502 while pulling moby/buildkit and that single failed job was rerun.
  - Canonical deploy/synology configuration does not declare or pass DOWNLOADS_ALLOWED_ARTIFACT_HOSTS, so the repository contains no declared active Download Center rollout contract that this bounded repair must migrate.
derived:
  - Because SaveClientReleaseRequest delegates artifact_url validation to ApprovedArtifactUrl and PublishClientRelease revalidates ArtifactUrlPolicy under database locks, strengthening ArtifactUrlPolicy enforces the same immutable-reference invariant at draft validation and publication without widening scope.
  - The Codex rollout comment is an operational enablement concern rather than an Issue #948 implementation defect: #948 explicitly requires runtime fail-closed behavior, declares independent rollout and forbids deployment/environment/production mutation.
  - The earlier fixture review finding is closed by exact-head CI plus Downloads Acceptance, Content Scale Acceptance and full Acceptance E2E passing with the acceptance/testing compatibility contract.
unknown: []
conflicts: []
first_failure:
  marker: mutable-approved-host-artifact-reference
  evidence: Issue #948 proved the prior policy accepted an approved HTTPS host with any non-root pathname even when the external object reference remained overwriteable.
rejected_hypotheses:
  - A pathname that merely looks versioned is sufficient evidence of immutability; Issue #948 explicitly rejects that assumption.
  - Administrator-supplied SHA-256 can substitute for immutable-reference enforcement without fetching bytes; the preserved no-fetch boundary means Platform cannot independently bind that metadata to external object contents.
  - Existing acceptance/browser fixtures require forbidden acceptance-script edits; disproven by passing Downloads Acceptance, Content Scale Acceptance and Acceptance E2E under the bounded acceptance host contract.
  - The current Synology deployment declares an existing DOWNLOADS_ALLOWED_ARTIFACT_HOSTS rollout that must be migrated in this PR; disproven by direct inspection of deploy/synology/.env.example and deploy/synology/compose.yml.
changed_paths:
  - app/Downloads/Security/ArtifactUrlPolicy.php
  - config/downloads.php
  - tests/Unit/Downloads/ArtifactUrlPolicyTest.php
  - tests/Feature/Downloads/DownloadCenterTest.php
  - docs/agents/tasks/active/OTERYN-20260809-download-artifact-immutability.md
validation:
  - command: whole PR changed-file and diff review at implementation head 4cd73c5abf69d278b94aab89c03c80a83bf93592
    result: PASS
    evidence: exactly five bounded paths; no forbidden Issue #948 mutation and no unresolved implementation defect found.
  - command: CI run 31427544699
    result: PASS
    evidence: classification/checkpoint validation, Composer metadata/audit, Pint, PHPStan, PHPUnit and required test gate passed.
  - command: Downloads Acceptance run 31427544620
    result: PASS
    evidence: real HTTP/browser lifecycle and portability acceptance passed with the immutable-reference repair.
  - command: Content Scale Acceptance run 31427544582
    result: PASS
    evidence: acceptance content-scale fixtures remain compatible with the explicit synthetic-host trust contract.
  - command: Acceptance E2E and Visual UX run 31427544670
    result: PASS
    evidence: full browser acceptance completed successfully.
  - command: Portal Acceptance Contract plus Portal Exhaustive Audit plus Edge Security Emulation plus Phase 7 Production-Like Validation
    result: PASS
    evidence: runs 31427545204, 31427544502, 31427544580 and 31427544804 all completed successfully on the implementation head.
  - command: Build Synology Staging Images run 31427544965
    result: BLOCKED
    evidence: deploy package validation and Platform/game-gateway image builds passed; deploy-runner setup-buildx alone hit external HTTP 502 pulling moby/buildkit and was rerun as infrastructure recovery.
blockers:
  - none
next_action: Resolve the four addressed PR #966 review threads, verify the task-only checkpoint commit passes governance/CI and the infrastructure-only build rerun has no task-code failure, then squash-merge PR #966 and archive this task with Issue #948/claim ownership released.
```
