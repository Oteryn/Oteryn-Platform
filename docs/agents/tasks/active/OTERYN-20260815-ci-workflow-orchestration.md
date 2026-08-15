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

- [ ] Documentation/governance-only main pushes do not execute Portal Acceptance runtime/account-lifecycle jobs.
- [ ] Editing an ordinary workflow no longer fans out to every heavy runtime gate; central routing changes remain fail-closed.
- [ ] Repository tests fail when domain workflows regress to unbounded push/PR triggering, retain retired-workflow coupling, lose superseded-run cancellation, or use an unstable/non-isolated PR concurrency identity.
- [ ] Proven obsolete diagnostic/task-wrapper workflows are removed without deleting unique product test coverage.
- [ ] Completed one-off deep/exhaustive orchestration no longer runs automatically on ordinary product PRs/main pushes.
- [ ] Domain acceptance workflows no longer inherit broad trigger paths solely from retired exhaustive orchestration.
- [ ] Mixed validation/operation workflows cancel only superseded PR validation while preserving trusted/manual/scheduled operation serialization.
- [ ] PHP application coverage is measured outside the blocking PR fast path with a ratchet-ready policy and durable report artifact.
- [ ] Workflow lifecycle rules make temporary/task-specific workflow retention explicit and testable.
- [ ] Legacy public-host reuse guard permits the focused architecture retirement statement only while that exact fail-closed retirement invariant remains present.
- [ ] Exact-head CI, self-review, diff scope, review hygiene and source-branch closeout satisfy repository gates.

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
  - branch-protection required-check configuration is not readable through the connected GitHub integration; preserve existing check names for retained automatic gates and avoid deleting unique required product checks
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
    - exact classic branch-protection required-status-check list is not readable through the connected integration
  rationale: CI and workflow changes can alter repository-wide merge evidence; routing remains fail-closed at the control plane and unique retained gate names are preserved where live protection configuration cannot be inspected.
  self_review:
    result: PENDING
    exact_head: 7bb0442a3b5b62e933c33e9b9cb180b94cecd1ab
    evidence:
      - whole-diff review is in progress; final exact-head review will be recorded only after all CI findings are repaired
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T08:48:00Z
head: 7bb0442a3b5b62e933c33e9b9cb180b94cecd1ab
branch: ci/issue-1085-workflow-orchestration
pr: 1086
status: implementing
context_routes:
  - testing
  - ci-repair
  - agent-governance
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
proven:
  - task branch has draft PR #1086 and Issue #1085 is the owning remediation Issue
  - candidate 7bb0442a3b5b62e933c33e9b9cb180b94cecd1ab includes protected-main commit ad0fec2bb0ce12a236916fa84af5be8f8f382050 from merged historical closeout PR #1088
  - baseline workflow inventory reached 59 workflows after PR #1083; the candidate removes six proven obsolete workflow definitions
  - portal-acceptance-contract.yml previously had an unfiltered push-to-main trigger and executed complete account lifecycle for docs-only main commit 860033172c8b4f1ba21d8d79263f04e2f0a49928
  - scripts/ci/classify_changes.py previously mapped generic workflow changes to ALL_GATES
  - account-security format/static diagnostic workflows duplicated blocking Pint and PHPStan/Larastan checks already present in central CI
  - portal-exhaustive acceptance/trigger-coupling and deep/portal-exhaustive programme workflows belonged to completed archived validation programmes rather than active delivery ownership
  - Editorial Media and Wiki Reconciliation acceptance triggers carried broad paths and retired portal-exhaustive coupling; focused domain dependencies have now been identified and retained
  - exact-head generation ed4b78b59263afe44089a764654eeeb680bd4360 exposed a test-parser defect for safe conditional PR cancellation; repaired without changing workflow safety semantics
  - exact-head generation 1bab2057ed30384361e8ffa726a996251af3e04a exposed cloudflare-oteryn-edge-audit.yml PR cancellation debt and missing PR identity in this task checkpoint; both were repaired
  - exact-head generation 691a57530e7f13024f6d15b9e9dc0aad2e541d23 aggregated twelve retained workflows whose PR validation was not supersedable; each was classified by event semantics before repair
  - candidate 7bb0442a3b5b62e933c33e9b9cb180b94cecd1ab repairs all twelve with PR-only cancellation and event-safe concurrency identities while retaining non-PR operation serialization
  - active public-domain and native-auth production-verification tasks do not claim the exact workflow paths modified by this task
  - Cloudflare Endpoints run 31874879332 on candidate 7bb0442a3b5b62e933c33e9b9cb180b94cecd1ab failed only the legacy-host reuse guard because current focused architecture names the retired hostname in an explicit retirement statement
  - docs/architecture/PUBLIC_EDGE_ARCHITECTURE.md states exactly that login.oteryn.molehill.cloud is retired and must not be restored as a canonical endpoint without an explicit contract/decision change
  - current GitHub integration returns 403 for classic branch-protection configuration and repository rulesets endpoint returns no repository rulesets
derived:
  - the PublicEdge architecture reference is a legitimate current retirement invariant, not hostname reuse; the guard should permit it only while an exact retirement statement is present
  - Cloudflare trusted-main push and pull_request_target operations must remain non-cancelled while ordinary pull_request validation may safely cancel superseded generations
  - Liquid20 and Synology hygiene must retain their existing non-PR operation mutex while PR validation receives a separate per-PR concurrency group
  - native-auth/manual dispatch and native-protocol main-push runs must remain non-cancelled while PR validation becomes supersedable
  - coverage should run after relevant application changes and after an exact ci.yml change, not after every arbitrary workflow-file edit
  - workflow trigger-economy must validate both cancellation behavior and a stable per-PR concurrency identity
unknown:
  - exact classic branch-protection required-status-check list
conflicts: []
first_failure:
  marker: tests/ci/test_workflow_trigger_economy.py conditional cancellation parser
  evidence: CI run 31873486132 job 94985519513 on ed4b78b59263afe44089a764654eeeb680bd4360
rejected_hypotheses:
  - delete broad product acceptance workflows solely to reach an arbitrary workflow count
  - weaken workflow-economy assertions when they reveal an existing replaceable-PR cancellation defect
  - keep retired exhaustive workflow names in domain triggers as historical provenance
  - set cancel-in-progress true globally on mixed validation/operation workflows
  - blindly allowlist PUBLIC_EDGE_ARCHITECTURE.md without validating its exact retirement statement
changed_paths:
  - CI routing, lifecycle, coverage policy, workflow trigger/concurrency, legacy-host guard and task-record paths declared above
validation:
  - command: repository preflight and live overlap inspection
    result: PASS
    evidence: Issue/open-PR/active-task ownership and workflow evidence inspected through GitHub connector
  - command: exact-head CI generation ed4b78b59263afe44089a764654eeeb680bd4360
    result: FAIL
    evidence: run 31873486132 job 94985519513 identified a new test-parser defect; repaired in a later candidate head
  - command: exact-head CI generation 1bab2057ed30384361e8ffa726a996251af3e04a
    result: FAIL
    evidence: run 31873708280 job 94986114553 identified Cloudflare PR cancellation debt; Agent Governance run 31873708252 identified omitted PR identity; both repaired later
  - command: exact-head CI generation 691a57530e7f13024f6d15b9e9dc0aad2e541d23
    result: FAIL
    evidence: run 31874328618 job 94987667267 aggregated twelve retained workflows without superseded PR cancellation; Agent Governance also identified invalid historical validation-result labels and a separately owned stale task subsequently closed by PR #1088
  - command: exact-head CI generation 7bb0442a3b5b62e933c33e9b9cb180b94cecd1ab
    result: FAIL
    evidence: Cloudflare Endpoints run 31874879332 job 94989011461 identified the focused PublicEdge retirement reference as outside its existing allowlist; all other observed failure evidence is being preserved and repaired rather than suppressed
blockers:
  - none
next_action: persist the strict retirement-reference guard repair, then inspect a fresh exact-head CI generation
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 6
  session_id: chat-20260815-ci-1085-01
  session_started_at: 2026-08-15T07:41:00Z
  checkpointed_at: 2026-08-15T08:48:00Z
  last_progress_at: 2026-08-15T08:48:00Z
  phase: repair
  exact_head: 7bb0442a3b5b62e933c33e9b9cb180b94cecd1ab
  pull_request: 1086
  active_operation: repair legacy-host guard against current focused architecture
  external_run_ids:
    - 31873486132
    - 31873708280
    - 31873708252
    - 31874328618
    - 31874328584
    - 31874879332
  operation_started_at: 2026-08-15T08:48:00Z
  wait_deadline_at: null
  check_generation: draft
  checks_used: 2
  status: active
  safe_to_resume: true
  resume_condition: branch remains owned by Issue #1085 and PR #1086 with no overlapping path owner
  next_action: persist the strict retirement-reference guard repair, then inspect a fresh exact-head CI generation
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: PR #1086 remains draft and unmerged
```

## Notes

Workflow-count reduction is evidence-driven rather than a numerical target. Unique product/security/operations checks are retained unless their proof is already supplied by a current canonical workflow; historical workflow runs and Git history retain provenance after executable task-specific orchestration is removed.
