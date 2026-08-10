---
task_id: OTERYN-20260809-download-artifact-immutability
mode: implementation
issue: 948
status: completed
programme: OTERYN_PLATFORM_REMEDIATION
portal_programme: OTERYN_PORTAL_COMPLETION
---

# OTERYN-20260809-download-artifact-immutability

## Goal

Repair Issue #948 by enforcing a machine-testable immutable artifact-reference contract for Download Center releases while preserving the no-upload, no-proxy, no-fetch boundary and truthful administrator-supplied SHA-256 semantics.

## Completion

- [x] PR #966 delivered the bounded security repair and was squash-merged to protected `main` as `775261a407807c6efc569ac645c44aa00a2641fe`.
- [x] Issue #948 auto-closed from the merge.
- [x] Exact implementation-head self-review passed with no unresolved material finding.
- [x] Required CI, heightened security validation, Download Center acceptance and broader portal acceptance passed.
- [x] All four Codex review threads were addressed and resolved before merge.
- [x] The repair claim and `module:downloads-artifact-immutability` coordination ownership were released on Issue #948.
- [x] The deterministic repair branch was absent after merge.
- [x] No deployment, environment, credential, production or external-repository mutation occurred.

## Delivered contract

- Runtime artifact hosts require an explicit supported immutable-reference contract in addition to the exact HTTPS host allowlist.
- `object_version_query` requires exactly one configured version query parameter and rejects absent, wrong, duplicate, extra, empty, malformed, `latest` and `current` references.
- `host_path_immutable` is an explicit trusted storage contract and rejects every query string so exact host plus concrete path is the immutable reference identity.
- Missing or invalid runtime contracts fail closed.
- Acceptance/testing compatibility is bounded to synthetic repository test environments and does not weaken explicit runtime fail-closed behavior.
- Publication continues to revalidate the strengthened policy under existing release/artifact locks.
- Administrator-supplied SHA-256 remains metadata; Platform still does not claim independent artifact verification.

## Lifecycle closeout

```yaml
lifecycle_closeout:
  implementation_pr: 966
  implementation_merge_sha: 775261a407807c6efc569ac645c44aa00a2641fe
  implementation_exact_head: 4cd73c5abf69d278b94aab89c03c80a83bf93592
  final_pr_checkpoint_head: 2332062736475140b25b38df6c13bc080d420911
  issue: 948
  issue_state: closed
  claim_release_comment: 5245619667
  repair_branch_present_after_merge: false
  batching:
    applied: false
    reason: The only other active task records are genuine blocked external/production verification records and are not terminal lifecycle items; delaying this completed security task merely to reach lifecycle batch size two would violate the anti-stall rule.
  runtime_or_product_change_in_closeout: false
```

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  result: PASS
  implementation_exact_head: 4cd73c5abf69d278b94aab89c03c80a83bf93592
  evidence:
    - CI run 31427544699 passed Pint, PHPStan, complete PHPUnit and the required test gate.
    - Agent Governance run 31427544905 passed.
    - Downloads Acceptance run 31427544620 passed.
    - Content Scale Acceptance run 31427544582 passed.
    - Acceptance E2E and Visual UX run 31427544670 passed.
    - Portal Acceptance Contract run 31427545204 passed.
    - Portal Exhaustive Audit run 31427544502 passed.
    - Edge Security Emulation run 31427544580 passed.
    - Phase 7 Production-Like Validation run 31427544804 passed.
    - Platform DB Outage Validation run 31427544697 passed.
    - Final task-only checkpoint head 2332062736475140b25b38df6c13bc080d420911 also passed Agent Governance, CI, Downloads Acceptance, Portal Acceptance Contract, Portal Exhaustive Audit, Edge Security Emulation, Phase 7, Content Scale, Platform DB Outage and all four Synology staging image jobs.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T22:32:00+02:00
head: 775261a407807c6efc569ac645c44aa00a2641fe
branch: none
pr: 966
status: completed
context_routes:
  - agent-governance
  - security
  - public-web
  - testing
owned_paths: []
proven:
  - Protected main resolved to implementation merge 775261a407807c6efc569ac645c44aa00a2641fe immediately after PR #966 merged.
  - Issue #948 is closed and its implementation/coordination claim is explicitly released by Issue comment 5245619667.
  - The repair branch repair/issue-948 is no longer present.
  - PR #966 changed exactly ArtifactUrlPolicy, downloads configuration, focused unit/feature tests and the task record; no Issue #948 forbidden path changed.
  - The implementation and final checkpoint generations passed required exact-head CI and heightened validation with zero unresolved review threads before merge.
  - The remaining active public-domain and native-auth task records are blocked on external/protected or future native-runtime authority and are not compatible terminal lifecycle batch items.
derived:
  - Issue #948 has no remaining implementation, review, branch, lease or coordination ownership.
  - Repository archival is the only closeout mutation represented by this record and carries no runtime behavior.
unknown: []
conflicts: []
first_failure:
  marker: mutable-approved-host-artifact-reference
  evidence: resolved by PR #966 through explicit machine-testable immutable-reference contracts.
rejected_hypotheses:
  - A version-looking pathname alone proves immutability.
  - Administrator-supplied SHA-256 can replace immutable-reference enforcement without fetching bytes.
  - A deployment/environment rollout mutation belongs in Issue #948 despite its explicit forbidden-path boundary.
  - Another currently active task is a compatible terminal lifecycle item suitable for batching with this archive.
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20260809-download-artifact-immutability.md
  - docs/agents/tasks/active/OTERYN-20260809-download-artifact-immutability.md
validation:
  - command: implementation PR #966 exact-head validation and merge verification
    result: PASS
    evidence: required CI and heightened validation passed before squash merge 775261a407807c6efc569ac645c44aa00a2641fe.
  - command: post-merge Issue/branch/claim reconciliation
    result: PASS
    evidence: Issue #948 closed, claim release comment 5245619667 persisted and repair/issue-948 is absent.
  - command: lifecycle-only runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: this terminal move changes only task lifecycle metadata and cannot alter executable behavior.
blockers: []
next_action: Re-evaluate the live OTERYN_PORTAL_COMPLETION queue; Issue #944 is the next safe unowned implementation-authorized P1 candidate if it remains READY.
```
