---
task_id: OTERYN-20260815-ci-workflow-orchestration
policy_version: 2
project_lane: oteryn-platform-core
task_kind: implementation
execution_mode: github-only
implementation_authorized: true
parent_issue: 1085
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
search_first:
  - open PRs and active tasks touching CI/workflow paths
optional_reads: []
---

# OTERYN-20260815-ci-workflow-orchestration

## Goal

Make Oteryn Platform CI risk/path-aware and lifecycle-managed so complete validation is preserved while unrelated heavy workflows, historical one-off orchestration and unmeasured test execution no longer accumulate or delay normal delivery.

## Acceptance criteria

- [x] Documentation/governance-only main pushes no longer admit Portal Acceptance runtime/account-lifecycle jobs.
- [x] Ordinary workflow edits no longer fan out to every heavy runtime gate; central routing changes remain fail-closed.
- [x] Repository contracts reject unbounded workflow admission, retired-workflow coupling, missing PR cancellation and unstable/non-isolated PR concurrency identity.
- [x] Six proven obsolete diagnostic/task-wrapper workflows are retired without deleting their current proving layers.
- [x] Completed deep/exhaustive programme orchestration no longer runs automatically on ordinary product PRs/main pushes.
- [x] Editorial Media and Wiki acceptance no longer inherit broad trigger paths solely from retired exhaustive orchestration.
- [x] Mixed validation/operation workflows cancel only superseded PR validation while preserving trusted/manual/scheduled operation serialization.
- [x] PHP application coverage is measured on relevant main pushes outside the blocking PR path with a ratchet-ready policy and bounded artifact.
- [x] Workflow lifecycle rules make new/task-specific workflow retention explicit and machine-enforced.
- [x] Legacy public-host reuse guard permits the focused architecture reference only while the exact fail-closed retirement invariant remains present.
- [ ] Final exact-head CI, ready-state checks, merge, Issue closure, archival and source-branch closeout remain.

## Ownership

```yaml
owned_paths:
  - .github/workflows/ci.yml
  - .github/workflows/portal-acceptance-contract.yml
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - .github/workflows/cloudflare-oteryn-endpoint-main-operation.yml
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - .github/workflows/cloudflare-oteryn-hsts-stage1.yml
  - .github/workflows/cloudflare-oteryn-public-edge-repair.yml
  - .github/workflows/cloudflare-zone-edge-audit.yml
  - .github/workflows/editorial-media-acceptance.yml
  - .github/workflows/liquid20-synology-control.yml
  - .github/workflows/native-auth-canary-cache-build.yml
  - .github/workflows/native-protocol-contract-audits.yml
  - .github/workflows/native-protocol-contract.yml
  - .github/workflows/oteryn-public-edge-validation.yml
  - .github/workflows/synology-container-hygiene.yml
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/wiki-reconciliation-acceptance.yml
  - .github/workflows/deep-system-validation.yml
  - .github/workflows/portal-exhaustive-audit.yml
  - .github/workflows/portal-exhaustive-acceptance.yml
  - .github/workflows/portal-exhaustive-trigger-coupling.yml
  - .github/workflows/account-security-format-diagnostics.yml
  - .github/workflows/account-security-static-diagnostics.yml
  - scripts/ci/classify_changes.py
  - tests/ci/test_classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tests/ci/test_workflow_trigger_economy.py
  - tests/operations/cloudflare-oteryn-endpoints/check-legacy-hostname.sh
  - tools/validation/workflow_inventory.py
  - tools/validation/test_workflow_inventory.py
  - tools/validation/php_coverage_policy.py
  - tools/validation/test_php_coverage_policy.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/CI_COVERAGE_POLICY.json
  - docs/agents/tasks/active/OTERYN-20260815-ci-workflow-orchestration.md
modules:
  - ci
  - testing
  - agent-governance
dependencies:
  - Issue #1085
blockers:
  - none
cross_repository_tasks:
  - none
```

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: implementation owner
  classified_at: 2026-08-15T08:15:12Z
  risk: high
  triggers:
    - CI routing and required-gate behavior
    - GitHub Actions workflow lifecycle
    - workflow deletion and concurrency semantics
    - domain acceptance trigger admission
    - post-merge coverage instrumentation
  unknown_or_conflict:
    - exact classic branch-protection required-status-check list is unreadable through the connected GitHub integration (403); mergeability and exact-head checks are therefore verified from the live PR/check state without bypass
  rationale: CI changes can alter repository-wide merge evidence; central routing remains fail-closed, event authority is preserved, and retained gate names are not administratively bypassed.
  self_review:
    result: PASS
    exact_head: ac3f3682045eedd40d0fbc6e95e51c3021dae4cf
    acceptance_checked: true
    full_diff_checked: true
    negative_paths_checked: true
    rollback_checked: true
    compatibility_checked: true
    related_prs_checked: true
    findings: []
    evidence:
      - full PR diff and all twelve concurrency workflow patches were inspected individually
      - accidental Cloudflare capability-audit credential drift was found by self-review and restored before PASS
      - current runtime head is synchronized with protected main ad0fec2bb0ce12a236916fa84af5be8f8f382050
      - PR #1086 has zero submitted reviews and zero review threads at this checkpoint
      - product/browser E2E is NOT_APPLICABLE because no product behavior/UI changes; real GitHub Actions executions are the integration E2E for CI routing/orchestration
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T08:46:37Z
head: ac3f3682045eedd40d0fbc6e95e51c3021dae4cf
branch: ci/issue-1085-workflow-orchestration
pr: 1086
status: validating
context_routes:
  - testing
  - ci-repair
  - agent-governance
owned_paths:
  - .github/workflows/**
  - scripts/ci/**
  - tests/ci/**
  - tests/operations/cloudflare-oteryn-endpoints/check-legacy-hostname.sh
  - tools/validation/workflow_inventory.py
  - tools/validation/test_workflow_inventory.py
  - tools/validation/php_coverage_policy.py
  - tools/validation/test_php_coverage_policy.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/CI_COVERAGE_POLICY.json
  - docs/agents/tasks/active/OTERYN-20260815-ci-workflow-orchestration.md
proven:
  - live workflow baseline reached 59 definitions after PR #1083; the candidate retires six obsolete wrappers/diagnostics, yielding a registered budgeted inventory of 53
  - docs-only main commit 860033172c8b4f1ba21d8d79263f04e2f0a49928 previously ran Portal Acceptance complete account lifecycle because push/main lacked path admission
  - generic workflow-file changes previously mapped to ALL_GATES; the candidate routes ordinary workflow edits to core CI, own heavy workflow edits to core CI plus their lane, and central CI/router edits to ALL_GATES
  - workflow lifecycle registry fails closed on unregistered growth, retired reintroduction, missing registered files and budget overflow
  - trigger-economy contract now scans the full workflow inventory, rejects retired trigger coupling, and verifies PR cancellation plus stable per-PR concurrency identity
  - trusted/main/manual operations retain non-PR serialization; only replaceable pull_request validation is cancellable
  - Editorial Media and Wiki acceptance retain exact domain code/migrations/routes/browser specs/portal-manifest dependencies while broad historical exhaustive coupling is removed
  - coverage policy is report_only because historical CI used coverage:none and no verified numerical baseline exists; relevant main pushes use PCOV/Clover and a 14-day evidence artifact
  - focused PublicEdge architecture names login.oteryn.molehill.cloud only in an exact retirement invariant; the legacy-host guard now verifies that exact statement rather than blindly allowlisting the file
  - previous failed generations were investigated from logs and repaired at root cause rather than rerun or weakened
  - on runtime head ac3f3682045eedd40d0fbc6e95e51c3021dae4cf central CI, Agent Governance, Phase 7, DB outage, edge security, game-auth concurrency, Cloudflare validations, Editorial Media and Wiki acceptance were observed PASS; Native Auth build and Portal Acceptance were still pending at the last pre-checkpoint observation
  - active unrelated tasks do not own the exact workflow paths modified here; historical-work closeout was independently completed by PR #1088
  - repository delete_branch_on_merge is true
  - repository rulesets endpoint is empty; classic branch-protection configuration remains unreadable through this integration
  - full-diff self-review is PASS on runtime head ac3f3682045eedd40d0fbc6e95e51c3021dae4cf with zero open findings

derived:
  - a numerical coverage floor must not be invented before stable observed baseline evidence exists
  - final checkpoint-only commit does not change runtime semantics; nevertheless the PR-wide workflow path set may emit a fresh exact-head generation, which remains required before merge
unknown:
  - exact classic branch-protection required-status-check list
conflicts: []
first_failure:
  marker: tests/ci/test_workflow_trigger_economy.py conditional cancellation parser
  evidence: CI run 31873486132 job 94985519513
rejected_hypotheses:
  - delete unique product/security workflows merely to hit an arbitrary workflow count
  - weaken failing tests or broad safety checks to obtain green CI
  - globally cancel trusted/manual/production operation events
  - invent an immediate code-coverage percentage gate without a measured baseline
changed_paths:
  - 37 files in PR #1086, all within declared CI/testing/governance scope
validation:
  - command: repository preflight, overlap/ownership and main synchronization
    result: PASS
    evidence: live GitHub Issue/PR/task/main state inspected; runtime head includes protected main ad0fec2bb0ce12a236916fa84af5be8f8f382050
  - command: historical exact-head repair generations
    result: PASS
    evidence: each prior FAIL was diagnosed from job logs and followed by a materially changed candidate; no identical blind reruns
  - command: full exact-head diff/self-review on ac3f3682045eedd40d0fbc6e95e51c3021dae4cf
    result: PASS
    evidence: 37 changed paths inspected; one accidental secret-reference drift found and repaired before PASS
  - command: review hygiene on PR #1086
    result: PASS
    evidence: zero submitted reviews and zero review threads
  - command: product/browser E2E
    result: NOT_APPLICABLE
    evidence: no product behavior/UI path changed; GitHub Actions is the executable integration surface under test
blockers:
  - none
next_action: observe the final exact-head PR generation after this checkpoint commit; when all applicable checks pass, mark PR #1086 ready and complete the protected squash-merge gate
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 7
  session_id: chat-20260815-ci-1085-01
  session_started_at: 2026-08-15T07:41:00Z
  checkpointed_at: 2026-08-15T08:46:37Z
  last_progress_at: 2026-08-15T08:46:37Z
  phase: validate
  exact_head: ac3f3682045eedd40d0fbc6e95e51c3021dae4cf
  pull_request: 1086
  active_operation: final exact-head validation after checkpoint persistence
  external_run_ids: []
  operation_started_at: 2026-08-15T08:46:37Z
  wait_deadline_at: 2026-08-15T09:31:37Z
  check_generation: draft
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: PR #1086 head remains unchanged and no requested change, ownership conflict or safety blocker appears
  next_action: observe the final exact-head PR generation after this checkpoint commit; when all applicable checks pass, mark PR #1086 ready and complete the protected squash-merge gate
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: PR #1086 is still open; repository delete_branch_on_merge=true will be relied on only after protected merge and verified afterward
source_branch_evidence: pending final merge
```

## Notes

Workflow-count reduction is evidence-driven rather than a numerical target. Unique product/security/operations checks are retained unless their proof is already supplied by a current canonical workflow; historical workflow runs and Git history retain provenance after executable task-specific orchestration is removed.
