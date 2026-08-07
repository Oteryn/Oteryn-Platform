---
task_id: OTERYN-20260807-native-protocol-audit-routing
issue: 829
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
status: validating
risk: high
validation_intensity: HEIGHTENED
execution_mode: github_only
branch: repair/issue-829
base_branch: main
pr: 832
production_activation_authorized: false
cross_repository_mutation_authorized: false
owned_paths:
  - .github/workflows/native-protocol-contract-audits.yml
  - scripts/validate_native_protocol_change_boundary.py
  - scripts/test_native_protocol_change_boundary.py
  - docs/agents/tasks/active/OTERYN-20260807-native-protocol-audit-routing.md
modules:
  - ci-native-protocol-audit-routing
coordination_key: ci:native-protocol-audit-routing
blockers: []
cross_repository_tasks: []
---

# OTERYN-20260807 native protocol audit routing

## Goal

Repair Issue #829 so the native-protocol architecture boundary audit does not reject unrelated runtime changes merely because an unrelated file under `docs/contracts/**` or `docs/architecture/**` changed, while true native-protocol producer corrections remain fail-closed.

## Acceptance criteria

- [x] Unrelated contract + unrelated runtime changes do not invoke native-protocol producer ownership enforcement.
- [x] A native-protocol producer correction with runtime changes still requires the canonical active producer task record.
- [x] Native-protocol producer runtime changes outside the existing allowlist still fail closed.
- [x] Focused deterministic regression fixtures cover unrelated-change PASS, missing-task FAIL, escaped-runtime FAIL, native-doc-only PASS, and valid producer PASS.
- [x] Existing architecture, security/downgrade, parser/schema, Canary regression, rollout and rollback audits remain present and unchanged in meaning.
- [ ] Required exact-head CI and Agent Governance pass; workflow-specific validation passes.
- [x] E2E is `NOT_APPLICABLE`: this task changes CI routing/validation code only and no product/runtime user journey.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T18:39:00+02:00
head: f03eaae6744b5e9be203488651a6420686cc0130
branch: repair/issue-829
pr: 832
status: validating
context_routes:
  - agent-governance
  - testing
  - ci-repair
owned_paths:
  - .github/workflows/native-protocol-contract-audits.yml
  - scripts/validate_native_protocol_change_boundary.py
  - scripts/test_native_protocol_change_boundary.py
  - docs/agents/tasks/active/OTERYN-20260807-native-protocol-audit-routing.md
proven:
  - Issue #829 reproduces because generic docs/contracts or docs/architecture triggering caused Audit 1 to classify every runtime path in the PR as a native-protocol producer correction.
  - The repair isolates producer-boundary routing in a deterministic Python classifier and leaves the four other native-protocol audit jobs unchanged in meaning.
  - The classifier requires a specific native-protocol producer signal before applying runtime ownership enforcement.
  - Focused fixtures cover unrelated contract/runtime and unrelated architecture/runtime as NOT_APPLICABLE, native documentation-only PASS, missing producer task FAIL, missing task file FAIL, escaped runtime FAIL, and valid governed runtime PASS.
  - Main branch protection requires classify-changes and test.
derived:
  - Generic contract or architecture maintenance can no longer produce this native-protocol ownership false positive while signaled producer corrections retain the prior fail-closed task and runtime allowlist gates.
unknown: []
conflicts: []
first_failure:
  marker: agent-governance-31198268345
  evidence: initial claim checkpoint omitted canonical required checkpoint fields and used provisional policy metadata inside the checkpoint block; the implementation candidate replaces it with contract-v1 fields and records PR 832.
rejected_hypotheses:
  - Removing the producer ownership check would fix the false positive; rejected because it would weaken a real native-protocol governance boundary.
  - Narrowing the whole workflow trigger to only native files is sufficient; rejected because Audits 2-5 intentionally validate broader contract and architecture changes and must retain their existing trigger surface.
changed_paths:
  - .github/workflows/native-protocol-contract-audits.yml
  - scripts/validate_native_protocol_change_boundary.py
  - scripts/test_native_protocol_change_boundary.py
  - docs/agents/tasks/active/OTERYN-20260807-native-protocol-audit-routing.md
validation:
  - command: Agent Governance / run 31198268345
    result: FAIL
    evidence: initial claim checkpoint was structurally incomplete; repaired in the coherent implementation candidate.
  - command: CI / run 31198268957
    result: PASS
    evidence: task-claim baseline passed required repository CI before workflow implementation.
  - command: focused native-protocol change-boundary fixtures
    result: NOT_RUN
    evidence: wired into Audit 1 on the implementation candidate so GitHub Actions executes them on the exact candidate head.
blockers: []
next_action: validate the coherent implementation candidate on PR 832, inspect the first attributable failure if any, then perform exact-head self-review and merge only after required gates pass
```

## Design

Audit 1 now separates two concerns:

1. canonical native-protocol architecture invariants always run when the workflow is triggered;
2. producer runtime ownership enforcement runs only when the PR changes a specific native-protocol producer signal such as the canonical gameplay contract/proto/migration, native protocol ADR/threat/rollout documents, producer operation/prompt, native fixtures, or the canonical contract validator.

The generic `docs/contracts/**` and `docs/architecture/**` workflow triggers remain intact for Audits 2-5. Workflow/script maintenance itself does not pretend to be a gameplay producer correction.

## Safety

Repository-only CI/governance repair. No production deployment, protected-environment operation, secret access, runtime product behavior, Canary mutation or external-repository mutation is authorized.
